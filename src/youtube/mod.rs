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
use futures::future;
use futures::stream::Stream;
use futures::task::Poll;
use futures_util::stream::{StreamExt, TryStreamExt};
use reqwest;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use stream::paginate;

impl From<model::YoutubeVideo> for proto::djtool::Track {
    fn from(video: model::YoutubeVideo) -> proto::djtool::Track {
        proto::djtool::Track {
            id: Some(proto::djtool::TrackId {
                source: proto::djtool::Service::Youtube as i32,
                id: video.video_id,
                playlist_id: None, // unknown at this point
            }),
            name: video.title,
            duration_millis: 0,
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
            move |continuation| self.search_page(search_query.to_owned(), continuation), // &user_id, Some(limit), Some(offset)),
                                                                                         // DEFAULT_PAGINATION_CHUNKS,
        )
    }
}

#[async_trait]
impl Sink for Youtube {
    async fn audio_download_url(&self, track: &proto::djtool::Track) -> Result<(String, String)> {
        // let track_id = track
        //     .id
        //     .as_ref()
        //     .ok_or(anyhow::anyhow!("no video id"))?
        //     .id
        //     .to_owned();
        let track_id = track.id()?;
        let video = self.get_video(&track_id.id).await?;
        let audio_formats = video.formats.audio();
        let format = audio_formats
            .iter()
            .next()
            .ok_or(anyhow::anyhow!("no format"))?;
        let title = video.title.to_owned().ok_or(anyhow::anyhow!("untitled"))?;
        let sanitized_filename = utils::sanitize_filename(&title);
        let stream_url = self.get_stream_url(&video, &format).await?;
        crate::debug!(&video.title);
        crate::debug!(&format);
        crate::debug!(&sanitized_filename);
        crate::debug!(&stream_url);

        Ok((stream_url, sanitized_filename))
    }
    // let mut download = download::Download::new(&stream_url, &output_path).await?;
    // if let Some(progress) = progress {
    // // download.on_progress(|progress: download::DownloadProgress| {});
    // download.on_progress(progress);

    async fn download(
        &self,
        // track: TrackDescription,
        track: &proto::djtool::Track,
        output_path: &(dyn AsRef<Path> + Sync + Send),
        method: Option<Method>,
        progress: Option<Box<dyn Fn(download::DownloadProgress) -> () + Send + Sync + 'static>>,
        // progress: impl Fn(download::DownloadProgress) -> () + 'static,
    ) -> Result<DownloadedTrack> {
        // search the video first
        // let method = method.unwrap_or(Method::First);
        // let video_id = self.find_best_video(&track, method).await?;

        // let video_id = track
        //     .id
        //     .as_ref()
        //     .ok_or(anyhow::anyhow!("no video id"))?
        //     .id
        //     .to_owned();
        // let video = self.get_video(&video_id).await?;
        // let audio_formats = video.formats.audio();
        // // if audio_formats.len() < 1 {
        // //     panic!("todo: error when no audio formats");
        // // }
        // // for (i, f) in audio_formats.iter().enumerate() {
        // //     println!(
        // //         "{}: {:?} {:?} {:?}",
        // //         i, f.quality_label, f.mime_type, f.bitrate
        // //     );
        // // }
        // let format = audio_formats
        //     .first()
        //     .ok_or(anyhow::anyhow!("no format"))?
        //     .to_owned()
        //     .to_owned();
        // // println!(
        // //     "Video '{:?}' - Quality '{:?}' - Codec '{:?}'",
        // //     video.title, format.quality_label, format.mime_type
        // // );

        // let title = video.title.to_owned().ok_or(anyhow::anyhow!("untitled"))?;
        // // let artist = video.author.to_owned();
        // // let filename = vec![Some(title), artist]
        // // let sanitized_filename = utils::sanitize_filename(format!("{} - {}", title, artist));
        // let sanitized_filename = utils::sanitize_filename(&title);
        // // println!("sanitized filename: {}", sanitized_filename);

        // // let output_path = output_path.to_owned();
        // // println!("output path: {}", output_path.display());

        // // create the directory if it does not already exist
        // // let content_length = self.download(&video, &format, output_path.clone()).await?;
        // let stream_url = self.get_stream_url(&video, &format).await?;
        //     println!("stream url: {}", stream_url);
        let track_id = track.id()?;
        let (stream_url, sanitized_filename) = self.audio_download_url(track).await?;
        let mut download = download::Download::new(&stream_url, &output_path).await?;
        if let Some(progress) = progress {
            // download.on_progress(|progress: download::DownloadProgress| {});
            download.on_progress(progress);
        };
        download.start().await?;

        Ok(DownloadedTrack {
            track: proto::djtool::Track {
                name: track.name.to_owned(),
                id: Some(proto::djtool::TrackId {
                    id: track_id.id.to_owned(),
                    source: proto::djtool::Service::Youtube as i32,
                    playlist_id: None,
                }),
                artist: track.artist.to_owned(),
                artwork: None,
                preview: None,
                duration_millis: 0, // todo
                info: None,
            },
            output_path: output_path.as_ref().to_owned(),
        })
    }

    async fn candidates(
        &self,
        track: &proto::djtool::Track,
        progress: Box<dyn Fn(sink::QueryProgress) -> () + Send + 'static>,
        limit: Option<usize>,
    ) -> Vec<proto::djtool::Track> {
        let stream = self.candidates_stream(track, progress, limit);
        stream.collect::<Vec<proto::djtool::Track>>().await
    }

    fn candidates_stream<'b, 'a>(
        &'a self,
        track: &'b proto::djtool::Track,
        progress: Box<dyn Fn(sink::QueryProgress) -> () + Send + 'static>,
        limit: Option<usize>,
        // ) -> impl Stream<Item = Result<proto::djtool::Track>> + 'a + Send {
        // ) -> impl Stream<Item = Result<proto::djtool::Track>> + Send + Unpin {
    ) -> Pin<Box<dyn Stream<Item = (proto::djtool::Track)> + Send + 'a>> {
        // ) -> Pin<Box<dyn Stream<Item = (proto::djtool::Track)> + Send + 'a>> {
        // ) -> impl Stream<Item = Result<proto::djtool::Track>> + 'a + Send {
        // ) -> Result<Vec<proto::djtool::Track>> {
        let query = format!("{} {}", track.name, track.artist);
        // println!("youtube query: {}", query);
        let mut found = 0;
        // let found = AtomicUsize::new(42);
        // let mut stream: Box<dyn Stream<Item = model::YoutubeVideo> + Send> = Box::new(
        let stream = self
            .search_stream(query)
            .filter_map(|video: Result<model::YoutubeVideo>| async move {
                crate::debug!(&video);
                video.ok()
            })
            .map(|video: model::YoutubeVideo| video.into());
        match limit {
            Some(limit) => {
                Box::pin(stream.take(limit))
                // Ok(stream
                // .take(limit)
                // .collect::<Vec<proto::djtool::Track>>()
                // .await),
            }
            None => Box::pin(stream),
            // Ok(stream.collect::<Vec<proto::djtool::Track>>().await),
        }

        // let stream: Pin<Box<dyn Stream<Item = (proto::djtool::Track)> + Send + 'a>> = match limit {
        //     Some(limit) => {
        //         Box::pin(stream.take(limit))
        //         // Ok(stream
        //         // .take(limit)
        //         // .collect::<Vec<proto::djtool::Track>>()
        //         // .await),
        //     }
        //     None => Box::pin(stream),
        //     // Ok(stream.collect::<Vec<proto::djtool::Track>>().await),
        // };
        // stream
        // Box::pin(stream)
        // println!("youtube first hit: {:?}", first_hit);
        // Ok(first_hit.video_id.to_owned())
        // Ok(search_results)
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
