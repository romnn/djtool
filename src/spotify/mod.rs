pub mod auth;
pub mod config;
pub mod error;
pub mod iter;

use super::config::Persist;
use super::utils::{random_string, Alphanumeric, PKCECodeVerifier};
use anyhow::Result;
use async_stream::stream;
use async_trait::async_trait;
use base64;
use chrono::{DateTime, Duration, Utc};
use futures::stream::{self, Stream};
use futures_util::pin_mut;
use futures_util::stream::{StreamExt, TryStreamExt};
// use futures::stream::{StreamExt, TryStreamExt};
use std::iter::Iterator;
// use futures_util::StreamExt;
use iter::{paginate, Paginator};
use reqwest;
use reqwest::Url;
use reqwest::{header::HeaderMap, Error as HttpError};
use rspotify_model::{
    FullPlaylist, Id, Market, Page, PlaylistId, PlaylistItem, SimplifiedPlaylist, UserId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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

    pub async fn user_playlists_manual(
        &self,
        user_id: &UserId,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Page<SimplifiedPlaylist>> {
        let params = HashMap::<&str, Value>::from_iter(
            vec![
                limit.map(|limit| ("limit", limit.into())),
                offset.map(|offset| ("offset", offset.into())),
            ]
            .into_iter()
            .filter_map(|e| e),
        );
        // let res = self
        //     .client
        //     .get(api!(format!("users/{}/playlists", user_id.id()))?)
        //     .headers(self.auth_headers().await)
        //     .query(&params)
        //     .send()
        //     .await?
        //     .text()
        //     .await?;
        // // println!("url: {:?}", URL.as_string());
        // println!("params: {:?}", params);
        // println!("headers: {:?}", self.auth_headers().await);
        // println!("response: {}", res);
        self.client
            .get(api!(format!("users/{}/playlists", user_id.id()))?)
            .headers(self.auth_headers().await)
            .query(&params)
            .send()
            .await?
            .json::<Page<SimplifiedPlaylist>>()
            .await
            .map_err(Into::into)
    }

    pub fn user_playlists_stream<'a>(
        &'a self,
        user_id: &'a UserId,
    ) -> impl Stream<Item = Result<SimplifiedPlaylist>> + 'a + Send {
        paginate(
            move |limit, offset| self.user_playlists_manual(&user_id, Some(limit), Some(offset)),
            DEFAULT_PAGINATION_CHUNKS,
        )
    }

    pub async fn user_playlists(&self, user_id: &UserId) -> Vec<Result<SimplifiedPlaylist>> {
        self.user_playlists_stream(user_id)
            .collect::<Vec<Result<SimplifiedPlaylist>>>()
            .await
    }

    pub async fn playlist_items_manual(
        &self,
        playlist_id: PlaylistId,
        fields: Option<&str>,
        market: Option<&Market>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Page<PlaylistItem>> {
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

        self.client
            .get(api!(format!("playlists/{}/tracks", playlist_id.id()))?)
            .headers(self.auth_headers().await)
            .query(&params)
            .send()
            .await?
            .json::<Page<PlaylistItem>>()
            .await
            .map_err(Into::into)
    }

    pub async fn playlist_items(
        &self,
        playlist_id: &PlaylistId,
        fields: Option<&str>,
        market: Option<&Market>,
    ) -> Vec<Result<PlaylistItem>> {
        self.playlist_items_stream(playlist_id.to_owned(), fields, market)
            .collect::<Vec<Result<PlaylistItem>>>()
            .await
    }

    pub fn playlist_items_stream<'a>(
        &'a self,
        playlist_id: PlaylistId,
        fields: Option<&'a str>,
        market: Option<&'a Market>,
    ) -> impl Stream<Item = Result<PlaylistItem>> + 'a + Send {
        paginate(
            move |limit, offset| {
                self.playlist_items_manual(
                    playlist_id.to_owned(),
                    fields,
                    market,
                    Some(limit),
                    Some(offset),
                )
            },
            DEFAULT_PAGINATION_CHUNKS,
        )
    }

    pub fn user_playlists_items_stream<'a>(
        &'a self,
        user_id: &'a UserId,
        fields: Option<&'a str>,
        market: Option<&'a Market>,
    ) -> impl Stream<Item = Result<PlaylistItem>> + 'a + Send {
        let playlist_stream = self.user_playlists_stream(user_id);
        playlist_stream.flat_map(move |playlist| {
            self.playlist_items_stream(playlist.unwrap().id, fields, market)
        })
    }

    pub async fn user_playlists_items(
        &self,
        user_id: &UserId,
        fields: Option<&str>,
        market: Option<&Market>,
    ) -> Vec<Result<PlaylistItem>> {
        self.user_playlists_items_stream(user_id, fields, market)
            .collect::<Vec<Result<PlaylistItem>>>()
            .await
    }
}
