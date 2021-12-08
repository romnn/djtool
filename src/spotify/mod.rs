pub mod auth;
pub mod config;
pub mod error;
pub mod model;
pub mod stream;

use super::config::Persist;
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
            }
            Err(err) => panic!("{}", err),
            Ok(_) => {}
        };
        println!("authenticated");

        self.authenticator.auth_headers().await
    }

    pub async fn user_playlists_page(
        &self,
        // user_id: &model::UserId,
        user_id: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<model::Page<model::SimplifiedPlaylist>> {
        let user_id = model::UserId::from_id(user_id)?;
        let params = HashMap::<&str, Value>::from_iter(
            vec![
                limit.map(|limit| ("limit", limit.into())),
                offset.map(|offset| ("offset", offset.into())),
            ]
            .into_iter()
            .filter_map(|e| e),
        );
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

impl From<model::SimplifiedPlaylist> for proto::djtool::Playlist {
    fn from(playlist: model::SimplifiedPlaylist) -> proto::djtool::Playlist {
        proto::djtool::Playlist {
            id: Some(proto::djtool::PlaylistId {
                source: proto::djtool::Service::Spotify as i32,
                id: playlist.id.id().to_string(),
            }),
            name: playlist.name,
            tracks: Vec::new(),
        }
    }
}

impl TryFrom<model::PlaylistItem> for proto::djtool::Track {
    type Error = anyhow::Error;

    fn try_from(track: model::PlaylistItem) -> Result<proto::djtool::Track, Self::Error> {
        match track.track {
            Some(model::PlayableItem::Track(track)) => Ok(proto::djtool::Track {
                track_id: Some(proto::djtool::TrackId {
                    source: proto::djtool::Service::Spotify as i32,
                    // tracks dont need an ID if they are local
                    id: track.id.map(|id| id.to_string()).unwrap(),
                    playlist_id: None, // unknown at this point
                }),
                name: track.name,
            }),
            Some(model::PlayableItem::Episode(ep)) => Ok(proto::djtool::Track {
                track_id: Some(proto::djtool::TrackId {
                    source: proto::djtool::Service::Spotify as i32,
                    id: ep.id.to_string(), // episodes always have an ID
                    playlist_id: None,     // unknown at this point
                }),
                name: ep.name,
            }),
            _ => Err(anyhow::anyhow!("not playable")),
        }
    }
}

impl TryFrom<proto::djtool::PlaylistId> for model::PlaylistId {
    type Error = model::IdError;

    fn try_from(id: proto::djtool::PlaylistId) -> Result<model::PlaylistId, Self::Error> {
        model::PlaylistId::from_id(&id.id)
    }
}

impl Source for Spotify {
    fn id(&self) -> proto::djtool::Service {
        proto::djtool::Service::Spotify
    }

    fn user_playlists_stream<'a>(&'a self, user_id: &'a str) -> Result<PlaylistStream> {
        let playlists = paginate(
            move |limit, offset| self.user_playlists_page(&user_id, Some(limit), Some(offset)),
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
        // fn user_playlist_tracks_stream(&self, playlist_id: String) -> Result<TrackStream> {
        let playlist_clone = playlist.to_owned();
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
                    }) = t.track_id
                    {
                        *playlist_id = playlist_id.to_owned();
                    }
                    t
                })
        });
        Ok(Box::pin(tracks))
    }
}
