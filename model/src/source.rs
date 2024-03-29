use futures::stream::Stream;
use std::pin::Pin;

pub type PlaylistStream<'a> =
    Pin<Box<dyn Stream<Item = Result<super::Playlist, Error>> + 'a + Send>>;

pub type TrackStream<'a> = Pin<Box<dyn Stream<Item = Result<super::Track, Error>> + 'a + Send>>;

pub type SearchResultStream<'a, R> = Pin<Box<dyn Stream<Item = Result<R, Error>> + 'a + Send>>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not found")]
    NotFound,
    #[error("source error: `{0:?}`")]
    Custom(#[source] Box<dyn std::error::Error + Send + Sync>),
}

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

#[async_trait::async_trait]
pub trait Source {
    fn id(&self) -> super::Service;
    // get user info (username, profile picture)

    // get track and playlist info
    async fn playlist_by_id(&self, id: &String) -> Result<Option<super::Playlist>, Error>;

    async fn track_by_id(&self, id: &String) -> Result<Option<super::Track>, Error>;

    async fn search(
        &self,
        query: SearchQuery,
        progress: Box<dyn Fn(QueryProgress) -> () + Send + 'static>,
        limit: Option<usize>,
    ) -> Vec<Result<super::Track, Error>>;

    // fn track_by_name_stream<'a>(
    //     &'a self,
    //     name: &str,
    // ) -> source::SearchResultStream<super::Track>;
    // Pin<Box<dyn Stream<Item = (super::Track)> + Send + 'a>>;

    // get stream of playlists
    fn user_playlists_stream<'a>(&'a self, user_id: &'a String) -> Result<PlaylistStream, Error>;
    // fn user_playlists_stream_test<'a>(&'a self, user_id: &'a str) -> Result<PlaylistStreamTest>;
    fn user_playlist_tracks_stream<'a>(
        &'a self,
        // playlist_id: String,
        playlist_id: super::Playlist,
    ) -> Result<TrackStream, Error>;

    // get stream of tracks based on name
    fn search_stream<'a>(
        &'a self,
        query: SearchQuery,
        progress: Box<dyn Fn(QueryProgress) -> () + Send + 'static>,
        limit: Option<usize>,
    ) -> SearchResultStream<super::Track>;

    async fn handle_user_login_callback(
        &self,
        login: super::UserLoginCallback,
    ) -> Result<(), Error>;

    // async fn reauthenticate(&self) -> Result<Option<reqwest::Url>, Error>;
  
    // fn user_playlist_tracks_stream<'a>(
    //     &'a self,
    //     playlist_id: &'a str,
    // ) -> Result<TrackStream>;

    // get stream of playlist tracks
}
