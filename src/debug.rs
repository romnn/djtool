// use super::SpotifyClient;
// use super::YoutubeClient;
use super::proto;
use super::DjTool;
use crate::youtube::model::YoutubeVideo;
use crate::youtube::Youtube;
use anyhow::Result;
use futures_util::pin_mut;
use futures_util::{StreamExt, TryStreamExt};
use rspotify_model::{Id, PlaylistId, PlaylistItem, UserId};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use warp::{Filter, Rejection, Reply};

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! debug {
    ($x:expr) => {
        dbg!($x)
    };
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! debug {
    ($x:expr) => {
        std::convert::identity($x)
    };
}

#[macro_export]
macro_rules! debug_to_file {
    ($file:expr, $x:expr) => {
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open($file)
            .map(|file| serde_json::to_writer_pretty(file, $x));
    };
}

#[derive(Deserialize, Clone, Debug)]
pub struct DebugSpotifyPlaylistsQuery {
    user_id: String,
    playlist_id: Option<String>,
    limit: Option<usize>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct DebugYoutubeSearchQuery {
    query: String,
    parsed: Option<bool>,
    limit: Option<usize>,
}

pub async fn debug_youtube_search_handler(
    query: DebugYoutubeSearchQuery,
    tool: DjTool,
) -> std::result::Result<impl Reply, Infallible> {
    // let stream = youtube
    //     .search_stream(query.query)
    //     .take(query.limit.unwrap_or(1));
    // let results = stream.collect::<Vec<Result<YoutubeVideo>>>().await;
    // let results = results
    //     .iter()
    //     .flat_map(|r| r.as_ref().ok())
    //     .collect::<Vec<&YoutubeVideo>>();

    // tool.sync_library().await.unwrap();
    // let results: Vec<&YoutubeVideo> = Vec::new();
    // return Ok(warp::reply::json(&results));

    let youtube = Youtube::new();
    // let sinks = tool.sinks.read().await;
    // let youtube = &sinks[&proto::djtool::Service::Youtube];
    if query.parsed.unwrap_or(true) {
        Ok(warp::reply::json(
            &youtube.search_page(query.query, None).await.unwrap(),
        ))
    } else {
        Ok(warp::reply::json(
            &youtube
                .search_page_response(query.query, None, None)
                .await
                .unwrap(),
        ))
    }
}

pub async fn debug_spotify_playlists_handler(
    query: DebugSpotifyPlaylistsQuery,
    tool: DjTool,
) -> std::result::Result<impl Reply, Infallible> {
    let user_id = UserId::from_id(&query.user_id).unwrap();
    let playlist_id = query
        .playlist_id
        .and_then(|id| PlaylistId::from_id(&id).ok());

    // let playlists = spotify
    //     .user_playlists_items_stream(&user_id, None, None)
    //     .take(query.limit.unwrap_or(1))
    //     .collect::<Vec<Result<PlaylistItem>>>()
    //     .await;
    // let playlists = playlists
    //     .iter()
    //     .flat_map(|playlist| playlist.as_ref().ok())
    //     .collect::<Vec<&PlaylistItem>>();

    tool.sync_library().await.unwrap();
    let results: Vec<&YoutubeVideo> = Vec::new();
    return Ok(warp::reply::json(&results));

    let playlists: Vec<&PlaylistItem> = Vec::new();
    // todo: map the playlist for each playlist item
    // create library manager to check if items are already downloaded
    // check if static server works

    Ok(warp::reply::json(&playlists))
}
