use anyhow::Result;
use async_trait::async_trait;
use chrono::{Utc, Date};
use std::path::{Path, PathBuf};

// #[derive(Debug, Clone)]
// pub struct OutputVideo {
//     pub info: Video,
//     pub thumbnail: Option<PathBuf>,
//     pub audio_file: PathBuf,
//     pub content_length: u64,
//     pub format: Format,
// }

#[derive(Debug, Clone)]
pub struct TrackDescription {
    name: String,
    artist: Option<String>,
    album: Option<String>,
    release_date: Option<Date<Utc>>,
    duration: Option<u32>,
    reference_audio: Option<PathBuf>,
}

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

#[async_trait]
pub trait ExtractorBackend {
    async fn download<P: AsRef<Path> + Send + Sync>(
        track: TrackDescription,
        output_file: P,
        method: Method,
    ) -> Result<()>;
}
