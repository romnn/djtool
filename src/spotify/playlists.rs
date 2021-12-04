use super::image::Image;
use super::iter::Page;
use super::show;
use super::track;
use super::user::PublicUser;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Hash)]
pub struct PlaylistId(String);

// impl Id for PlaylistId {
//     #[inline]
//     fn id(&self) -> &str {
//         &self.0
//     }

//     #[inline]
//     fn _type(&self) -> Type {
//         Type::Playlist
//     }

//     #[inline]
//     fn _type_static() -> Type
//     where
//         Self: Sized,
//     {
//         Type::Playlist
//     }

//     #[inline]
//     unsafe fn from_id_unchecked(id: &str) -> Self {
//         Self(id.to_owned())
//     }
// }
//

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Followers {
    // This field will always set to null, as the Web API does not support it at the moment.
    // pub href: Option<String>,
    pub total: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum PlayableItem {
    Track(track::FullTrack),
    Episode(show::FullEpisode),
}

impl PlayableItem {
    // pub fn id(&self) -> Option<&dyn PlayableId> {
    //     match self {
    //         PlayableItem::Track(t) => t.id.as_ref().map(|t| t as &dyn PlayableId),
    //         PlayableItem::Episode(e) => Some(&e.id),
    //     }
    // }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PlaylistTracksRef {
    pub href: String,
    pub total: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimplifiedPlaylist {
    pub collaborative: bool,
    pub external_urls: HashMap<String, String>,
    pub href: String,
    pub id: PlaylistId,
    pub images: Vec<Image>,
    pub name: String,
    pub owner: PublicUser,
    pub public: Option<bool>,
    pub snapshot_id: String,
    pub tracks: PlaylistTracksRef,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FullPlaylist {
    pub collaborative: bool,
    pub description: Option<String>,
    pub external_urls: HashMap<String, String>,
    // pub followers: Followers,
    pub href: String,
    pub id: PlaylistId,
    pub images: Vec<Image>,
    pub name: String,
    pub owner: PublicUser,
    pub public: Option<bool>,
    pub snapshot_id: String,
    pub tracks: Page<PlaylistItem>,
}

/// Playlist track object
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PlaylistItem {
    pub added_at: Option<DateTime<Utc>>,
    pub added_by: Option<PublicUser>,
    pub is_local: bool,
    pub track: Option<PlayableItem>,
}
