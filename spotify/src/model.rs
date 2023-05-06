pub use djtool_model::*;

pub mod spotify {
    use crate::error::{ApiError, Error};
    use djtool_model as model;
    use rspotify_model::Id;
    use serde::{Deserialize, Serialize};
    // use rspotify_model;

    //
    // pub trait TryIntoTrack {
    //     type Error;
    //     fn into_playlist_id(self) -> model::PlaylistId;
    // }
    //
    // impl<'a> TryIntoPlaylistId for spotify_model::PlaylistId<'a> {
    //     type Error = spotify_model::IdError;
    //
    //     fn into_playlist_id(self) -> model::PlaylistId {
    //         spotify_model::PlaylistId::from_id(self.id)
    //     }
    // }

    // pub trait TryIntoPlaylistId {
    //     type Error;
    //     fn into_playlist_id(self) -> model::PlaylistId;
    // }
    //
    // impl<'a> TryIntoPlaylistId for spotify_model::PlaylistId<'a> {
    //     type Error = spotify_model::IdError;
    //
    //     fn into_playlist_id(self) -> model::PlaylistId {
    //         spotify_model::PlaylistId::from_id(self.id)
    //     }
    // }

    #[repr(transparent)]
    #[derive(Serialize, Deserialize)]
    pub struct SimplifiedPlaylist(pub rspotify_model::SimplifiedPlaylist);

    impl std::ops::Deref for SimplifiedPlaylist {
        type Target = rspotify_model::SimplifiedPlaylist;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl From<SimplifiedPlaylist> for model::Playlist {
        fn from(playlist: SimplifiedPlaylist) -> Self {
            let playlist = playlist.0;
            let id = model::PlaylistId {
                source: model::Service::Spotify as i32,
                id: playlist.id.id().to_string(),
            };
            model::Playlist {
                id: Some(id),
                total: playlist.tracks.total,
                name: playlist.name,
                tracks: Vec::new(),
            }
        }
    }

    #[repr(transparent)]
    #[derive(Serialize, Deserialize)]
    pub struct Image(pub rspotify_model::Image);

    impl std::ops::Deref for Image {
        type Target = rspotify_model::Image;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl From<Image> for model::Artwork {
        fn from(img: Image) -> Self {
            model::Artwork {
                url: img.url.clone(),
                width: img.width.unwrap_or(0),
                height: img.height.unwrap_or(0),
            }
        }
    }

    // #[repr(transparent)]
    // pub struct Artwork(pub model::Artwork);
    //
    // impl std::ops::Deref for Artwork {
    //     type Target = model::Artwork;
    //
    //     fn deref(&self) -> &Self::Target {
    //         &self.0
    //     }
    // }
    //
    // impl From<spotify_model::Image> for Artwork {
    //     fn from(img: spotify_model::Image) -> Self {
    //         Self(model::Artwork {
    //             url: img.url,
    //             width: img.width.unwrap_or(0),
    //             height: img.height.unwrap_or(0),
    //         })
    //     }
    // }

    #[repr(transparent)]
    pub struct PlaylistId<'a>(pub rspotify_model::PlaylistId<'a>);

    impl<'a> std::ops::Deref for PlaylistId<'a> {
        type Target = rspotify_model::PlaylistId<'a>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<'a> TryFrom<model::PlaylistId> for PlaylistId<'a> {
        type Error = rspotify_model::IdError;

        fn try_from(id: model::PlaylistId) -> Result<Self, Self::Error> {
            let id = rspotify_model::PlaylistId::from_id(id.id)?;
            Ok(Self(id))
        }
    }

    // #[repr(transparent)]
    // pub struct PlaylistId(pub model::PlaylistId);
    //
    // impl<'a> TryFrom<PlaylistId> for spotify_model::PlaylistId<'a> {
    //     type Error = spotify_model::IdError;
    //
    //     fn try_from(id: PlaylistId) -> Result<Self, Self::Error> {
    //         Self::from_id(id.id)
    //     }
    // }

    //

    #[repr(transparent)]
    #[derive(Serialize, Deserialize)]
    pub struct PlaylistItem(pub rspotify_model::PlaylistItem);

    impl std::ops::Deref for PlaylistItem {
        type Target = rspotify_model::PlaylistItem;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl TryFrom<PlaylistItem> for model::Track {
        type Error = Error;

        fn try_from(track: PlaylistItem) -> Result<Self, Self::Error> {
            use rspotify_model::PlayableItem;
            match track.0.track {
                Some(PlayableItem::Track(track)) => Ok(FullTrack(track).into()),
                Some(PlayableItem::Episode(ep)) => {
                    let id = model::TrackId {
                        source: model::Service::Spotify as i32,
                        id: ep.id.to_string(), // episodes always have an ID
                        playlist_id: None,     // unknown at this point
                    };
                    let mut images = ep
                        .show
                        .images
                        .into_iter()
                        .map(Image)
                        .map(model::Artwork::from)
                        .collect::<Vec<model::Artwork>>();
                    images.sort_by(|b, a| (a.width * a.height).cmp(&(b.width * b.height)));
                    let artwork = images.first().map(|a| a.to_owned());

                    let preview = ep.audio_preview_url.map(|url| model::TrackPreview { url });
                    Ok(model::Track {
                        id: Some(id),
                        duration_millis: ep.duration.num_milliseconds() as u64,
                        artwork,
                        preview,
                        name: ep.name,
                        artist: ep.show.publisher,
                        info: None,
                    })
                }
                _ => Err(Error::Api(ApiError::InvalidMediaType)),
            }
        }
    }

    #[repr(transparent)]
    #[derive(Deserialize, Serialize)]
    pub struct FullPlaylist(pub rspotify_model::FullPlaylist);

    impl std::ops::Deref for FullPlaylist {
        type Target = rspotify_model::FullPlaylist;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl From<FullPlaylist> for model::Playlist {
        fn from(playlist: FullPlaylist) -> Self {
            let playlist = playlist.0;
            Self {
                id: Some(model::PlaylistId {
                    source: model::Service::Spotify as i32,
                    id: playlist.id.id().to_string(),
                }),
                total: playlist.tracks.total,
                name: playlist.name,
                tracks: Vec::new(),
            }
        }
    }

    #[repr(transparent)]
    #[derive(Serialize, Deserialize)]
    pub struct FullTrack(pub rspotify_model::FullTrack);

    impl std::ops::Deref for FullTrack {
        type Target = rspotify_model::FullTrack;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl From<FullTrack> for model::Track {
        fn from(track: FullTrack) -> Self {
            let track = track.0;
            let id = model::TrackId {
                source: model::Service::Spotify as i32,
                // tracks dont need an ID if they are local
                id: track
                    .id
                    .map(|id| id.id().to_string())
                    .unwrap_or("unknown".to_string()),
                playlist_id: None, // unknown at this point
            };
            let mut images: Vec<model::Artwork> = track
                .album
                .images
                .into_iter()
                .map(Image)
                .map(model::Artwork::from)
                .collect();

            images.sort_by(|b, a| (a.width * a.height).cmp(&(b.width * b.height)));
            let artwork = images.first().map(|a| a.to_owned());

            let preview = track.preview_url.map(|url| model::TrackPreview { url });
            let artist = track
                .artists
                .into_iter()
                .map(|a| a.name)
                .collect::<Vec<String>>()
                .join(", ");

            model::Track {
                id: Some(id),
                name: track.name,
                duration_millis: track.duration.num_milliseconds() as u64,
                artwork,
                preview,
                artist,
                info: None,
            }
        }
    }

    // // impl TryInto<rspotify_model::SearchResult> for rspotify_model::Page<rspotify_model::FullTrack> {
    // //     type Error = Error;
    //
    // //     fn try_from(result: rspotify_model::SearchResult) -> Result<rspotify_model::Page<rspotify_model::FullTrack>, Self::Error> {
    // //         match result {
    // //             rspotify_model::SearchResult::Tracks(track_page) => Ok(track_page),
    // //             _ => Err(Error::SearchResultInvalidType(SearchResultType)),
    // //         }
    // //         // model::Track {
    // //         //     id: Some(model::TrackId {
    // //         //         source: model::Service::Spotify as i32,
    // //         //         // tracks dont need an ID if they are local
    // //         //         id: track
    // //         //             .id
    // //         //             .map(|id| id.id().to_string())
    // //         //             .unwrap_or("unknown".to_string()),
    // //         //         playlist_id: None, // unknown at this point
    // //         //     }),
    // //         //     name: track.name,
    // //         //     duration_millis: track.duration.as_millis() as u64,
    // //         //     artwork: {
    // //         //         let mut images = track
    // //         //             .album
    // //         //             .images
    // //         //             .into_iter()
    // //         //             .map(model::Artwork::from)
    // //         //             .collect::<Vec<model::Artwork>>();
    // //         //         images.sort_by(|b, a| (a.width * a.height).cmp(&(b.width * b.height)));
    // //         //         images.first().map(|a| a.to_owned())
    // //         //     },
    // //         //     preview: track
    // //         //         .preview_url
    // //         //         .map(|url| model::TrackPreview { url }),
    // //         //     artist: track
    // //         //         .artists
    // //         //         .into_iter()
    // //         //         .map(|a| a.name)
    // //         //         .collect::<Vec<String>>()
    // //         //         .join(", "),
    // //         //     info: None,
    // //         // }
    // //     }
    // // }
}
