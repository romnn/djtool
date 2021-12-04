use super::image::Image;
use super::serialization::duration_ms;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FullTrack {
    pub album: SimplifiedAlbum,
    pub artists: Vec<SimplifiedArtist>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub available_markets: Vec<String>,
    pub disc_number: i32,
    #[serde(with = "duration_ms", rename = "duration_ms")]
    pub duration: Duration,
    pub explicit: bool,
    pub external_ids: HashMap<String, String>,
    pub external_urls: HashMap<String, String>,
    pub href: Option<String>,
    pub id: TrackId,
    pub is_local: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_playable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_from: Option<TrackLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restrictions: Option<Restriction>,
    pub name: String,
    pub popularity: u32,
    pub preview_url: Option<String>,
    pub track_number: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrackLink {
    pub external_urls: HashMap<String, String>,
    pub href: String,
    pub id: TrackId,
}

#[derive(Deserialize)]
pub struct FullTracks {
    pub tracks: Vec<FullTrack>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SimplifiedTrack {
    pub artists: Vec<SimplifiedArtist>,
    pub available_markets: Option<Vec<String>>,
    pub disc_number: i32,
    #[serde(with = "duration_ms", rename = "duration_ms")]
    pub duration: Duration,
    pub explicit: bool,
    pub external_urls: HashMap<String, String>,
    #[serde(default)]
    pub href: Option<String>,
    pub id: Option<TrackId>,
    pub is_local: bool,
    pub is_playable: Option<bool>,
    pub linked_from: Option<TrackLink>,
    pub restrictions: Option<Restriction>,
    pub name: String,
    pub preview_url: Option<String>,
    pub track_number: u32,
}
