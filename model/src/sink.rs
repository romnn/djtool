use futures::stream::Stream;
use std::path::{Path, PathBuf};
use std::pin::Pin;

#[derive(Debug, Clone)]
pub enum Method {
    Best {
        max_candidates: Option<u32>,
        min_confidence: Option<f32>,
    },
    Fast {
        max_candidates: Option<u32>,
        min_confidence: Option<f32>,
    },
    First,
}

#[derive(Debug, Clone)]
pub struct DownloadedTrack {
    pub track: super::Track,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct QueryProgress {}

#[derive(thiserror::Error, Debug)]
pub enum Error {}

#[derive(Debug)]
pub struct DownloadProgress {}

// progress: Option<Box<dyn Fn(DownloadProgress) -> () + Send + Sync + 'static>>,

#[async_trait::async_trait]
pub trait Sink: Send + Sync + 'static {
    async fn audio_download_url(&self, track: &super::Track) -> Result<(String, String), Error>;

    async fn download(
        &self,
        track: &super::Track,
        output_path: &(dyn AsRef<Path> + Sync + Send),
        method: Option<Method>,
        progress: &(dyn Fn(DownloadProgress)),
    ) -> Result<DownloadedTrack, Error>;

    async fn candidates(
        &self,
        track: &super::Track,
        progress: Box<dyn Fn(QueryProgress) -> () + Send + 'static>,
        limit: Option<usize>,
    ) -> Vec<super::Track>;

    fn candidates_stream<'b, 'a>(
        &'a self,
        track: &'b super::Track,
        progress: Box<dyn Fn(QueryProgress) -> () + Send + 'static>,
        limit: Option<usize>,
    ) -> Pin<Box<dyn Stream<Item = super::Track> + Send + 'a>>;
}
