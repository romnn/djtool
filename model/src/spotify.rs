use anyhow::Result;

use rspotify_model as spotify_model;


// impl<'a> TryFrom<model::PlaylistId> for spotify_model::PlaylistId<'a> {
//     type Error = spotify_model::IdError;
//
//     fn try_from(id: model::PlaylistId) -> Result<spotify_model::PlaylistId<'a>, Self::Error> {
//         spotify_model::PlaylistId::from_id(id.id)
//     }
// }
//
// impl Into<model::Artwork> for spotify_model::Image {
//     fn into(img: spotify_model::Image) -> model::Artwork {
//         model::Artwork {
//             url: img.url,
//             width: img.width.unwrap_or(0),
//             height: img.height.unwrap_or(0),
//         }
//     }
// }
//
// impl From<spotify_model::Image> for model::Artwork {
//     fn from(img: spotify_model::Image) -> model::Artwork {
//         model::Artwork {
//             url: img.url,
//             width: img.width.unwrap_or(0),
//             height: img.height.unwrap_or(0),
//         }
//     }
// }

// impl From<spotify_model::SimplifiedPlaylist> for model::Playlist {
//     fn from(playlist: spotify_model::SimplifiedPlaylist) -> model::Playlist {
//         model::Playlist {
//             id: Some(model::PlaylistId {
//                 source: model::Service::Spotify as i32,
//                 id: playlist.id.id().to_string(),
//             }),
//             total: playlist.tracks.total,
//             name: playlist.name,
//             tracks: Vec::new(),
//         }
//     }
// }
//
// impl From<spotify_model::FullPlaylist> for model::Playlist {
//     fn from(playlist: spotify_model::FullPlaylist) -> model::Playlist {
//         model::Playlist {
//             id: Some(model::PlaylistId {
//                 source: model::Service::Spotify as i32,
//                 id: playlist.id.id().to_string(),
//             }),
//             total: playlist.tracks.total,
//             name: playlist.name,
//             tracks: Vec::new(),
//         }
//     }
// }
//

impl From<spotify_model::FullTrack> for super::Track {
    fn from(track: spotify_model::FullTrack) -> super::Track {
        let id = super::TrackId {
            source: super::Service::Spotify as i32,
            // tracks dont need an ID if they are local
            id: track
                .id
                .map(|id| id.id().to_string())
                .unwrap_or("unknown".to_string()),
            playlist_id: None, // unknown at this point
        };
        let mut images = track
            .album
            .images
            .into_iter()
            .map(super::Artwork::from)
            .collect::<Vec<super::Artwork>>();
        images.sort_by(|b, a| (a.width * a.height).cmp(&(b.width * b.height)));
        let artwork = images.first().map(|a| a.to_owned());

        super::Track {
            id: Some(id),
            name: track.name,
            duration_millis: track.duration.num_milliseconds() as u64,
            artwork,
            preview: track.preview_url.map(|url| super::TrackPreview { url }),
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

impl TryFrom<spotify_model::PlaylistItem> for super::Track {
    // type Error = anyhow::Error;
    type Error = std::convert::Infallible;

    fn try_from(track: spotify_model::PlaylistItem) -> Result<super::Track, Self::Error> {
        match track.track {
            Some(spotify_model::PlayableItem::Track(track)) => Ok(track.into()),
            Some(spotify_model::PlayableItem::Episode(ep)) => Ok(super::Track {
                id: Some(super::TrackId {
                    source: super::Service::Spotify as i32,
                    id: ep.id.to_string(), // episodes always have an ID
                    playlist_id: None,     // unknown at this point
                }),
                duration_millis: ep.duration.num_milliseconds() as u64,
                artwork: {
                    let mut images = ep
                        .show
                        .images
                        .into_iter()
                        .map(super::Artwork::from)
                        .collect::<Vec<super::Artwork>>();
                    images.sort_by(|b, a| (a.width * a.height).cmp(&(b.width * b.height)));
                    images.first().map(|a| a.to_owned())
                },
                // .map(Into::into),
                preview: ep.audio_preview_url.map(|url| super::TrackPreview { url }),
                name: ep.name,
                artist: ep.show.publisher,
                info: None,
            }),
            _ => Err(std::convert::Infallible),
            // Error::Api(ApiError::InvalidMediaType)),
        }
    }
}
