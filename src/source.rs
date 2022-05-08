use super::proto;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{Date, Utc};
use futures::stream::Stream;
use std::path::{Path, PathBuf};
use std::pin::Pin;

pub type PlaylistStream<'a> =
    Pin<Box<dyn Stream<Item = Result<proto::djtool::Playlist>> + 'a + Send>>;
pub type TrackStream<'a> = Pin<Box<dyn Stream<Item = Result<proto::djtool::Track>> + 'a + Send>>;

#[async_trait]
pub trait Source {
    fn id(&self) -> proto::djtool::Service;
    // use protos for the interface types here
    // get user info (username, profile picture)

    // get track and playlist info
    async fn playlist_by_id(&self, id: &String) -> Result<Option<proto::djtool::Playlist>>;

    async fn track_by_id(&self, id: &String) -> Result<Option<proto::djtool::Track>>;

    // get stream of playlists
    fn user_playlists_stream<'a>(&'a self, user_id: &'a String) -> Result<PlaylistStream>;
    // fn user_playlists_stream_test<'a>(&'a self, user_id: &'a str) -> Result<PlaylistStreamTest>;
    fn user_playlist_tracks_stream<'a>(
        &'a self,
        // playlist_id: String,
        playlist_id: proto::djtool::Playlist,
    ) -> Result<TrackStream>;

    async fn handle_user_login_callback(
        &self,
        login: proto::djtool::UserLoginCallback,
    ) -> Result<()>;

    async fn reauthenticate(&self) -> Result<Option<reqwest::Url>>;
    // fn user_playlist_tracks_stream<'a>(
    //     &'a self,
    //     playlist_id: &'a str,
    // ) -> Result<TrackStream>;

    // get stream of playlist tracks
}

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
