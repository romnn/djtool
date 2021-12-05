use super::SpotifyClient;
use super::YoutubeClient;
use anyhow::Result;
use futures_util::{StreamExt, TryStreamExt};
use rspotify_model::{Id, PlaylistId, PlaylistItem, UserId};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use warp::{Filter, Rejection, Reply};

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
    limit: Option<u32>,
}

pub async fn debug_youtube_search_handler(
    query: DebugYoutubeSearchQuery,
    youtube: YoutubeClient,
) -> std::result::Result<impl Reply, Infallible> {
    if query.parsed.unwrap_or(true) {
        Ok(warp::reply::json(
            &youtube.search(query.query).await.unwrap(),
        ))
    } else {
        Ok(warp::reply::json(
            &youtube
                .get_search_response(query.query, None)
                .await
                .unwrap(),
        ))
    }
}

pub async fn debug_spotify_playlists_handler(
    query: DebugSpotifyPlaylistsQuery,
    spotify: SpotifyClient,
) -> std::result::Result<impl Reply, Infallible> {
    let user_id = UserId::from_id(&query.user_id).unwrap();
    let playlist_id = query
        .playlist_id
        .and_then(|id| PlaylistId::from_id(&id).ok());

    let playlists = spotify
        .user_playlists_items_stream(&user_id, None, None)
        .take(query.limit.unwrap_or(1))
        .collect::<Vec<Result<PlaylistItem>>>()
        .await;
    let playlists = playlists
        .iter()
        .flat_map(|playlist| playlist.as_ref().ok())
        .collect::<Vec<&PlaylistItem>>();

    // todo: map the playlist for each playlist item
    // create library manager to check if items are already downloaded
    // check if static server works

    Ok(warp::reply::json(&playlists))
}
