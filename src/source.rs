use super::proto;
// use anyhow::Result;
use super::spotify;
use async_trait::async_trait;
use chrono::{Date, Utc};
use futures::stream::Stream;
use std::path::{Path, PathBuf};
use std::pin::Pin;

pub type PlaylistStream<'a> =
    Pin<Box<dyn Stream<Item = Result<proto::djtool::Playlist, Error>> + 'a + Send>>;

pub type TrackStream<'a> =
    Pin<Box<dyn Stream<Item = Result<proto::djtool::Track, Error>> + 'a + Send>>;

pub type SearchResultStream<'a, R> = Pin<Box<dyn Stream<Item = Result<R, Error>> + 'a + Send>>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // todo: add some common error types
    #[error("spotify error: `{0:?}`")]
    Spotify(#[from] spotify::Error),
    // Spotify {
    //     #[from]
    //     source: spotify::Error,
    //     // backtrace: std::backtrace::Backtrace,
    // },
    #[error("not found")]
    NotFound,
    #[error("source error: `{0:?}`")]
    // Custom(Box<dyn std::error::Error + Send>),
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

// impl<T: std::error::Error> From<T> for Error {
//     fn from(err: T) -> Error {
//         Error::Custom(err.into())
//     }
// }

#[derive(Clone, Debug)]
pub enum SearchFilterYear {
    Year(u64),
    Range(u64, u64),
}

#[derive(Clone, Debug)]
pub enum SearchQueryFilter {
    Album(String),
    Artist(String),
    Track(String),
    Year(SearchFilterYear),
    // Upc,
    // Hipster,
    // New,
    // Isrc,
    Genre(String),
}

#[derive(Clone, Debug)]
pub enum SearchType {
    Artist,
    Album,
    Track,
    Playlist,
    Show,
    Episode,
}

#[derive(Clone, Debug)]
pub struct SearchQuery {
    pub types: Vec<SearchType>,
    pub query: Vec<SearchQueryFilter>,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            types: vec![SearchType::Track],
            query: vec![],
        }
    }
}

impl SearchQuery {
    pub fn track<S: Into<String>>(name: S, artist: Option<S>) -> Self {
        let mut query = vec![SearchQueryFilter::Track(name.into())];
        if let Some(artist) = artist {
            query.push(SearchQueryFilter::Artist(artist.into()));
        }
        Self {
            types: vec![SearchType::Track],
            query,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryProgress {}

#[async_trait]
pub trait Source {
    fn id(&self) -> proto::djtool::Service;
    // use protos for the interface types here
    // get user info (username, profile picture)

    // get track and playlist info
    async fn playlist_by_id(&self, id: &String) -> Result<Option<proto::djtool::Playlist>, Error>;

    async fn track_by_id(&self, id: &String) -> Result<Option<proto::djtool::Track>, Error>;

    async fn search(
        &self,
        query: SearchQuery,
        progress: Box<dyn Fn(QueryProgress) -> () + Send + 'static>,
        limit: Option<usize>,
    ) -> Vec<Result<proto::djtool::Track, Error>>;

    // fn track_by_name_stream<'a>(
    //     &'a self,
    //     name: &str,
    // ) -> source::SearchResultStream<proto::djtool::Track>;
    // Pin<Box<dyn Stream<Item = (proto::djtool::Track)> + Send + 'a>>;

    // get stream of playlists
    fn user_playlists_stream<'a>(&'a self, user_id: &'a String) -> Result<PlaylistStream, Error>;
    // fn user_playlists_stream_test<'a>(&'a self, user_id: &'a str) -> Result<PlaylistStreamTest>;
    fn user_playlist_tracks_stream<'a>(
        &'a self,
        // playlist_id: String,
        playlist_id: proto::djtool::Playlist,
    ) -> Result<TrackStream, Error>;

    // get stream of tracks based on name
    fn search_stream<'a>(
        &'a self,
        query: SearchQuery,
        progress: Box<dyn Fn(QueryProgress) -> () + Send + 'static>,
        limit: Option<usize>,
    ) -> SearchResultStream<proto::djtool::Track>;

    async fn handle_user_login_callback(
        &self,
        login: proto::djtool::UserLoginCallback,
    ) -> Result<(), Error>;

    async fn reauthenticate(&self) -> Result<Option<reqwest::Url>, Error>;
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
