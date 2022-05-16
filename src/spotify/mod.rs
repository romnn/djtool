pub mod auth;
pub mod cli;
pub mod config;
pub mod error;
pub mod model;
pub mod stream;

use super::config::Persist;
// use crate::config::Persist;
use super::proto;
use super::source::{PlaylistStream, Source, TrackStream};
use super::utils::{random_string, Alphanumeric, PKCECodeVerifier};
use anyhow::Result;
use async_trait::async_trait;
use base64;
use chrono::{DateTime, Duration, Utc};
use futures::stream::Stream;
use futures_util::pin_mut;
use futures_util::stream::{StreamExt, TryStreamExt};
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
use thiserror::Error;
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

    pub async fn user_playlists_page(
        &self,
        // user_id: &model::UserId,
        user_id: String,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<model::Page<model::SimplifiedPlaylist>> {
        let user_id = model::UserId::from_id(&user_id)?;
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

        self.client
            .get(api!(format!("users/{}/playlists", user_id.id()))?)
            .headers(self.auth_headers().await)
            .query(&params)
            .send()
            .await?
            .json::<model::Page<model::SimplifiedPlaylist>>()
            .await
            .map_err(Into::into)
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
        market: Option<&model::Market>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<model::Page<model::PlaylistItem>> {
        let params = HashMap::<&str, Value>::from_iter(
            vec![
                fields.map(|fields| ("fields", fields.into())),
                market.map(|market| ("market", market.as_ref().into())),
                limit.map(|limit| ("limit", limit.into())),
                offset.map(|offset| ("offset", offset.into())),
            ]
            .into_iter()
            .filter_map(|e| e),
        );

        // let playlist_id = PlaylistId::from_id(&playlist_id)?;

        let sp_playlist_id: model::PlaylistId = playlist
            .id
            .ok_or(anyhow::anyhow!("missing playlist id"))?
            .try_into()?;
        // PlaylistId::from_id(&playlist_id)?;
        self.client
            .get(api!(format!("playlists/{}/tracks", sp_playlist_id.id()))?)
            .headers(self.auth_headers().await)
            .query(&params)
            .send()
            .await?
            .json::<model::Page<model::PlaylistItem>>()
            .await
            .map_err(Into::into)
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
    type Error = anyhow::Error;

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
            _ => Err(anyhow::anyhow!("not playable")),
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

impl TryFrom<proto::djtool::PlaylistId> for model::PlaylistId {
    type Error = model::IdError;

    fn try_from(id: proto::djtool::PlaylistId) -> Result<model::PlaylistId, Self::Error> {
        model::PlaylistId::from_id(&id.id)
    }
}

#[async_trait]
impl Source for Spotify {
    fn id(&self) -> proto::djtool::Service {
        proto::djtool::Service::Spotify
    }

    async fn reauthenticate(&self) -> Result<Option<reqwest::Url>> {
        match self.authenticator.reauthenticate().await {
            Err(crate::spotify::error::Error::Auth(
                crate::spotify::error::AuthError::RequireUserLogin { auth_url },
            )) => Ok(Some(auth_url)),
            Err(err) => Err(err.into()),
            Ok(_) => Ok(None),
        }
    }

    async fn playlist_by_id(&self, id: &String) -> Result<Option<proto::djtool::Playlist>> {
        let res = self
            .client
            .get(api!(format!("playlists/{}", id))?)
            .headers(self.auth_headers().await)
            .send()
            .await?;
        // println!("playlist by id: {:?}", res);
        match res.status() {
            reqwest::StatusCode::OK => {
                let playlist = res.json::<model::FullPlaylist>().await?;
                Ok(Some(playlist.into()))
                // Ok(None)
            }
            reqwest::StatusCode::BAD_REQUEST => Ok(None),
            _ => res.error_for_status().map_err(Into::into).map(|_| None),
        }
        // println!("response: {:?}", res.json::<serde_json::Value>().await);
    }

    async fn track_by_id(&self, id: &String) -> Result<Option<proto::djtool::Track>> {
        let res = self
            .client
            .get(api!(format!("tracks/{}", id))?)
            .headers(self.auth_headers().await)
            .send()
            .await?;
        match res.status() {
            reqwest::StatusCode::OK => {
                let track = res.json::<model::FullTrack>().await?;
                Ok(Some(track.into()))
            }
            reqwest::StatusCode::BAD_REQUEST => Ok(None),
            _ => res.error_for_status().map_err(Into::into).map(|_| None),
        }
        // println!("response: {:?}", res.json::<serde_json::Value>().await);
    }

    fn user_playlists_stream<'a>(&'a self, user_id: &'a String) -> Result<PlaylistStream> {
        let playlists = paginate(
            move |limit, offset| {
                self.user_playlists_page(user_id.to_owned(), Some(limit), Some(offset))
            },
            DEFAULT_PAGINATION_CHUNKS,
        );
        let playlists = playlists.map(|playlist| playlist.map(|p| p.into()));
        Ok(Box::pin(playlists))
    }

    // fn user_playlist_tracks_stream<'a>(&'a self, playlist_id: String) -> Result<TrackStream> {
    fn user_playlist_tracks_stream<'a>(
        &'a self,
        playlist: proto::djtool::Playlist,
    ) -> Result<TrackStream> {
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
        let tracks = tracks.map(move |track| {
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
        });
        Ok(Box::pin(tracks))
    }

    async fn handle_user_login_callback(
        &self,
        login: proto::djtool::UserLoginCallback,
    ) -> Result<()> {
        match login {
            proto::djtool::UserLoginCallback {
                login: Some(proto::djtool::user_login_callback::Login::SpotifyLogin(login_test)),
            } => {
                self.authenticator
                    .handle_user_login_callback(login_test)
                    .await
            }
            _ => panic!("wrong login callback received"),
        }
    }
}
