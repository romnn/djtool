#![allow(warnings)]

pub mod auth;
pub mod config;
pub mod error;
pub mod model;
pub mod source;
pub mod stream;
#[cfg(feature = "cli")]
pub mod cli;

use error::{ApiError, Error};
use reqwest::header::HeaderMap;
use rspotify_model::Id;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

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

#[macro_export]
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
                eprintln!("need user login!");
            }
            Err(err) => panic!("{}", err),
            Ok(_) => {}
        };
        // println!("authenticated");

        self.authenticator.auth_headers().await
    }

    pub async fn search_page<'a>(
        &'a self,
        search_query: djtool_model::source::SearchQuery,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<rspotify_model::SearchResult, Error> {
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
            .await
            .map_err(ApiError::from)?;
        // crate::debug!(r);
        // .json::<model::spotify::Page<model::spotify::SimplifiedPlaylist>>()
        // .await
        // .map_err(Into::into)
        unimplemented!();
    }

    pub async fn user_playlists_page(
        &self,
        user_id: String,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<rspotify_model::Page<model::spotify::SimplifiedPlaylist>, Error> {
        let user_id = rspotify_model::UserId::from_id(&user_id)
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
        res.json::<rspotify_model::Page<model::spotify::SimplifiedPlaylist>>()
            .await
            .map_err(ApiError::from)
            .map_err(Error::from)

        // .map_err(Into::into)
    }

    // pub async fn user_playlists(&self, user_id: &model::spotify::UserId) -> Vec<Result<SimplifiedPlaylist>> {
    //     self.user_playlists_stream(user_id)
    //         .collect::<Vec<Result<SimplifiedPlaylist>>>()
    //         .await
    // }

    pub async fn playlist_tracks_page(
        &self,
        playlist: model::Playlist,
        fields: Option<&str>,
        market: Option<rspotify_model::Market>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<rspotify_model::Page<model::spotify::PlaylistItem>, Error> {
        let market: Option<&str> = market.map(Into::into);
        let params = HashMap::<&str, Value>::from_iter(
            vec![
                fields.map(|fields| ("fields", fields.into())),
                market.map(|market| ("market", market.into())),
                limit.map(|limit| ("limit", limit.into())),
                offset.map(|offset| ("offset", offset.into())),
            ]
            .into_iter()
            .filter_map(|e| e),
        );

        // let playlist_id = PlaylistId::from_id(&playlist_id)?;

        // let id = model::PlaylistId(playlist.id.ok_or(Error::NotFound)?);

        // let sp_playlist_id: model::spotify::PlaylistId<'_> =
        //     id.try_into().map_err(|_| Error::NotFound)?;
        // let playlist_id =
        //     rspotify_model::PlaylistId::try_from(playlist.id.ok_or(Error::NotFound)?)?;
        let playlist_id = playlist.id.ok_or(Error::NotFound)?;
        let playlist_id =
            model::spotify::PlaylistId::try_from(playlist_id).map_err(|_| Error::NotFound)?;

        // .try_into()
        // .map_err(ApiError::from)
        // .map_err(Error::from)?;
        // .ok_or(anyhow::anyhow!("missing playlist id"))?
        // .try_into()?;
        // PlaylistId::from_id(&playlist_id)?;
        let url = api!(format!("playlists/{}/tracks", playlist_id.id()))
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

        res.json::<rspotify_model::Page<model::spotify::PlaylistItem>>()
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
    //     // ) -> Result<impl Stream<Item = Result<model::Playlist>> + 'a + Send> {
    //         }

    // pub fn user_playlists_items_stream<'a>(
    //     &'a self,
    //     // user_id: &'a model::spotify::UserId,
    //     user_id: &'a model::spotify::UserId,
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
