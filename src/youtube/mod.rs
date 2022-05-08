mod extractor;
pub mod model;
mod rank;
mod search;
mod stream;

// use super::model;
// use super::Youtube;
use crate::download;
use crate::proto;
use crate::sink;
use crate::sink::{DownloadedTrack, Method, Sink};
use crate::utils;
use anyhow::Result;
use async_trait::async_trait;
use futures::stream::Stream;
use futures_util::stream::{StreamExt, TryStreamExt};
use reqwest;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use stream::paginate;

impl From<model::YoutubeVideo> for proto::djtool::Track {
    fn from(video: model::YoutubeVideo) -> proto::djtool::Track {
        proto::djtool::Track {
            id: Some(proto::djtool::TrackId {
                source: proto::djtool::Service::Youtube as i32,
                id: video.video_id,
                // .map(|id| id.id().to_string())
                // .unwrap_or("unknown".to_string()),
                playlist_id: None, // unknown at this point
            }),
            name: video.title,
            duration_secs: 0, // track.duration.as_secs(),
            artwork: None,
            preview: None,
            artist: "".to_string(),
            info: None,
            // artwork: {
            //     let mut images = track
            //         .album
            //         .images
            //         .into_iter()
            //         .map(proto::djtool::Artwork::from)
            //         .collect::<Vec<proto::djtool::Artwork>>();
            //     images.sort_by(|b, a| (a.width * a.height).cmp(&(b.width * b.height)));
            //     images.first().map(|a| a.to_owned())
            // },
            // preview: track
            //     .preview_url
            //     .map(|url| proto::djtool::TrackPreview { url }),
            // artist: track
            //     .artists
            //     .into_iter()
            //     .map(|a| a.name)
            //     .collect::<Vec<String>>()
            //     .join(", "),
        }
    }
}

// impl Youtube {

//     pub async fn rank_results(&self) -> Result<()> {
//         Ok(())
//     }
// }
#[derive(Debug, Clone)]
pub struct Youtube {
    // debug_dir: PathBuf,
    client: Arc<reqwest::Client>,
}

impl Youtube {
    // pub fn new<P: AsRef<Path> + Send + Sync>(debug_dir: P) -> Result<Self> {
    // pub fn new() -> Result<Self> {
    pub fn new() -> Self {
        Self {
            // debug_dir: debug_dir.as_ref().to_owned(),
            client: Arc::new(reqwest::Client::new()),
        }
    }

    // pub async fn search_page(
    //     &self,
    //     user_id: &UserId,
    //     limit: Option<u32>,
    //     offset: Option<u32>,
    // ) -> Result<Page<SimplifiedPlaylist>> {
    //     let params = HashMap::<&str, Value>::from_iter(
    //         vec![
    //             limit.map(|limit| ("limit", limit.into())),
    //             offset.map(|offset| ("offset", offset.into())),
    //         ]
    //         .into_iter()
    //         .filter_map(|e| e),
    //     );
    //     self.client
    //         .get(api!(format!("users/{}/playlists", user_id.id()))?)
    //         .headers(self.auth_headers().await)
    //         .query(&params)
    //         .send()
    //         .await?
    //         .json::<Page<SimplifiedPlaylist>>()
    //         .await
    //         .map_err(Into::into)
    // }

    pub fn search_stream<'a>(
        // pub fn search_stream(
        &'a self,
        // &self,
        search_query: String,
        // continuation: Option<String>,
        // user_id: &'a UserId,
        // ) -> impl Stream<Item = Result<YoutubeVideo>> + 'a + Send {
    ) -> impl Stream<Item = Result<model::YoutubeVideo>> + 'a + Send {
        paginate(
            move |continuation| self.search_page(search_query.to_owned(), continuation)
            // &user_id, Some(limit), Some(offset)),
            // DEFAULT_PAGINATION_CHUNKS,
        )
    }
}

#[async_trait]
impl Sink for Youtube {
    async fn download(
        &self,
        // track: TrackDescription,
        track: &proto::djtool::Track,
        // output_path: &PathBuf,
        output_path: &(dyn AsRef<Path> + Sync + Send),
        method: Option<Method>,
        progress: Box<dyn Fn(download::DownloadProgress) -> () + Send + 'static>,
        // progress: impl Fn(download::DownloadProgress) -> () + 'static,
    ) -> Result<DownloadedTrack> {
        // search the video first
        // let method = method.unwrap_or(Method::First);
        // let video_id = self.find_best_video(&track, method).await?;

        let video_id = track
            .id
            .as_ref()
            .ok_or(anyhow::anyhow!("no video id"))?
            .id
            .to_owned();
        let video = self.get_video(&video_id).await?;
        let audio_formats = video.formats.audio();
        // if audio_formats.len() < 1 {
        //     panic!("todo: error when no audio formats");
        // }
        // for (i, f) in audio_formats.iter().enumerate() {
        //     println!(
        //         "{}: {:?} {:?} {:?}",
        //         i, f.quality_label, f.mime_type, f.bitrate
        //     );
        // }
        let format = audio_formats
            .first()
            .ok_or(anyhow::anyhow!("no format"))?
            .to_owned()
            .to_owned();
        // println!(
        //     "Video '{:?}' - Quality '{:?}' - Codec '{:?}'",
        //     video.title, format.quality_label, format.mime_type
        // );

        let title = video.title.to_owned().ok_or(anyhow::anyhow!("untitled"))?;
        // let artist = video.author.to_owned();
        // let filename = vec![Some(title), artist]
        // let sanitized_filename = utils::sanitize_filename(format!("{} - {}", title, artist));
        let sanitized_filename = utils::sanitize_filename(&title);
        // println!("sanitized filename: {}", sanitized_filename);

        // let output_path = output_path.to_owned();
        // println!("output path: {}", output_path.display());

        // create the directory if it does not already exist
        // let content_length = self.download(&video, &format, output_path.clone()).await?;
        let stream_url = self.get_stream_url(&video, &format).await?;
        //     println!("stream url: {}", stream_url);
        let mut download = download::Download::new(&stream_url, &output_path).await?;
        download
            // .start(progress) // |progress: download::DownloadProgress| {})
            .start(|progress: download::DownloadProgress| {})
            .await?;

        Ok(DownloadedTrack {
            track: proto::djtool::Track {
                name: track.name.to_owned(),
                id: Some(proto::djtool::TrackId {
                    id: video_id.to_owned(),
                    source: proto::djtool::Service::Youtube as i32,
                    playlist_id: None,
                }),
                artist: track.artist.to_owned(),
                artwork: None,
                preview: None,
                duration_secs: 0, // todo
                info: None,
            },
            output_path: output_path.as_ref().to_owned(),
        })
        // Ok(OutputVideo {
        //     info: video,
        //     thumbnail: None,
        //     audio_file: output_path,
        //     content_length: download.info.content_length,
        //     format,
        // });
    }

    async fn candidates(
        &self,
        track: &proto::djtool::Track,
        progress: Box<dyn Fn(sink::QueryProgress) -> () + Send + 'static>,
        limit: Option<usize>,
    ) -> Result<Vec<proto::djtool::Track>> {
        let query = format!("{} {}", track.name, track.artist);
        // println!("youtube query: {}", query);
        let search_result_stream = self
            .search_stream(query)
            .take(10)
            .filter_map(|video: Result<model::YoutubeVideo>| async move { video.ok() });
        let search_result_stream = match limit {
            Some(limit) => search_result_stream.take(limit).into_inner(),
            None => search_result_stream,
        };
        let search_results = search_result_stream
            // .collect::<Vec<model::YoutubeVideo>>()
            .map(|video: model::YoutubeVideo| video.into())
            .collect::<Vec<proto::djtool::Track>>()
            .await;

        // println!("youtube search results : {:?}", search_results);
        // let first_hit = search_results
        //     .first()
        //     .ok_or(anyhow::anyhow!("no results"))?;
        // let candidates = search_results[0..limit.unwrap_or(10).min(search_results.len() - 1)];

        // println!("youtube first hit: {:?}", first_hit);
        // Ok(first_hit.video_id.to_owned())
        Ok(search_results)
    }

    // pub async fn download(
    //     &self,
    //     video: &Video,
    //     format: &Format,
    //     output_path: PathBuf,
    // ) -> Result<u64> {
    //     let stream_url = self.get_stream_url(video, format).await?;
    //     println!("stream url: {}", stream_url);
    //     let mut download = Download::new(stream_url, output_path).await?;
    //     download.start().await?;
    //     Ok(download.info.content_length)
    // }

    // pub async fn download_audio(&self, id: String, dest: &PathBuf) -> Result<OutputVideo> {
    //     let video = self.get_video(&id).await?;
    //     // if video.formats.len() < 1 {
    //     // todo: raise error here
    //     // panic!("todo: error when no formats");
    //     // }
    //     let audio_formats = video.formats.audio();
    //     for (i, f) in audio_formats.iter().enumerate() {
    //         println!(
    //             "{}: {:?} {:?} {:?}",
    //             i, f.quality_label, f.mime_type, f.bitrate
    //         );

    //         // println!(
    //         //     "{}: {:?} {:?} {:?} {:?}",
    //         //     i, f.quality_label, f.mime_type, f.bitrate, f.url
    //         // );
    //     }
    //     let format = audio_formats.first().unwrap().to_owned().to_owned();
    //     println!(
    //         "Video '{:?}' - Quality '{:?}' - Codec '{:?}'",
    //         video.title, format.quality_label, format.mime_type
    //     );

    //     // let random_filename = utils::random_filename(25);
    //     // println!("random filename: {}", random_filename);

    //     let sanitized_filename = utils::sanitize_filename(video.title.clone().unwrap());
    //     println!("sanitized filename: {}", sanitized_filename);

    //     let output_path = dest.to_owned();
    //     println!("output path: {}", output_path.display());

    //     // create the directory if it does not already exist
    //     let content_length = self.download(&video, &format, output_path.clone()).await?;

    //     Ok(OutputVideo {
    //         info: video,
    //         thumbnail: None,
    //         audio_file: output_path,
    //         content_length,
    //         format,
    //     })
    // }
}
