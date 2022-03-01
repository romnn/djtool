mod extractor;
pub mod model;
mod rank;
mod search;
mod stream;

use crate::download::Download;
use crate::proto;
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
    ) -> Result<DownloadedTrack> {
        // search the video first
        let method = method.unwrap_or(Method::First);
        let video_id = self.find_best_video(&track, method).await?;
        let video = self.get_video(&video_id).await?;
        let audio_formats = video.formats.audio();
        if audio_formats.len() < 1 {
            panic!("todo: error when no audio formats");
        }
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
        let mut download = Download::new(&stream_url, &output_path).await?;
        download.start().await?;

        Ok(DownloadedTrack {
            track: proto::djtool::Track {
                name: track.name.to_owned(),
                artist: track.artist.to_owned(),
                artwork: None,
                preview: None,
                track_id: Some(proto::djtool::TrackId {
                    id: video_id.to_owned(),
                    source: proto::djtool::Service::Youtube as i32,
                    playlist_id: None,
                }),
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
