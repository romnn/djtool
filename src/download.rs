use crate::utils;
use anyhow::Result;
use boa;
use futures_util::{stream, StreamExt};
use http::header::HeaderMap;
use lazy_static::lazy_static;
use rayon::prelude::*;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tempdir::TempDir;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, Mutex};
use url::Url;

#[derive(Debug)]
pub struct Downloader {
    client: Arc<reqwest::Client>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InnertubeClient {
    hl: String,
    gl: String,
    client_name: String,
    client_version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InnertubeContext {
    client: InnertubeClient,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContentPlaybackContext {
    signature_timestamp: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackContext {
    content_playback_context: ContentPlaybackContext,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InnertubeRequest {
    video_id: String,
    context: InnertubeContext,
    playback_context: PlaybackContext,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnail {
    url: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FormatRange {
    start: Option<String>,
    end: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Format {
    itag: Option<i32>,
    url: Option<String>,
    mime_type: Option<String>,
    quality: Option<String>,
    signature_cipher: Option<String>,
    bitrate: Option<i32>,
    fps: Option<i32>,
    width: Option<i32>,
    height: Option<i32>,
    last_modified: Option<String>,
    content_length: Option<String>,
    quality_label: Option<String>,
    projection_type: Option<String>,
    average_bitrate: Option<i32>,
    audio_quality: Option<String>,
    approx_duration_ms: Option<String>,
    audio_sample_rate: Option<String>,
    audio_channels: Option<i16>,

    // InitRange is only available for adaptive formats
    init_range: Option<FormatRange>,

    // IndexRange is only available for adaptive formats
    index_range: Option<FormatRange>,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum Codec {
    // audio
    OPUS,
    MP4,
    // video
    AV01,
    VP9,
    AVC1,
    // other
    OTHER,
}

impl Format {
    pub fn codec(&self) -> Codec {
        let c = self
            .mime_type
            .as_ref()
            .map(|mime| {
                let mime = mime.to_lowercase();
                if mime.contains("mp4") {
                    return Codec::MP4;
                } else if mime.contains("opus") {
                    return Codec::OPUS;
                } else if mime.contains("av01") {
                    return Codec::AV01;
                } else if mime.contains("vp9") {
                    return Codec::VP9;
                } else if mime.contains("avc1") {
                    return Codec::AVC1;
                }
                Codec::OTHER
            })
            .unwrap_or(Codec::OTHER);
        // println!("codec {:?}", c);
        c
    }
}

#[derive(Debug, Clone)]
pub struct FormatList {
    formats: Vec<Format>,
}

impl From<Vec<Format>> for FormatList {
    fn from(mut formats: Vec<Format>) -> FormatList {
        formats.sort_by(FormatList::cmp_format);
        FormatList { formats }
    }
}

impl FormatList {
    #[allow(dead_code)]
    fn sort(&mut self) {
        self.formats.sort_by(Self::cmp_format);
    }

    fn with_mime_type(&self, substr: &str) -> Vec<&Format> {
        self.formats
            .iter()
            .filter(|f| {
                f.mime_type
                    .as_ref()
                    .map(|m| m.to_lowercase().contains(substr))
                    .unwrap_or(false)
            })
            .collect()
    }

    fn audio(&self) -> Vec<&Format> {
        self.with_mime_type("audio")
    }

    #[allow(dead_code)]
    fn video(&self) -> Vec<&Format> {
        self.with_mime_type("video")
    }

    fn cmp_format(a: &Format, b: &Format) -> Ordering {
        // sort by width
        if a.width == b.width {
            // Format 137 downloads slowly, give it less priority
            // see https://github.com/kkdai/youtube/pull/171
            if a.itag == Some(137) {
                return Ordering::Less;
            }
            if b.itag == Some(137) {
                return Ordering::Greater;
            }
            // sort by fps
            if a.fps == b.fps {
                let a_audio_channels = a.audio_channels.unwrap_or(0);
                let b_audio_channels = b.audio_channels.unwrap_or(0);
                if a.fps.is_none()
            // if a.fps.unwrap_or(0) == 0
                && a_audio_channels > 0
                && b_audio_channels > 0
                {
                    // audio
                    // sort by codec
                    if a.codec() == b.codec() {
                        // sort by audio channel
                        if a_audio_channels == b_audio_channels {
                            // sort by audio bitrate
                            if a.bitrate == b.bitrate {
                                // sort by audio sample rate
                                return b.audio_sample_rate.cmp(&a.audio_sample_rate);
                            }
                            return b.bitrate.cmp(&a.bitrate);
                        }
                        return b_audio_channels.cmp(&a_audio_channels);
                    }
                    let mut rank: HashMap<Codec, u16> = HashMap::new();
                    rank.insert(Codec::MP4, 2);
                    rank.insert(Codec::OPUS, 1);
                    return rank
                        .get(&b.codec())
                        .unwrap_or(&0)
                        .cmp(&rank.get(&a.codec()).unwrap_or(&0));
                }
                // video
                // sort by codec
                let mut rank: HashMap<Codec, u16> = HashMap::new();
                rank.insert(Codec::AV01, 3);
                rank.insert(Codec::VP9, 2);
                rank.insert(Codec::AVC1, 1);

                if a.codec() == b.codec() {
                    // sort by audio bitrate
                    return b.bitrate.cmp(&a.bitrate);
                }
                return rank
                    .get(&b.codec())
                    .unwrap_or(&0)
                    .cmp(&rank.get(&a.codec()).unwrap_or(&0));
            }
            return b.fps.unwrap_or(0).cmp(&a.fps.unwrap_or(0));
        }
        return b.width.unwrap_or(0).cmp(&a.width.unwrap_or(0));
    }
}

/*
    // Sort by Width
    if formats[i].Width == formats[j].Width {
        // Format 137 downloads slowly, give it less priority
        // see https://github.com/kkdai/youtube/pull/171

        // Sort by FPS
        if formats[i].FPS == formats[j].FPS {
            if formats[i].FPS == 0 && formats[i].AudioChannels > 0 && formats[j].AudioChannels > 0 {
                // Audio
                // Sort by codec
                codec := map[int]int{}
                for _, index := range []int{i, j} {
                    if strings.Contains(formats[index].MimeType, "mp4") {
                        codec[index] = 1
                    } else if strings.Contains(formats[index].MimeType, "opus") {
                        codec[index] = 2
                    }
                }
                if codec[i] == codec[j] {
                    // Sort by Audio Channel
                    if formats[i].AudioChannels == formats[j].AudioChannels {
                        // Sort by Audio Bitrate
                        if formats[i].Bitrate == formats[j].Bitrate {
                            // Sort by Audio Sample Rate
                            return formats[i].AudioSampleRate > formats[j].AudioSampleRate
                        }
                        return formats[i].Bitrate > formats[j].Bitrate
                    }
                    return formats[i].AudioChannels > formats[j].AudioChannels
                }
                return codec[i] < codec[j]
            }
            // Video
            // Sort by codec
            codec := map[int]int{}
            for _, index := range []int{i, j} {
                if strings.Contains(formats[index].MimeType, "av01") {
                    codec[index] = 1
                } else if strings.Contains(formats[index].MimeType, "vp9") {
                    codec[index] = 2
                } else if strings.Contains(formats[index].MimeType, "avc1") {
                    codec[index] = 3
                }
            }
            if codec[i] == codec[j] {
                // Sort by Audio Bitrate
                return formats[i].Bitrate > formats[j].Bitrate
            }
            return codec[i] < codec[j]
        }
        return formats[i].FPS > formats[j].FPS
    }
    return formats[i].Width > formats[j].Width
}
*/

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnails {
    thumbnails: Vec<Thumbnail>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MiniplayerRenderer {
    playback_mode: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Miniplayer {
    miniplayer_renderer: MiniplayerRenderer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayabilityStatus {
    status: Option<String>,
    reason: Option<String>,
    playable_in_embed: Option<bool>,
    miniplayer: Miniplayer,
    context_params: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamingData {
    expires_in_seconds: Option<String>,
    formats: Option<Vec<Format>>,
    adaptive_formats: Option<Vec<Format>>,
    dash_manifest_url: Option<String>,
    hls_manifest_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VideoDetails {
    video_id: Option<String>,
    title: Option<String>,
    length_seconds: Option<String>,
    keywords: Option<Vec<String>>,
    channel_id: Option<String>,
    is_owner_viewing: Option<bool>,
    short_description: Option<String>,
    is_crawlable: Option<bool>,
    thumbnail: Option<Thumbnails>,
    average_rating: Option<f32>,
    allow_ratings: Option<bool>,
    view_count: Option<String>,
    author: Option<String>,
    is_private: Option<bool>,
    is_unplugged_corpus: Option<bool>,
    is_live_content: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SimpleText {
    simple_text: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerMicroformatRenderer {
    thumbnail: Option<Thumbnails>,
    title: Option<SimpleText>,
    description: Option<SimpleText>,
    length_seconds: Option<String>,
    owner_profile_url: Option<String>,
    external_channel_id: Option<String>,
    is_family_safe: Option<bool>,
    available_countries: Option<Vec<String>>,
    is_unlisted: Option<bool>,
    has_ypc_metadata: Option<bool>,
    view_count: Option<String>,
    category: Option<String>,
    publish_date: Option<String>,
    owner_channel_name: Option<String>,
    upload_date: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Microformat {
    player_microformat_renderer: Option<PlayerMicroformatRenderer>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerResponseData {
    playability_status: Option<PlayabilityStatus>,
    streaming_data: Option<StreamingData>,
    video_details: Option<VideoDetails>,
    microformat: Option<Microformat>,
}

#[derive(Debug, Clone)]
pub struct Video {
    id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    author: Option<String>,
    hls_manifest_url: Option<String>,
    dash_manifest_url: Option<String>,
    thumbnails: Vec<Thumbnail>,
    duration_seconds: Option<i32>,
    formats: FormatList,
}

impl Video {
    pub fn from_player_response(data: PlayerResponseData) -> Self {
        let duration_seconds = data
            .microformat
            .and_then(|mf| mf.player_microformat_renderer)
            .and_then(|mfr| mfr.length_seconds)
            .and_then(|sec| sec.parse::<i32>().ok());
        // let publish_date =
        let mut formats: Vec<Format> = data
            .streaming_data
            .as_ref()
            .and_then(|sd| sd.formats.as_ref())
            .unwrap_or(&Vec::new())
            .to_vec();
        formats.extend(
            data.streaming_data
                .as_ref()
                .and_then(|sd| sd.adaptive_formats.as_ref())
                .unwrap_or(&Vec::new())
                .to_vec(),
        );
        let formats: FormatList = formats.into();
        Self {
            id: data
                .video_details
                .as_ref()
                .and_then(|vd| vd.video_id.clone()),
            title: data.video_details.as_ref().and_then(|vd| vd.title.clone()),
            description: data
                .video_details
                .as_ref()
                .and_then(|vd| vd.short_description.clone()),
            author: data.video_details.as_ref().and_then(|vd| vd.author.clone()),
            hls_manifest_url: data
                .streaming_data
                .as_ref()
                .and_then(|vd| vd.hls_manifest_url.clone()),
            dash_manifest_url: data
                .streaming_data
                .as_ref()
                .and_then(|vd| vd.dash_manifest_url.clone()),
            thumbnails: data
                .video_details
                .and_then(|vd| vd.thumbnail)
                .map(|t| t.thumbnails)
                .unwrap_or(Vec::new()),
            duration_seconds,
            formats,
        }
    }
    // if seconds, _ := strconv.Atoi(prData.Microformat.PlayerMicroformatRenderer.LengthSeconds); seconds > 0 {
    // 	v.Duration = time.Duration(seconds) * time.Second
    // }

    // if str := prData.Microformat.PlayerMicroformatRenderer.PublishDate; str != "" {
    // 	v.PublishDate, _ = time.Parse(dateFormat, str)
    // }

    // // Assign Streams
    // v.Formats = append(prData.StreamingData.Formats, prData.StreamingData.AdaptiveFormats...)
    // if len(v.Formats) == 0 {
    // 	return errors.New("no formats found in the server's answer")
    // }

    // // Sort formats by bitrate
    // sort.SliceStable(v.Formats, v.SortBitrateDesc)

    // v.HLSManifestURL = prData.StreamingData.HlsManifestURL
    // v.DASHManifestURL = prData.StreamingData.DashManifestURL

    // fmt.Println(v.Formats)
    // for _, format := range v.Formats {
    // 	fmt.Printf("format %s with quality %s (bitrate %d)\n", format.MimeType, format.Quality, format.Bitrate)
    // }
    // // panic("test")
    // return nil
    // }
}

struct PreflightDownloadInfo {
    content_length: u64,
    #[allow(dead_code)]
    content_disposition_name: Option<String>,
    #[allow(dead_code)]
    rangeable: bool,
}

struct Chunk {
    client: Arc<reqwest::Client>,
    // progress: mpsc::Sender<Result<usize>>,
    headers: HeaderMap,
    url: String,
    path: PathBuf,
    range_start: u64,
    range_end: u64,
    downloaded: u64,
}

impl Chunk {
    async fn download(&mut self, progress: &mpsc::Sender<Result<usize>>) -> Result<()> {
        let res = self
            .client
            .get(self.url.clone())
            .headers(self.headers.clone())
            .header(
                "Range",
                format!("bytes={}-{}", self.range_start, self.range_end),
            )
            .send()
            .await?;

        let mut data_stream = res.bytes_stream();
        let mut chunk_file = tokio::fs::File::create(self.path.clone()).await?;
        // if let Err(err) = chunk_file {
        //     let _ = progress.send(Err(err.into())).await;
        //     break;
        // }
        // let chunk_file = chunk_file.unwrap();
        while let Some(byte_chunk) = data_stream.next().await {
            // let chunk = item.or(Err(format!("Error while downloading file")))?;
            let byte_chunk = byte_chunk?;
            // output_file.write(&chunk).unwrap();
            // .or(Err(format!("Error while writing to file")))?;
            // let new = min(downloaded + (chunk.len() as u64), total_size);
            // let new_percent = 100.0 * (downloaded as f32 / total_size as f32);
            // if new_percent - old_percent > 10.0 {
            //     old_percent = new_percent;
            //     println!("downloaded: {}%", new_percent);
            // }
            // downloaded = new;
            // pb.set_position(new);
            // output_file.start_seek(chunk.range_start).await?;
            chunk_file.write(&byte_chunk).await?;
            let _ = progress.send(Ok(byte_chunk.len())).await;
            self.downloaded += byte_chunk.len() as u64;
        }
        // chunk_file.close()?;
        Ok(())
    }
}
struct Download {
    client: Arc<reqwest::Client>,
    temp_dir: TempDir,
    concurrency: usize,
    url: String,
    headers: HeaderMap,
    output_path: PathBuf,
    chunk_size: u64,
    info: PreflightDownloadInfo,
    #[allow(dead_code)]
    downloaded: u64,
    chunks: Arc<Mutex<Vec<Chunk>>>,
    started_at: Instant,
}

impl Download {
    async fn new(url: String, output_path: PathBuf) -> Result<Self> {
        let client = Arc::new(reqwest::Client::new());
        let info = Self::preflight(&client, &url).await?;
        // println!("preflight check completed");
        let concurrency = Self::default_concurrency();
        let chunk_size = Self::default_chunk_size(&info, concurrency, None, None);
        // let sanitized_name = utils::sanitize_filename(video.title.clone().unwrap());
        //
        // create channel for download updates
        // let (tx, mut rx) = mpsc::channel::<Result<usize>>(100);

        let mut download = Self {
            client,
            temp_dir: TempDir::new("djtool")?,
            concurrency,
            url,
            headers: HeaderMap::new(),
            output_path,
            chunk_size,
            info,
            downloaded: 0,
            chunks: Arc::new(Mutex::new(Vec::<Chunk>::new())),
            started_at: Instant::now(),
        };
        download.compute_chunks();
        Ok(download)
    }

    #[allow(dead_code)]
    fn set_chunk_size(&mut self, chunk_size: u64) {
        self.chunk_size = chunk_size;
        // we do not worry about too much concurrency for too little chunks, as this wont create
        // additional overhead
        self.compute_chunks();
    }

    #[allow(dead_code)]
    fn set_concurrency(&mut self, concurrency: usize, min: Option<u64>, max: Option<u64>) {
        self.chunk_size = Self::default_chunk_size(&self.info, concurrency, min, max);
        self.compute_chunks();
    }

    fn default_concurrency() -> usize {
        let mut c = num_cpus::get() * 3;
        c = c.min(20);
        if c <= 2 {
            c = 4;
        }
        c
    }

    fn default_chunk_size(
        info: &PreflightDownloadInfo,
        concurrency: usize,
        min: Option<u64>,
        max: Option<u64>,
    ) -> u64 {
        // let cs: f32 = NumCast::from(info.content_length).unwrap();
        // let cs: f32 = cs / NumCast::from(concurrency).unwrap();
        // let mut cs: u64 = NumCast::from(cs).unwrap();
        let mut cs = (info.content_length as f32 / concurrency as f32) as u64;

        // if chunk size >= 102400000 bytes set default to (chunk size / 2)
        if cs >= 102400000 {
            cs = cs / 2;
        }

        // set default min chunk size to 2M, or file size / 2
        let mut min = min.unwrap_or(2097152u64);
        if min >= info.content_length {
            min = info.content_length / 2;
        }

        // if chunk size < min size set chunk size to min
        cs = cs.max(min);

        // change chunk size if max chunk size are set and chunk size > max size
        if let Some(max) = max {
            cs = cs.min(max);
        }

        // when chunk size > total file size, divide chunk / 2
        if cs >= info.content_length {
            cs = info.content_length / 2;
        }
        cs
    }

    async fn preflight(client: &reqwest::Client, url: &String) -> Result<PreflightDownloadInfo> {
        let res = client.get(url).header("Range", "bytes=0-0").send().await?;
        if res.status().as_u16() >= 300 {
            // todo: return error
            // Response status code is not ok: %d res.status.as_str()
        }
        let headers = res.headers();
        let mut rangeable = false;
        // let mut content_length: Option<u64> = None;
        let mut content_length = res.content_length().unwrap();
        let content_disposition_name = headers
            .get("content-disposition")
            .and_then(|val| val.to_str().map(|s| s.to_string()).ok());
        if let Some(content_range) = headers
            .get("content-range")
            .and_then(|val| val.to_str().ok())
        {
            // check that content-range header is valid
            if !content_range.is_empty() && content_length == 1 {
                let content_range_parts: Vec<&str> = content_range.split("/").collect();
                if content_range_parts.len() == 2 {
                    content_length = content_range_parts[1].parse::<u64>()?;
                    rangeable = true
                }
            }
        }
        Ok(PreflightDownloadInfo {
            content_length,
            // range_length,
            content_disposition_name,
            rangeable,
        })
    }

    fn compute_chunks(&mut self) {
        let num_chunks =
            0..(self.info.content_length as f32 / self.chunk_size as f32).ceil() as usize;
        // let (tx, _) = &self.progress;
        self.chunks = Arc::new(Mutex::new(
            num_chunks
                .into_iter()
                .map(|chunk| {
                    let range_start = chunk as u64 * self.chunk_size;
                    let range_end =
                        ((chunk as u64 + 1) * self.chunk_size - 1).min(self.info.content_length);
                    let chunk_name = format!(
                        "{}.{}-{}.chunk",
                        self.output_path
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .unwrap_or(""),
                        range_start,
                        range_end,
                    );
                    let path = self.temp_dir.path().join(chunk_name);

                    Chunk {
                        client: self.client.clone(),
                        // progress: tx.clone(),
                        headers: self.headers.clone(),
                        url: self.url.clone(),
                        path,
                        range_start,
                        range_end,
                        downloaded: 0,
                    }
                })
                .collect(),
        ));
    }

    async fn start(&mut self) -> Result<()> {
        self.started_at = Instant::now();
        let chunks_clone = self.chunks.clone();

        let (tx, mut rx) = mpsc::channel::<Result<usize>>(100);
        // println!("starting download with {} chunks", chunks.len());

        // tokio::spawn(async move {
        // let chunks: Vec<_> = stream::iter(&self.chunks)
        //     .map(|chunk| {
        //         let sender = tx.clone();
        //         async move {
        //             println!("chunk {}-{}", chunk.range_start, chunk.range_end);
        //             let _ = sender.send(0).await;
        //             0
        //         }
        //     })
        //     .collect()
        //     .await;

        let concurrency = self.concurrency.clone();
        let client = self.client.clone();
        let output_path = self.output_path.clone();
        let headers = self.headers.clone();
        let download = tokio::spawn(async move {
            let mut chunks = chunks_clone.lock().await;
            stream::iter(chunks.iter_mut())
                .for_each_concurrent(concurrency, move |chunk| {
                    // let progress = tx.clone();
                    // let client = client.clone();
                    // let headers = headers.clone();
                    let progress = tx.clone();
                    println!("{}", chunk.path.display());

                    async move {
                        println!("chunk {}-{}", chunk.range_start, chunk.range_end);
                        if let Err(err) = chunk.download(&progress).await {
                            let _ = progress.send(Err(err.into())).await;
                        };
                        // println!("chunk is done");
                    }
                })
                .await;
        });

        let mut downloaded = 0;
        // let (_, rx) = &mut self.progress;
        while let Some(chunk_downloaded) = rx.recv().await {
            match chunk_downloaded {
                Ok(chunk_downloaded) => {
                    downloaded += chunk_downloaded;
                    // println!("downloaded: {}", downloaded);
                }
                Err(err) => eprintln!("chunk failed: {}", err),
            }
        }
        download.await?;
        println!("download completed: {}", downloaded);

        // concat = tokio::spawn(async move {

        // pre-allocate the output file
        let mut output_file = tokio::fs::File::create(self.output_path.clone()).await?;
        // output_file.set_len(self.info.content_length).await?;

        // recombine the chunk files into final file sequentially
        let chunks = self.chunks.lock().await;
        // let chunk_stream = stream::iter(chunks.iter())
        // chunks.iter()
        // .for_each(|chunk| async move {
        for chunk in chunks.iter() {
            let mut chunk_file = tokio::fs::File::open(chunk.path.clone()).await?;
            tokio::io::copy(&mut chunk_file, &mut output_file).await?;
        }
        println!("concatenated: {}", self.output_path.display());

        // self.chunks: Vec<_> = num_chunks
        //     .into_iter()
        //     .map(|chunk| {
        //         // let client = self.client.clone();
        //         // let url = stream_url.clone();
        //         // async move {
        //         //     // req.Header.Set("Range", fmt.Sprintf("bytes=%v-%v", pos, pos+chunkSize-1))
        //         //     client.get(url).send().await
        //         // }
        //     })
        //     .into_iter()
        //     .map(tokio::spawn)
        //     .collect();

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct OutputVideo {
    pub info: Video,
    // id: Option<String>,
    // title: Option<String>,
    // description: Option<String>,
    // author: Option<String>,
    // duration_seconds: Option<i32>,
    pub thumbnail: Option<PathBuf>,
    pub audio_file: PathBuf,
    pub content_length: u64,
    pub format: Format,
}

impl Downloader {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Arc::new(reqwest::Client::new()),
        })
    }

    async fn fetch_player_config(&self, id: &String) -> Result<String> {
        let embed_url = format!("https://youtube.com/embed/{}?hl=en", id);
        let embed_body = self.client.get(embed_url).send().await?.text().await?;

        // example: /s/player/f676c671/player_ias.vflset/en_US/base.js
        lazy_static! {
            static ref BASEJS_PATTERN: Regex =
                Regex::new(r"(/s/player/\w+/player_ias.vflset/\w+/base.js)").unwrap();
        }
        let escaped_basejs_url: Vec<&str> = BASEJS_PATTERN
            .find_iter(&embed_body)
            .map(|m| m.as_str())
            .collect();
        // todo: error handling
        let escaped_basejs_url = escaped_basejs_url.first().unwrap();

        // if escapedBasejsURL == "" {
        // println!("playerConfig: {}", embedBody);
        // rrors.New("unable to find basejs URL in playerConfig")
        // TODO: return error here
        // }
        let basejs_url = format!("https://youtube.com{}", escaped_basejs_url);
        println!("basejs url: {}", basejs_url);
        self.client
            .get(basejs_url)
            .send()
            .await?
            .text()
            .await
            .map_err(|err| err.into())
    }

    async fn get_signature_timestamp(&self, id: &String) -> Result<String> {
        let basejs_body = self.fetch_player_config(id).await?;

        lazy_static! {
            static ref SIGNATURE_PATTERN: Regex =
                Regex::new(r"(?m)(?:^|,)(?:signatureTimestamp:)(\d+)").unwrap();
        }
        let result: Vec<&str> = SIGNATURE_PATTERN
            .captures_iter(&basejs_body)
            .map(|m| m.get(1).map(|c| c.as_str()))
            .filter_map(|m| m)
            .collect();
        // todo: error handling
        // ErrSignatureTimestampNotFound
        let result = result.first().unwrap().to_string();
        println!("signature timestamp: {:?}", result);
        Ok(result)
    }

    async fn video_data_by_innertube(&self, id: &String) -> Result<String> {
        let signature_ts = self.get_signature_timestamp(id).await?;
        let data = InnertubeRequest {
            video_id: id.to_string(),
            context: InnertubeContext {
                client: InnertubeClient {
                    hl: "en".to_string(),
                    gl: "US".to_string(),
                    // client_name: "WEB".to_string(),
                    client_name: "ANDROID".to_string(),
                    // client_version: "2.20210617.01.00".to_string(),
                    // client_version: "2.20210622.10.00".to_string(),
                    client_version: "16.20".to_string(),
                },
            },
            playback_context: PlaybackContext {
                content_playback_context: ContentPlaybackContext {
                    signature_timestamp: signature_ts,
                },
            },
        };

        let player_key = "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8";
        // .data(serde_json::to_string(&data)?)
        let player_url = format!(
            "https://www.youtube.com/youtubei/v1/player?key={}",
            player_key
        );
        println!("player_url: {}", player_url);
        let response = self
            .client
            .post(player_url)
            .json(&data)
            .send()
            .await?
            .text()
            .await?;
        Ok(response)
    }

    // impl Video {
    //     async fn load(&self) {
    //         let body = video_data_by_innertube(&self.id).await;
    //     }
    // }

    // func (v *Video) parseVideoInfo(body []byte) error {
    //     var prData playerResponseData
    //     if err := json.Unmarshal(body, &prData); err != nil {
    //         return fmt.Errorf("unable to parse player response JSON: %w", err)
    //     }

    //     if err := v.isVideoFromInfoDownloadable(prData); err != nil {
    //         return err
    //     }

    //     return v.extractDataFromPlayerResponse(prData)
    // }

    // fn parse_video_info(&self, body: String) -> Result<String> {
    //     // println!("body: {:?}", body);
    //     // let info: serde_json::Value = serde_json::from_str(&body)?;
    //     // println!("info: {}", serde_json::to_string_pretty(&info).unwrap());
    //     let info: PlayerResponseData = serde_json::from_str(&body)?;
    //     println!("info: {:?}", info);
    //     Ok("".to_string())
    // }

    async fn get_video(&self, id: &String) -> Result<Video> {
        let body = self.video_data_by_innertube(id).await?;
        // let video_info = self.parse_video_info(body)?;
        let video_info: PlayerResponseData = serde_json::from_str(&body)?;
        println!("info: {:?}", video_info);
        if video_info
            .playability_status
            .as_ref()
            .and_then(|ps| ps.status.clone())
            == Some("LOGIN_REQUIRED".to_string())
        {
            if video_info
                .playability_status
                .as_ref()
                .and_then(|ps| ps.reason.clone())
                == Some("This video is private.".to_string())
            {
                // todo: return and error here
            }
            // todo: return login required error
        }

        if !video_info
            .playability_status
            .as_ref()
            .and_then(|ps| ps.playable_in_embed)
            .unwrap_or(false)
        {
            // todo: return error here
        }

        if video_info
            .playability_status
            .as_ref()
            .and_then(|ps| ps.status.clone())
            != Some("OK".to_string())
        {
            // todo: return error here
        }

        Ok(Video::from_player_response(video_info))
    }

    async fn decipher_url(&self, video_id: String, cipher: String) -> Result<String> {
        println!("cipher: {}", cipher);
        // let queryParams = url.ParseQuery(cipher)
        let parsed_url = Url::parse(&format!("https://youtube.com?{}", cipher)).unwrap();
        let hash_query: HashMap<_, _> = parsed_url.query_pairs().into_owned().collect();
        println!("cipher: {:?}", hash_query);

        lazy_static! {
            static ref SIG_JS_PATTERNS: Vec<Regex> = vec![
                Regex::new(
                    r#"\b[cs]\s*&&\s*[adf]\.set\([^,]+\s*,\s*encodeURIComponent\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\("#
                ).unwrap(),
                Regex::new(
                    r#"\b[a-zA-Z0-9]+\s*&&\s*[a-zA-Z0-9]+\.set\([^,]+\s*,\s*encodeURIComponent\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\("#
                ).unwrap(),
                Regex::new(r#"\bm=(?P<sig>[a-zA-Z0-9$]{2,})\(decodeURIComponent\(h\.s\)\)"#).unwrap(),
                Regex::new(r#"\bc&&\(c=(?P<sig>[a-zA-Z0-9$]{2,})\(decodeURIComponent\(c\)\)"#).unwrap(),
                Regex::new(
                    r#"(?:\b|[^a-zA-Z0-9$])(?P<sig>[a-zA-Z0-9$]{2,})\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\);[a-zA-Z0-9$]{2}\.[a-zA-Z0-9$]{2}\(a,\d+\)"#
                ).unwrap(),
                Regex::new(
                    r#"(?:\b|[^a-zA-Z0-9$])(?P<sig>[a-zA-Z0-9$]{2,})\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\)"#
                ).unwrap(),
                Regex::new(
                    r#"(?P<sig>[a-zA-Z0-9$]+)\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\)"#
                ).unwrap(),
            ];
        }

        let player_config = self.fetch_player_config(&video_id).await?;
        let matches: Vec<String> = SIG_JS_PATTERNS
            .par_iter()
            .map(|pattern| {
                let test = pattern
                    // .find_iter(&player_config)
                    .captures_iter(&player_config)
                    .map(|m| m.name("sig").map(|g| g.as_str().to_string()))
                    .filter_map(|m| m)
                    .collect::<Vec<String>>();
                test.first().map(|m| m.to_owned())
            })
            .filter_map(|m| m)
            .collect();

        let contents = std::fs::read_to_string("/Users/roman/Desktop/basejsBodyExample.js")
            .expect("Something went wrong reading the file");
        let js_code = boa::parse(&contents, false).unwrap();
        // let js_code = boa::parse(&player_config, false).unwrap();
        // let matches = matches.iter().collect();
        // let matches: Vec<_> = sig_js_patterns.matches(&player_config).into_iter().collect();
        println!("matches:  {:?}", matches);

        Ok("".to_string())
    }

    async fn get_stream_url(&self, video: &Video, format: &Format) -> Result<String> {
        if let Some(url) = &format.url {
            return Ok(url.to_string());
        }
        match &format.signature_cipher {
            Some(cipher) => {
                self.decipher_url(video.id.clone().unwrap(), cipher.clone())
                    .await
            }
            None => panic!("no cipher"),
        }
        // Ok("".to_string())
    }

    // pub fn download_block(
    //     &self,
    //     stream_url: String,
    //     video: &Video,
    //     format: &Format,
    //     mut output_file: std::fs::File,
    // ) -> Result<()> {
    //     let mut res = reqwest::blocking::get(stream_url.clone())?;
    //     println!("start");
    //     let result = res.bytes()?;
    //     println!("done: {}", result.len());
    //     Ok(())
    // }

    pub async fn download(
        &self,
        video: &Video,
        format: &Format,
        output_path: PathBuf,
    ) -> Result<u64> {
        let stream_url = self.get_stream_url(video, format).await?;
        println!("stream url: {}", stream_url);
        // let res = task::spawn_blocking(move || {
        //     let mut res = reqwest::blocking::get(stream_url.clone()).unwrap();
        //     println!("start");
        //     let result = res.bytes().unwrap();
        //     println!("done: {}", result.len());
        // })
        // .await?;
        // return Ok(());
        //
        // create the output file and pre allocate its size
        //
        let mut download = Download::new(stream_url, output_path).await?;
        download.start().await?;
        // client: self.client,
        // concurrency: 8,
        // url: stream_url,
        // output_file: output_file
        // chunk_size: 10_000,
        // // content_length: u64,
        // downloaded: u64,
        // chunks: Vec::<Chunk>,
        // started_at: Instant,
        // };
        // .and_then(|mut file| {
        //     file.poll_set_len(10)
        // })
        // .map(|res| {
        //     // handle returned result ..
        // })?.await
        // .map_err(|err| eprintln!("IO error: {:?}", err));

        // tokio::run(task);
        // return Ok(());

        // pos := int64(0); pos < format.ContentLength;
        // let mut res = reqwest::blocking::get(stream_url.clone())?;
        // let res = self.client.get(stream_url.clone()).send().await?;

        // let chunk_size: u64 = 10_000_000;
        // let total_size = res
        //     .content_length()
        //     .or(format
        //         .content_length
        //         .as_ref()
        //         .and_then(|length| length.parse::<u64>().ok()))
        //     .unwrap_or(0);

        // let outfile = tokio::fs::File::create(outfile).await?;
        // let mut outfile = tokio::io::BufWriter::new(outfile);
        //
        // let mut downloaded: u64 = 0;
        // let mut old_percent = 0.0;

        // println!("start");
        // let result = res.bytes().await?;
        // let result = res.bytes()?;
        // println!("done: {}", result.len());

        // return Ok(());

        // Do an asynchronous, buffered copy of the download to the output file
        // while let Some(chunk) = res.chunk().await? {
        //     // println!("chunk: {}", chunk.len());
        //     // outfile.write(&chunk).await?;
        //     let new = min(downloaded + (chunk.len() as u64), total_size);
        //     let new_percent = 100.0 * (downloaded as f32 / total_size as f32);
        //     if new_percent - old_percent > 10.0 {
        //         old_percent = new_percent;
        //         println!("downloaded: {}%", new_percent);
        //     }
        // }

        // let num_chunks = (0..(total_size as f32 / chunk_size as f32).ceil() as usize);

        // let tasks: Vec<_> = num_chunks
        //     .into_iter()
        //     .map(|chunk| {
        //         let client = self.client.clone();
        //         let url = stream_url.clone();
        //         async move {
        //             // req.Header.Set("Range", fmt.Sprintf("bytes=%v-%v", pos, pos+chunkSize-1))
        //             client.get(url).send().await
        //         }
        //     })
        //     .into_iter()
        //     .map(tokio::spawn)
        //     .collect();

        // let chunks: Vec<_> = join_all(tasks)
        //     .await
        //     .into_iter()
        //     .map(Result::unwrap)
        //     .collect();

        // for chunk in chunks {
        //     println!("chunk completed: {:?}", chunk);
        // }

        // let mut stream = res.bytes_stream();

        // while let Some(item) = stream.next().await {
        //     // let chunk = item.or(Err(format!("Error while downloading file")))?;
        //     let chunk = item.unwrap();
        //     // output_file.write(&chunk).unwrap();
        //     // .or(Err(format!("Error while writing to file")))?;
        //     let new = min(downloaded + (chunk.len() as u64), total_size);
        //     let new_percent = 100.0 * (downloaded as f32 / total_size as f32);
        //     if new_percent - old_percent > 10.0 {
        //         old_percent = new_percent;
        //         println!("downloaded: {}%", new_percent);
        //     }
        //     downloaded = new;
        //     // pb.set_position(new);
        // }

        Ok(download.info.content_length)
    }

    pub async fn download_audio(&self, id: String, dest: &PathBuf) -> Result<OutputVideo> {
        let video = self.get_video(&id).await?;
        // if video.formats.len() < 1 {
        // todo: raise error here
        // panic!("todo: error when no formats");
        // }
        let audio_formats = video.formats.audio();
        for (i, f) in audio_formats.iter().enumerate() {
            println!(
                "{}: {:?} {:?} {:?}",
                i, f.quality_label, f.mime_type, f.bitrate
            );

            // println!(
            //     "{}: {:?} {:?} {:?} {:?}",
            //     i, f.quality_label, f.mime_type, f.bitrate, f.url
            // );
        }
        let format = audio_formats.first().unwrap().to_owned().to_owned();
        println!(
            "Video '{:?}' - Quality '{:?}' - Codec '{:?}'",
            video.title, format.quality_label, format.mime_type
        );

        // let random_filename = utils::random_filename(25);
        // println!("random filename: {}", random_filename);

        let sanitized_filename = utils::sanitize_filename(video.title.clone().unwrap());
        println!("sanitized filename: {}", sanitized_filename);

        // let output_path = self.temp_dir.path().join(sanitized_filename);
        // let output_path = if dest.extension().is_some() {
        //     dest.to_owned()
        // } else {
        //     let _ = tokio::fs::create_dir_all(dest).await;
        //     dest.join(sanitized_filename)
        // };
        let output_path = dest.to_owned();
        println!("output path: {}", output_path.display());

        // create the directory if it does not already exist
        let content_length = self.download(&video, &format, output_path.clone()).await?;

        Ok(OutputVideo {
            info: video,
            // title: video.title
            // description: Option<String>,
            // author: Option<String>,
            // duration_seconds: vieo.duration_seconds,
            thumbnail: None,
            audio_file: output_path,
            content_length,
            format,
        })
    }
}
