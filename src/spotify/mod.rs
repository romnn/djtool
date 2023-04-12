pub mod auth;
pub mod cli;
pub mod config;
pub mod error;
pub mod model;
pub mod tasks;
pub mod stream;

use super::config::Persist;
// use crate::config::Persist;
use super::proto;
use super::source;
// ::{
//     PlaylistStream, SearchQuery, SearchResultStream, SearchType, Source, TrackStream,
// };
use super::utils::{random_string, Alphanumeric, PKCECodeVerifier};
use anyhow::Result;
use async_trait::async_trait;
use base64;
use chrono::{DateTime, Duration, Utc};
use futures::stream::Stream;
use futures_util::pin_mut;
use futures_util::stream::{StreamExt, TryStreamExt};
use futures_util::TryFutureExt;
use model::Id;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::iter::FromIterator;
// use model::{
//     FullPlaylist, Id, Market, Page, PlayableItem, PlaylistId, PlaylistItem, SimplifiedPlaylist,
//     UserId,
// };
use reqwest;
use reqwest::Url;
use reqwest::{header::HeaderMap, Error as HttpError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use stream::paginate;
// use thiserror::Error;
pub use error::{ApiError, Error};
use tokio::sync::{Mutex, RwLock};
use webbrowser;

// pub const DEFAULT_API_PREFIX: &str = "https://api.spotify.com/v1/";
// pub const DEFAULT_CACHE_PATH: &str = ".spotify_token_cache.json";
pub const DEFAULT_PAGINATION_CHUNKS: u32 = 50;

#[macro_export]
macro_rules! scopes {
    ($($key:expr),*) => {{
        let mut container = ::std::collections::HashSet::new();
        $(
            container.insert($key.to_owned());
        )*
        container
    }};
}

macro_rules! api {
    ($path:expr) => {
        reqwest::Url::parse("https://api.spotify.com/v1/").and_then(|url| url.join(&$path))
    };
}

#[derive(Clone)]
pub struct Spotify {
    pub authenticator: Arc<Box<dyn auth::Authenticator + Send + Sync>>,
    pub client: Arc<reqwest::Client>,
}

impl Spotify {
    async fn auth_headers(&self) -> HeaderMap {
        match self.authenticator.reauthenticate().await {
            Err(error::Error::Auth(error::AuthError::RequireUserLogin { auth_url })) => {
                // panic!("require user confirmation: {}", auth_url);
                // todo: get write lock and set a freeze until login callback received
                println!("need user login!");
            }
            Err(err) => panic!("{}", err),
            Ok(_) => {}
        };
        // println!("authenticated");

        self.authenticator.auth_headers().await
    }

    pub async fn search_page<'a>(
        &'a self,
        search_query: source::SearchQuery,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<model::SearchResult> {
        // let user_id = model::UserId::from_id(&user_id)?;
        let query = HashMap::<&str, Value>::from_iter(
            vec![
                limit.map(|limit| ("limit", limit.into())),
                offset.map(|offset| ("offset", offset.into())),
            ]
            .into_iter()
            .filter_map(|e| e),
        );
        let headers = self.auth_headers().await;
        let r = self
            .client
            .get(api!("search")?)
            .headers(self.auth_headers().await)
            .query(&query)
            .send()
            .await?;
        crate::debug!(r);
        // .json::<model::Page<model::SimplifiedPlaylist>>()
        // .await
        // .map_err(Into::into)
        Err(anyhow::anyhow!("not yet implemented"))
    }

    pub async fn user_playlists_page(
        &self,
        user_id: String,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<model::Page<model::SimplifiedPlaylist>, Error> {
        let user_id = model::UserId::from_id(&user_id)
            .map_err(ApiError::from)
            .map_err(Error::from)?;

        let params = HashMap::<&str, Value>::from_iter(
            vec![
                limit.map(|limit| ("limit", limit.into())),
                offset.map(|offset| ("offset", offset.into())),
            ]
            .into_iter()
            .filter_map(|e| e),
        );
        // println!("making the playlist request");
        let headers = self.auth_headers().await;
        // println!("auth headers: {:?}", headers);
        // let test = self
        //     .client
        //     .get(api!(format!("users/{}/playlists", user_id.id()))?)
        //     .headers(headers)
        //     .query(&params)
        //     .send()
        //     .await?
        //     .json::<serde_json::Value>()
        //     .await?;
        // println!("user playlists page: {:?}", test);

        let url = api!(format!("users/{}/playlists", user_id.id()))
            .map_err(ApiError::from)
            .map_err(Error::from)?;
        let res = self
            .client
            .get(url)
            .headers(self.auth_headers().await)
            .query(&params)
            .send()
            .await
            .map_err(ApiError::from)
            .map_err(Error::from)?;

        // let playlist = res
        res.json::<model::Page<model::SimplifiedPlaylist>>()
            .await
            .map_err(ApiError::from)
            .map_err(Error::from)

        // .map_err(Into::into)
    }

    // pub async fn user_playlists(&self, user_id: &model::UserId) -> Vec<Result<SimplifiedPlaylist>> {
    //     self.user_playlists_stream(user_id)
    //         .collect::<Vec<Result<SimplifiedPlaylist>>>()
    //         .await
    // }

    pub async fn playlist_tracks_page(
        &self,
        // playlist_id: PlaylistId,
        // playlist_id: String,
        playlist: proto::djtool::Playlist,
        fields: Option<&str>,
        market: Option<model::Market>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<model::Page<model::PlaylistItem>, Error> {
        let market: Option<&str>= market.map(Into::into);
        let params = HashMap::<&str, Value>::from_iter(
            vec![
                fields.map(|fields| ("fields", fields.into())),
                market.map(|market| ("market", market.into())),
                // market.map(|market| ("market", Into::<String>::into(market).into())),
                // market.and_then(|market("market", Into::<String>::into(market).into())),
                limit.map(|limit| ("limit", limit.into())),
                offset.map(|offset| ("offset", offset.into())),
            ]
            .into_iter()
            .filter_map(|e| e),
        );

        // let playlist_id = PlaylistId::from_id(&playlist_id)?;

        let sp_playlist_id: model::PlaylistId = playlist
            .id
            .ok_or(Error::NotFound)?
            .try_into()
            .map_err(ApiError::from)
            .map_err(Error::from)?;
        // .ok_or(anyhow::anyhow!("missing playlist id"))?
        // .try_into()?;
        // PlaylistId::from_id(&playlist_id)?;
        let url = api!(format!("playlists/{}/tracks", sp_playlist_id.id()))
            .map_err(ApiError::from)
            .map_err(Error::from)?;

        let res = self
            .client
            .get(url)
            .headers(self.auth_headers().await)
            .query(&params)
            .send()
            .await
            .map_err(ApiError::from)
            .map_err(Error::from)?;

        // let item = res
        res.json::<model::Page<model::PlaylistItem>>()
            .await
            .map_err(ApiError::from)
            .map_err(Error::from)
    }

    // pub async fn playlist_items(
    //     &self,
    //     playlist_id: &PlaylistId,
    //     fields: Option<&str>,
    //     market: Option<&Market>,
    // ) -> Vec<Result<PlaylistItem>> {
    //     self.playlist_items_stream(playlist_id.to_owned(), fields, market)
    //         .collect::<Vec<Result<PlaylistItem>>>()
    //         .await
    // }

    // pub fn playlist_items_stream<'a>(
    //     &'a self,
    //     playlist_id: PlaylistId,
    //     fields: Option<&'a str>,
    //     market: Option<&'a Market>,
    // ) -> impl Stream<Item = Result<PlaylistItem>> + 'a + Send {
    //     paginate(
    //         move |limit, offset| {
    //             self.playlist_items_page(
    //                 playlist_id.to_owned(),
    //                 fields,
    //                 market,
    //                 Some(limit),
    //                 Some(offset),
    //             )
    //         },
    //         DEFAULT_PAGINATION_CHUNKS,
    //     )
    // }

    // fn sp_user_playlists_stream<'a>(&'a self, user_id: &'a str) -> Result<PlaylistStream> {
    //     // ) -> Result<impl Stream<Item = Result<proto::djtool::Playlist>> + 'a + Send> {
    //         }

    // pub fn user_playlists_items_stream<'a>(
    //     &'a self,
    //     // user_id: &'a model::UserId,
    //     user_id: &'a model::UserId,
    //     fields: Option<&'a str>,
    //     market: Option<&'a Market>,
    // ) -> impl Stream<Item = Result<PlaylistItem>> + 'a + Send {
    //     let playlist_stream = self.user_playlists_stream(user_id);
    //     playlist_stream.flat_map(move |playlist| {
    //         self.playlist_items_stream(playlist.unwrap().id, fields, market)
    //     })
    // }

    // pub async fn user_playlists_items(
    //     &self,
    //     user_id: &UserId,
    //     fields: Option<&str>,
    //     market: Option<&Market>,
    // ) -> Vec<Result<PlaylistItem>> {
    //     self.user_playlists_items_stream(user_id, fields, market)
    //         .collect::<Vec<Result<PlaylistItem>>>()
    //         .await
    // }
}
// pub fn user_playlists_stream<'a>(
//         &'a self,
//         user_id: &'a UserId,
//     ) -> impl Stream<Item = Result<SimplifiedPlaylist>> + 'a + Send {
//         paginate(
//             move |limit, offset| self.user_playlists_page(&user_id, Some(limit), Some(offset)),
//             DEFAULT_PAGINATION_CHUNKS,
//         )
//     }
//
//
// const SPOTIFY_SOURCE_ID: &'static str = "SPOTIFY";

impl From<model::Image> for proto::djtool::Artwork {
    fn from(img: model::Image) -> proto::djtool::Artwork {
        proto::djtool::Artwork {
            url: img.url,
            width: img.width.unwrap_or(0),
            height: img.height.unwrap_or(0),
        }
    }
}

impl From<model::SimplifiedPlaylist> for proto::djtool::Playlist {
    fn from(playlist: model::SimplifiedPlaylist) -> proto::djtool::Playlist {
        proto::djtool::Playlist {
            id: Some(proto::djtool::PlaylistId {
                source: proto::djtool::Service::Spotify as i32,
                id: playlist.id.id().to_string(),
            }),
            total: playlist.tracks.total,
            name: playlist.name,
            tracks: Vec::new(),
        }
    }
}

impl From<model::FullPlaylist> for proto::djtool::Playlist {
    fn from(playlist: model::FullPlaylist) -> proto::djtool::Playlist {
        proto::djtool::Playlist {
            id: Some(proto::djtool::PlaylistId {
                source: proto::djtool::Service::Spotify as i32,
                // id: "fuck".to_string(), // playlist.id.id(),
                id: playlist.id.id().to_string(),
                // id: playlist.id.id().to_string().strip_prefix("spotify:track:")
            }),
            total: playlist.tracks.total,
            name: playlist.name,
            tracks: Vec::new(),
        }
    }
}

impl TryFrom<model::PlaylistItem> for proto::djtool::Track {
    // type Error = anyhow::Error;
    type Error = Error;

    fn try_from(track: model::PlaylistItem) -> Result<proto::djtool::Track, Self::Error> {
        match track.track {
            Some(model::PlayableItem::Track(track)) => Ok(track.into()),
            //                 Ok(proto::djtool::Track {
            //                 id: Some(proto::djtool::TrackId {
            //                     source: proto::djtool::Service::Spotify as i32,
            //                     // tracks dont need an ID if they are local
            //                     id: track.id.map(|id| id.to_string()).unwrap(),
            //                     playlist_id: None, // unknown at this point
            //                 }),
            //                 name: track.name,
            //                 artwork: {
            //                     let mut images = track
            //                         .album
            //                         .images
            //                         .into_iter()
            //                         .map(proto::djtool::Artwork::from)
            //                         .collect::<Vec<proto::djtool::Artwork>>();
            //                     images.sort_by(|b, a| (a.width * a.height).cmp(&(b.width * b.height)));
            //                     images.first().map(|a| a.to_owned())
            //                 },
            //                 preview: track
            //                     .preview_url
            //                     .map(|url| proto::djtool::TrackPreview { url }),
            //                 artist: track
            //                     .artists
            //                     .into_iter()
            //                     .map(|a| a.name)
            //                     .collect::<Vec<String>>()
            //                     .join(", "),
            //             }),
            Some(model::PlayableItem::Episode(ep)) => Ok(proto::djtool::Track {
                id: Some(proto::djtool::TrackId {
                    source: proto::djtool::Service::Spotify as i32,
                    id: ep.id.to_string(), // episodes always have an ID
                    playlist_id: None,     // unknown at this point
                }),
                duration_millis: ep.duration.as_millis() as u64,
                artwork: {
                    let mut images = ep
                        .show
                        .images
                        .into_iter()
                        .map(proto::djtool::Artwork::from)
                        .collect::<Vec<proto::djtool::Artwork>>();
                    images.sort_by(|b, a| (a.width * a.height).cmp(&(b.width * b.height)));
                    images.first().map(|a| a.to_owned())
                },
                // .map(Into::into),
                preview: ep
                    .audio_preview_url
                    .map(|url| proto::djtool::TrackPreview { url }),
                name: ep.name,
                artist: ep.show.publisher,
                info: None,
            }),
            // _ => Err(anyhow::anyhow!("not playable")),
            _ => Err(Error::Api(ApiError::InvalidMediaType)),
        }
    }
}

impl From<model::FullTrack> for proto::djtool::Track {
    fn from(track: model::FullTrack) -> proto::djtool::Track {
        proto::djtool::Track {
            id: Some(proto::djtool::TrackId {
                source: proto::djtool::Service::Spotify as i32,
                // tracks dont need an ID if they are local
                id: track
                    .id
                    .map(|id| id.id().to_string())
                    .unwrap_or("unknown".to_string()),
                playlist_id: None, // unknown at this point
            }),
            name: track.name,
            duration_millis: track.duration.as_millis() as u64,
            artwork: {
                let mut images = track
                    .album
                    .images
                    .into_iter()
                    .map(proto::djtool::Artwork::from)
                    .collect::<Vec<proto::djtool::Artwork>>();
                images.sort_by(|b, a| (a.width * a.height).cmp(&(b.width * b.height)));
                images.first().map(|a| a.to_owned())
            },
            preview: track
                .preview_url
                .map(|url| proto::djtool::TrackPreview { url }),
            artist: track
                .artists
                .into_iter()
                .map(|a| a.name)
                .collect::<Vec<String>>()
                .join(", "),
            info: None,
        }
    }
}

// #[derive(thiserror::Error, Debug)]
// pub enum ApiError {
//     #[error("http error: `{0:?}`")]
//     Http(#[from] reqwest::Error),
//     #[error("url parse error: `{0:?}`")]
//     ParseError(#[from] url::ParseError),
//     #[error("invalid id: `{0:?}`")]
//     InvalidID(#[from] model::IdError),
//     #[error("invalid media type (neither track or episode)")]
//     InvalidMediaType,
// }

// #[derive(thiserror::Error, Debug)]
// pub enum Error {
//     #[error("api error: `{0:?}`")]
//     Api(#[from] ApiError),
//     #[error("not found")]
//     NotFound,

//     #[error("search result is not of type `{0:?}`")]
//     InvalidSearchResultType(model::SearchType),
//     #[error("unknown spotify error: `{0:?}`")]
//     Unknown(Box<dyn std::error::Error + Send + Sync>),
// }

// impl Into<source::Error> for Error {
//     fn from(err: Error) -> source::Error {
//         source::Error::Custom(err.into())
//     }
// }

// impl std::error::Error for Error {}

// impl TryInto<model::SearchResult> for model::Page<model::FullTrack> {
//     type Error = Error;

//     fn try_from(result: model::SearchResult) -> Result<model::Page<model::FullTrack>, Self::Error> {
//         match result {
//             model::SearchResult::Tracks(track_page) => Ok(track_page),
//             _ => Err(Error::SearchResultInvalidType(SearchResultType)),
//         }
//         // proto::djtool::Track {
//         //     id: Some(proto::djtool::TrackId {
//         //         source: proto::djtool::Service::Spotify as i32,
//         //         // tracks dont need an ID if they are local
//         //         id: track
//         //             .id
//         //             .map(|id| id.id().to_string())
//         //             .unwrap_or("unknown".to_string()),
//         //         playlist_id: None, // unknown at this point
//         //     }),
//         //     name: track.name,
//         //     duration_millis: track.duration.as_millis() as u64,
//         //     artwork: {
//         //         let mut images = track
//         //             .album
//         //             .images
//         //             .into_iter()
//         //             .map(proto::djtool::Artwork::from)
//         //             .collect::<Vec<proto::djtool::Artwork>>();
//         //         images.sort_by(|b, a| (a.width * a.height).cmp(&(b.width * b.height)));
//         //         images.first().map(|a| a.to_owned())
//         //     },
//         //     preview: track
//         //         .preview_url
//         //         .map(|url| proto::djtool::TrackPreview { url }),
//         //     artist: track
//         //         .artists
//         //         .into_iter()
//         //         .map(|a| a.name)
//         //         .collect::<Vec<String>>()
//         //         .join(", "),
//         //     info: None,
//         // }
//     }
// }

impl<'a> TryFrom<proto::djtool::PlaylistId> for model::PlaylistId<'a> {
    type Error = model::IdError;

    fn try_from(id: proto::djtool::PlaylistId) -> Result<model::PlaylistId<'a>, Self::Error> {
        model::PlaylistId::from_id(id.id)
    }
}

#[async_trait]
impl source::Source for Spotify {
    fn id(&self) -> proto::djtool::Service {
        proto::djtool::Service::Spotify
    }

    async fn reauthenticate(&self) -> Result<Option<reqwest::Url>, source::Error> {
        match self.authenticator.reauthenticate().await {
            Err(crate::spotify::error::Error::Auth(
                crate::spotify::error::AuthError::RequireUserLogin { auth_url },
            )) => Ok(Some(auth_url)),
            Err(err) => Err(err.into()),
            // todo
            // Err(_) => Ok(None),
            Ok(_) => Ok(None),
        }
    }

    async fn playlist_by_id(
        &self,
        id: &String,
    ) -> Result<Option<proto::djtool::Playlist>, source::Error> {
        let url = api!(format!("playlists/{}", id))
            .map_err(ApiError::from)
            .map_err(Error::from)?;
        let res = self
            .client
            .get(url)
            .headers(self.auth_headers().await)
            .send()
            .await
            .map_err(ApiError::from)
            .map_err(Error::from)?;
        // println!("playlist by id: {:?}", res);
        match res.status() {
            reqwest::StatusCode::OK => {
                let playlist = res
                    .json::<model::FullPlaylist>()
                    .await
                    .map_err(ApiError::from)
                    .map_err(Error::from)?;
                Ok(Some(playlist.into()))
            }
            reqwest::StatusCode::BAD_REQUEST => Ok(None),
            _ => res
                .error_for_status()
                .map(|_| None)
                .map_err(ApiError::from)
                .map_err(Error::from)
                .map_err(source::Error::from),
        }
        // println!("response: {:?}", res.json::<serde_json::Value>().await);
    }

    async fn track_by_id(
        &self,
        id: &String,
    ) -> Result<Option<proto::djtool::Track>, source::Error> {
        let url = api!(format!("tracks/{}", id))
            .map_err(ApiError::from)
            .map_err(Error::from)?;

        let res = self
            .client
            .get(url)
            .headers(self.auth_headers().await)
            .send()
            .await
            .map_err(ApiError::from)
            .map_err(Error::from)?;

        match res.status() {
            reqwest::StatusCode::OK => {
                let track = res
                    .json::<model::FullTrack>()
                    .await
                    .map_err(ApiError::from)
                    .map_err(Error::from)?;

                Ok(Some(track.into()))
            }
            reqwest::StatusCode::BAD_REQUEST => Ok(None),
            _ => res
                .error_for_status()
                .map(|_| None)
                .map_err(ApiError::from)
                .map_err(Error::from)
                .map_err(source::Error::from),
        }
        // println!("response: {:?}", res.json::<serde_json::Value>().await);
    }

    async fn search(
        &self,
        query: source::SearchQuery,
        progress: Box<dyn Fn(source::QueryProgress) -> () + Send + 'static>,
        limit: Option<usize>,
    ) -> Vec<Result<proto::djtool::Track, source::Error>> {
        let stream = self.search_stream(query, progress, limit);
        stream
            .collect::<Vec<Result<proto::djtool::Track, _>>>()
            .await
    }

    fn search_stream<'a>(
        &'a self,
        query: source::SearchQuery,
        progress: Box<dyn Fn(source::QueryProgress) -> () + Send + 'static>,
        limit: Option<usize>,
    ) -> source::SearchResultStream<proto::djtool::Track> {
        let search_results = paginate(
            move |limit, offset| {
                let query = query.clone();
                async move {
                    // let search_result: Result<model::Page<model::FullTrack>> = self
                    let search_result = self
                        .search_page(query, Some(limit), Some(offset))
                        .await;
                    match search_result {
                        Ok(model::SearchResult::Tracks(track_page)) => Ok(track_page),
                        Ok(_) => Err(Error::InvalidSearchResultType(
                            // or from the source?
                            model::SearchType::Track,
                        )),
                        Err(err) => Err(Error::Unknown(err.into())),
                        // Box::new(err.into()))),
                    }

                    // .and_then(|search_result: model::SearchResult| search_result;
                    // search_result
                }

                // let tracks_page: Result<model::Page<model::FullTrack>> = self
                // .search_page(query, Some(limit), Some(offset))
                // .try_into();
                // tracks_page
            },
            DEFAULT_PAGINATION_CHUNKS,
        );
        let tracks = search_results
            .map(|track| track.map(|t| t.into()))
            .map_err(|err| err.into());

        //source::Error::Custom(err.into()));
        // Box::new(err.into())));
        // Box::pin(tracks)
        match limit {
            Some(limit) => {
                Box::pin(tracks.take(limit))
                // Ok(stream
                // .take(limit)
                // .collect::<Vec<proto::djtool::Track>>()
                // .await),
            }
            None => Box::pin(tracks),
            // Ok(stream.collect::<Vec<proto::djtool::Track>>().await),
        }

        // Err(anyhow::anyhow!("not implemented"))
    }

    // async fn track_by_name(&self, id: &str) -> Result<Vec<proto::djtool::Track>> {
    //     let res = self
    //         .client
    //         .get(api!(format!("search/{}", id))?)
    //         .headers(self.auth_headers().await)
    //         .send()
    //         .await?;
    //     match res.status() {
    //         reqwest::StatusCode::OK => {
    //             let track = res.json::<model::FullTrack>().await?;
    //             Ok(Some(track.into()))
    //         }
    //         reqwest::StatusCode::BAD_REQUEST => Ok(None),
    //         _ => res.error_for_status().map_err(Into::into).map(|_| None),
    //     }
    //     // println!("response: {:?}", res.json::<serde_json::Value>().await);
    // }

    fn user_playlists_stream<'a>(
        &'a self,
        user_id: &'a String,
    ) -> Result<source::PlaylistStream, source::Error> {
        let playlists = paginate(
            move |limit, offset| {
                self.user_playlists_page(user_id.to_owned(), Some(limit), Some(offset))
            },
            DEFAULT_PAGINATION_CHUNKS,
        );
        let playlists = playlists
            .map(|playlist| playlist.map(|p| p.into()))
            .map_err(source::Error::from);
        // .map_err(|err| err.into()); // source::Error::Custom(err.into()));
        Ok(Box::pin(playlists))
    }

    // fn user_playlist_tracks_stream<'a>(&'a self, playlist_id: String) -> Result<TrackStream> {
    fn user_playlist_tracks_stream<'a>(
        &'a self,
        playlist: proto::djtool::Playlist,
    ) -> Result<source::TrackStream, source::Error> {
        // println!("playlist: {:?}", playlist);
        let playlist_clone = playlist.to_owned();
        let playlist_id_clone = Arc::new(playlist.id.to_owned());
        let tracks = paginate(
            move |limit, offset| {
                self.playlist_tracks_page(
                    playlist_clone.to_owned(),
                    None,
                    None,
                    Some(limit),
                    Some(offset),
                )
            },
            DEFAULT_PAGINATION_CHUNKS,
        );
        let tracks = tracks
            .map(move |track| {
                track
                    .and_then(|t| t.try_into())
                    .map(|mut t: proto::djtool::Track| {
                        if let Some(proto::djtool::TrackId {
                            ref mut playlist_id,
                            ..
                        }) = t.id
                        {
                            *playlist_id = playlist_id_clone.deref().to_owned();
                        }
                        t
                    })
            })
            // .map_err(|err| source::Error::Custom(err.into()));
            .map_err(source::Error::from);
        Ok(Box::pin(tracks))
    }

    async fn handle_user_login_callback(
        &self,
        login: proto::djtool::UserLoginCallback,
    ) -> Result<(), source::Error> {
        match login {
            proto::djtool::UserLoginCallback {
                login: Some(proto::djtool::user_login_callback::Login::SpotifyLogin(login_test)),
            } => self
                .authenticator
                .handle_user_login_callback(login_test)
                .await
                .map_err(source::Error::from),
            _ => panic!("wrong login callback received"),
        }
    }
}
