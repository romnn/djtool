use crate::error::{ApiError, AuthError, Error};
use crate::{model, stream::paginate};
use djtool_model::source;
use futures::{StreamExt, TryStreamExt};
use std::sync::Arc;

pub const DEFAULT_PAGINATION_CHUNKS: u32 = 50;

#[async_trait::async_trait]
impl source::Source for crate::Spotify {
    fn id(&self) -> model::Service {
        model::Service::Spotify
    }

    // async fn reauthenticate(&self) -> Result<Option<reqwest::Url>, source::Error> {
    //     match self.authenticator.reauthenticate().await {
    //         Err(Error::Auth(AuthError::RequireUserLogin { auth_url })) => Ok(Some(auth_url)),
    //         Err(err) => Err(source::Error::Custom(err.into())),
    //         Ok(_) => Ok(None),
    //     }
    // }

    async fn playlist_by_id(&self, id: &String) -> Result<Option<model::Playlist>, source::Error> {
        let url = crate::api!(format!("playlists/{}", id))
            .map_err(ApiError::from)
            .map_err(Error::from)
            .map_err(|err| source::Error::Custom(err.into()))?;
        let res = self
            .client
            .get(url)
            .headers(self.auth_headers().await)
            .send()
            .await
            .map_err(ApiError::from)
            .map_err(Error::from)
            .map_err(|err| source::Error::Custom(err.into()))?;
        // println!("playlist by id: {:?}", res);
        match res.status() {
            reqwest::StatusCode::OK => {
                let playlist = res
                    .json::<model::spotify::FullPlaylist>()
                    .await
                    .map_err(ApiError::from)
                    .map_err(Error::from)
                    .map_err(|err| source::Error::Custom(err.into()))?;
                Ok(Some(playlist.into()))
            }
            reqwest::StatusCode::BAD_REQUEST => Ok(None),
            _ => res
                .error_for_status()
                .map(|_| None)
                .map_err(ApiError::from)
                .map_err(Error::from)
                .map_err(|err| source::Error::Custom(err.into())),
        }
        // println!("response: {:?}", res.json::<serde_json::Value>().await);
    }

    async fn track_by_id(&self, id: &String) -> Result<Option<model::Track>, source::Error> {
        let url = crate::api!(format!("tracks/{}", id))
            .map_err(ApiError::from)
            .map_err(Error::from)
            .map_err(|err| source::Error::Custom(err.into()))?;

        let res = self
            .client
            .get(url)
            .headers(self.auth_headers().await)
            .send()
            .await
            .map_err(ApiError::from)
            .map_err(Error::from)
            .map_err(|err| source::Error::Custom(err.into()))?;

        match res.status() {
            reqwest::StatusCode::OK => {
                let track = res
                    .json::<model::spotify::FullTrack>()
                    .await
                    .map_err(ApiError::from)
                    .map_err(|err| source::Error::Custom(err.into()))?;

                Ok(Some(track.into()))
            }
            reqwest::StatusCode::BAD_REQUEST => Ok(None),
            _ => res
                .error_for_status()
                .map(|_| None)
                .map_err(ApiError::from)
                .map_err(Error::from)
                .map_err(|err| source::Error::Custom(err.into())),
        }
        // println!("response: {:?}", res.json::<serde_json::Value>().await);
    }

    async fn search(
        &self,
        query: source::SearchQuery,
        progress: Box<dyn Fn(source::QueryProgress) -> () + Send + 'static>,
        limit: Option<usize>,
    ) -> Vec<Result<model::Track, source::Error>> {
        let stream = self.search_stream(query, progress, limit);
        stream.collect::<Vec<Result<model::Track, _>>>().await
    }

    fn search_stream<'a>(
        &'a self,
        query: source::SearchQuery,
        progress: Box<dyn Fn(source::QueryProgress) -> () + Send + 'static>,
        limit: Option<usize>,
    ) -> source::SearchResultStream<model::Track> {
        let search_results = paginate(
            move |limit, offset| {
                let query = query.clone();
                async move {
                    let search_result = self.search_page(query, Some(limit), Some(offset)).await;
                    match search_result {
                        Ok(rspotify_model::SearchResult::Tracks(track_page)) => Ok(track_page),
                        Ok(_) => Err(Error::InvalidSearchResultType(
                            // or from the source?
                            rspotify_model::SearchType::Track,
                        )),
                        Err(err) => Err(Error::Unknown(err.into())),
                    }
                }
            },
            DEFAULT_PAGINATION_CHUNKS,
        );
        let tracks = search_results
            .map(|track| track.map(|t| model::spotify::FullTrack(t).into()))
            .map_err(|err| source::Error::Custom(err.into()));

        match limit {
            Some(limit) => Box::pin(tracks.take(limit)),
            None => Box::pin(tracks),
        }
    }

    // async fn track_by_name(&self, id: &str) -> Result<Vec<model::Track>> {
    //     let res = self
    //         .client
    //         .get(crate::api!(format!("search/{}", id))?)
    //         .headers(self.auth_headers().await)
    //         .send()
    //         .await?;
    //     match res.status() {
    //         reqwest::StatusCode::OK => {
    //             let track = res.json::<rspotify_model::FullTrack>().await?;
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
            .map(|playlist: Result<model::spotify::SimplifiedPlaylist, _>| {
                playlist.map(|p| p.into())
            })
            .map_err(|err| source::Error::Custom(err.into()));
        Ok(Box::pin(playlists))
    }

    // fn user_playlist_tracks_stream<'a>(&'a self, playlist_id: String) -> Result<TrackStream> {
    fn user_playlist_tracks_stream<'a>(
        &'a self,
        playlist: model::Playlist,
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
                    .and_then(model::Track::try_from)
                    .map(|mut t: model::Track| {
                        if let Some(model::TrackId {
                            ref mut playlist_id,
                            ..
                        }) = t.id
                        {
                            *playlist_id = (*playlist_id_clone).to_owned();
                        }
                        t
                    })
            })
            .map_err(|err| source::Error::Custom(err.into()));
        // .map_err(source::Error::from);
        Ok(Box::pin(tracks))
    }

    async fn handle_user_login_callback(
        &self,
        login: model::UserLoginCallback,
    ) -> Result<(), source::Error> {
        match login {
            model::UserLoginCallback {
                login: Some(model::user_login_callback::Login::SpotifyLogin(login_test)),
            } => self
                .authenticator
                .handle_user_login_callback(login_test)
                .await
                .map_err(|err| source::Error::Custom(err.into())),
            // .map_err(source::Error::from),
            _ => panic!("wrong login callback received"),
        }
    }
}
