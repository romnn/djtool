use anyhow::Result;
use async_trait::async_trait;
use chrono::{Utc, Date};
use std::path::{Path, PathBuf};

// #[derive(Debug, Clone)]
// pub struct TrackDescription {
//     name: String,
//     artist: Option<String>,
//     album: Option<String>,
//     release_date: Option<Date<Utc>>,
//     duration: Option<u32>,
//     reference_audio: Option<PathBuf>,
// }

// #[derive(Debug, Clone)]
// pub enum Method {
//     Best {
//         max_candidates: Option<u32>,
//         min_confidence: Option<f32>,
//     },
//     Fast {
//         max_candidates: Option<u32>,
//         min_confidence: Option<f32>,
//     },
//     First,
// }

#[async_trait]
pub trait Source {
    // use protos for the interface types here
    // get user info (username, profile picture)
    // get stream of playlists
    // get stream of playlist tracks
}
