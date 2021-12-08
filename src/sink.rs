use super::proto;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{Date, Utc};
use std::path::{Path, PathBuf};

// #[derive(Debug, Clone)]
// pub struct OutputVideo {
//     pub info: Video,
//     pub thumbnail: Option<PathBuf>,
//     pub audio_file: PathBuf,
//     pub content_length: u64,
//     pub format: Format,
// }

// #[derive(Debug, Clone)]
// pub struct TrackDescription {
//     pub name: String,
//     pub artist: Option<String>,
//     pub album: Option<String>,
//     pub release_date: Option<Date<Utc>>,
//     pub duration: Option<u32>,
//     pub reference_audio: Option<PathBuf>,
// }

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
    pub track: proto::djtool::Track,
    pub output_path: PathBuf,
}

#[async_trait]
pub trait Sink {
    async fn download(
        &self,
        track: &proto::djtool::Track,
        output_path: &(dyn AsRef<Path> + Sync + Send),
        method: Option<Method>,
    ) -> Result<DownloadedTrack>;
}
