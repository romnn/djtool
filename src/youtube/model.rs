use crate::utils;
use anyhow::Result;
use boa;
use chrono;
use futures_util::{stream, StreamExt};
use http::header::HeaderMap;
use lazy_static::lazy_static;
use rayon::prelude::*;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::default::Default;
use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tempdir::TempDir;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, Mutex};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VideoRendererSimpleText {
    pub simple_text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VideoRendererTextRun {
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VideoRendererTextRuns {
    pub runs: Vec<VideoRendererTextRun>,
}

impl VideoRendererTextRuns {
    pub fn to_str(&self) -> Option<&str> {
        self.runs
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<&str>>()
            .first()
            .map(|s| s.to_owned())
    }
}

impl fmt::Debug for VideoRendererTextRuns {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str().unwrap_or(""))
    }
}

// fn from_duration_string<'de, D>(deserializer: D) -> Result<Option<chrono::Duration>, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     // let s: &str = Deserialize::deserialize(deserializer)?;
//     Deserialize::deserialize(deserializer).ok().map(|s| s.split(":")).and_then(|s| {
//         if s.len() == 2 {
//             chrono::Duration::seconds(60 * min *
//         } else {
//             None
//         }
//     })
//     let s = s.split(":");
//             // do better hex decoding than this

//     u64::from_str_radix(&s[2..], 16).map_err(D::Error::custom)
// }

// fn ok_or_default<T, D>(deserializer: D) -> Result<T, D::Error>
// where
//     T: Deserialize + Default,
//     D: Deserializer,
fn ok_or_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default,
    D: Deserializer<'de>,
{
    let v: Value = Deserialize::deserialize(deserializer)?;
    Ok(T::deserialize(v).unwrap_or_default())
}

// #[derive(Deserialize, Debug, Clone)]
// #[serde(rename_all = "camelCase")]
// struct VideoRendererDuration {
//     // #[serde(deserialize_with = "from_duration_string")]
//     // pub simple_text: chrono::Duration,
//     pub simple_text: String,
// }

// #[derive(Deserialize, Debug, Clone)]
// #[serde(rename_all = "camelCase")]
// struct VideoRendererViewCount{
//     // #[serde(deserialize_with = "from_duration_string")]
//     // pub simple_text: chrono::Duration,
//     pub simple_text: String,
// }
//
// deserialize_with = "ok_or_default",

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "style")]
pub enum OwnerBadgeStyle {
    BadgeStyleTypeVerifiedArtist,
    // BadgeStyleTypeVerified,
    BadgeStyleTypeUnknown,
}

impl Default for OwnerBadgeStyle {
    fn default() -> Self {
        Self::BadgeStyleTypeUnknown
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VideoRendererOwnerBadge {
    #[serde(deserialize_with = "ok_or_default")]
    pub metadata_badge_renderer: OwnerBadgeStyle,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VideoRenderer {
    pub video_id: String,
    pub thumbnail: Thumbnails,
    pub title: VideoRendererTextRuns,
    pub length_text: VideoRendererSimpleText,
    pub view_count_text: VideoRendererSimpleText,
    pub owner_badges: Option<Vec<VideoRendererOwnerBadge>>,
    pub owner_text: VideoRendererTextRuns,
    // TODO: use as fallback for the owner text
    pub short_byline_text: VideoRendererTextRuns,
    // TODO: use for finding thumbnailOverlayTimeStatusRenderer', 'text', 'simpleText'
    pub thumbnail_overlays: Option<Value>,
}

impl fmt::Debug for VideoRenderer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VideoRenderer")
            .field("id", &self.video_id)
            .field("title", &self.title)
            .field("user", &self.owner_text) // .iter().map(|r| r.title).first().unwrap_or(""))
            .finish()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemSectionRendererItem {
    pub video_renderer: VideoRenderer,
    // pub playlist_renderer: PlaylistRenderer,
    // pub playlist_renderer: PlaylistRenderer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContinuationCommand {
    pub token: String,
    pub request: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContinuationEndpoint {
    continuation_command: ContinuationCommand,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum SearchResult {
    #[serde(rename_all = "camelCase")]
    ItemSectionRenderer {
        contents: Vec<ItemSectionRendererItem>,
    },
    #[serde(rename_all = "camelCase")]
    RichItemRenderer {},
    #[serde(rename_all = "camelCase")]
    ContinuationItemRenderer {
        continuation_endpoint: ContinuationEndpoint,
        // possible recovery: ['continuationItemRenderer']['button']['buttonRenderer']['command']
        button: Option<Value>,
    },
}

impl fmt::Debug for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ItemSectionRenderer { contents } => f
                .debug_struct("SectionRenderer")
                .field("contents", contents)
                .finish(),
            Self::RichItemRenderer { .. } => f.debug_struct("RichRenderer").finish(),
            Self::ContinuationItemRenderer { .. } => {
                f.debug_struct("ContinuationRenderer").finish()
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultPage {
    pub results: Vec<SearchResult>,
}

impl fmt::Debug for SearchResultPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.results.iter()).finish()
    }
}

#[derive(Debug, Clone)]
pub struct InnertubeConfig<'a> {
    pub api_key: &'static str,
    pub client_name: &'static str,
    pub client_version: &'a str,
    pub client_hl: &'a str,
    pub host: &'a str,
    pub context_client_name: u32,
    pub require_js_player: bool,
}

impl Default for InnertubeConfig<'_> {
    fn default() -> Self {
        Self {
            api_key: "",
            client_name: "",
            client_version: "",
            client_hl: "en",
            host: "www.youtube.com",
            context_client_name: 1,
            require_js_player: true,
        }
    }
}

impl InnertubeConfig<'_> {
    pub fn context(&self) -> Value {
        json!({
            "client": {
                "clientName": self.client_name,
                "clientVersion": self.client_version,
                "hl": self.client_hl,
            }
        })
    }
}

#[derive(Debug, Clone)]
pub enum Innertube {
    Web,
    Android,
}

impl Innertube {
    pub fn config(&self) -> InnertubeConfig {
        match *self {
            Self::Web => InnertubeConfig {
                api_key: "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8",
                client_name: "WEB",
                client_version: "2.20210622.10.00",
                context_client_name: 1,
                ..Default::default()
            },
            Self::Android => InnertubeConfig {
                api_key: "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8",
                client_name: "ANDROID",
                client_version: "16.20",
                context_client_name: 3,
                require_js_player: false,
                ..Default::default()
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InnertubeClient {
    pub hl: String,
    pub gl: String,
    pub client_name: String,
    pub client_version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InnertubeContext {
    pub client: InnertubeClient,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContentPlaybackContext {
    pub signature_timestamp: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackContext {
    pub content_playback_context: ContentPlaybackContext,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InnertubeRequest {
    pub video_id: String,
    pub context: InnertubeContext,
    pub playback_context: PlaybackContext,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnail {
    pub url: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FormatRange {
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Format {
    pub itag: Option<i32>,
    pub url: Option<String>,
    pub mime_type: Option<String>,
    pub quality: Option<String>,
    pub signature_cipher: Option<String>,
    pub bitrate: Option<i32>,
    pub fps: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub last_modified: Option<String>,
    pub content_length: Option<String>,
    pub quality_label: Option<String>,
    pub projection_type: Option<String>,
    pub average_bitrate: Option<i32>,
    pub audio_quality: Option<String>,
    pub approx_duration_ms: Option<String>,
    pub audio_sample_rate: Option<String>,
    pub audio_channels: Option<i16>,

    // InitRange is only available for adaptive formats
    pub init_range: Option<FormatRange>,

    // IndexRange is only available for adaptive formats
    pub index_range: Option<FormatRange>,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum Codec {
    // audio
    OPUS,
    MP4,
    // video
    AV01,
    VP9,
    AVC1,
    // other
    OTHER,
}

impl Format {
    pub fn codec(&self) -> Codec {
        let c = self
            .mime_type
            .as_ref()
            .map(|mime| {
                let mime = mime.to_lowercase();
                if mime.contains("mp4") {
                    return Codec::MP4;
                } else if mime.contains("opus") {
                    return Codec::OPUS;
                } else if mime.contains("av01") {
                    return Codec::AV01;
                } else if mime.contains("vp9") {
                    return Codec::VP9;
                } else if mime.contains("avc1") {
                    return Codec::AVC1;
                }
                Codec::OTHER
            })
            .unwrap_or(Codec::OTHER);
        c
    }
}

#[derive(Debug, Clone)]
pub struct FormatList {
    pub formats: Vec<Format>,
}

impl From<Vec<Format>> for FormatList {
    fn from(mut formats: Vec<Format>) -> FormatList {
        formats.sort_by(FormatList::cmp_format);
        FormatList { formats }
    }
}

impl Deref for FormatList {
    type Target = Vec<Format>;

    fn deref(&self) -> &Self::Target {
        &self.formats
    }
}

impl FormatList {
    #[allow(dead_code)]
    fn sort(&mut self) {
        self.formats.sort_by(Self::cmp_format);
    }

    fn with_mime_type(&self, substr: &str) -> Vec<&Format> {
        self.formats
            .iter()
            .filter(|f| {
                f.mime_type
                    .as_ref()
                    .map(|m| m.to_lowercase().contains(substr))
                    .unwrap_or(false)
            })
            .collect()
    }

    fn audio(&self) -> Vec<&Format> {
        self.with_mime_type("audio")
    }

    #[allow(dead_code)]
    fn video(&self) -> Vec<&Format> {
        self.with_mime_type("video")
    }

    fn cmp_format(a: &Format, b: &Format) -> Ordering {
        // sort by width
        if a.width == b.width {
            // Format 137 downloads slowly, give it less priority
            // see https://github.com/kkdai/youtube/pull/171
            if a.itag == Some(137) {
                return Ordering::Less;
            }
            if b.itag == Some(137) {
                return Ordering::Greater;
            }
            // sort by fps
            if a.fps == b.fps {
                let a_audio_channels = a.audio_channels.unwrap_or(0);
                let b_audio_channels = b.audio_channels.unwrap_or(0);
                if a.fps.is_none()
            // if a.fps.unwrap_or(0) == 0
                && a_audio_channels > 0
                && b_audio_channels > 0
                {
                    // audio
                    // sort by codec
                    if a.codec() == b.codec() {
                        // sort by audio channel
                        if a_audio_channels == b_audio_channels {
                            // sort by audio bitrate
                            if a.bitrate == b.bitrate {
                                // sort by audio sample rate
                                return b.audio_sample_rate.cmp(&a.audio_sample_rate);
                            }
                            return b.bitrate.cmp(&a.bitrate);
                        }
                        return b_audio_channels.cmp(&a_audio_channels);
                    }
                    let mut rank: HashMap<Codec, u16> = HashMap::new();
                    rank.insert(Codec::MP4, 2);
                    rank.insert(Codec::OPUS, 1);
                    return rank
                        .get(&b.codec())
                        .unwrap_or(&0)
                        .cmp(&rank.get(&a.codec()).unwrap_or(&0));
                }
                // video
                // sort by codec
                let mut rank: HashMap<Codec, u16> = HashMap::new();
                rank.insert(Codec::AV01, 3);
                rank.insert(Codec::VP9, 2);
                rank.insert(Codec::AVC1, 1);

                if a.codec() == b.codec() {
                    // sort by audio bitrate
                    return b.bitrate.cmp(&a.bitrate);
                }
                return rank
                    .get(&b.codec())
                    .unwrap_or(&0)
                    .cmp(&rank.get(&a.codec()).unwrap_or(&0));
            }
            return b.fps.unwrap_or(0).cmp(&a.fps.unwrap_or(0));
        }
        return b.width.unwrap_or(0).cmp(&a.width.unwrap_or(0));
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnails {
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MiniplayerRenderer {
    pub playback_mode: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Miniplayer {
    pub miniplayer_renderer: MiniplayerRenderer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayabilityStatus {
    pub status: Option<String>,
    pub reason: Option<String>,
    pub playable_in_embed: Option<bool>,
    pub miniplayer: Miniplayer,
    pub context_params: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamingData {
    pub expires_in_seconds: Option<String>,
    pub formats: Option<Vec<Format>>,
    pub adaptive_formats: Option<Vec<Format>>,
    pub dash_manifest_url: Option<String>,
    pub hls_manifest_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VideoDetails {
    pub video_id: Option<String>,
    pub title: Option<String>,
    pub length_seconds: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub channel_id: Option<String>,
    pub is_owner_viewing: Option<bool>,
    pub short_description: Option<String>,
    pub is_crawlable: Option<bool>,
    pub thumbnail: Option<Thumbnails>,
    pub average_rating: Option<f32>,
    pub allow_ratings: Option<bool>,
    pub view_count: Option<String>,
    pub author: Option<String>,
    pub is_private: Option<bool>,
    pub is_unplugged_corpus: Option<bool>,
    pub is_live_content: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SimpleText {
    pub simple_text: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerMicroformatRenderer {
    pub thumbnail: Option<Thumbnails>,
    pub title: Option<SimpleText>,
    pub description: Option<SimpleText>,
    pub length_seconds: Option<String>,
    pub owner_profile_url: Option<String>,
    pub external_channel_id: Option<String>,
    pub is_family_safe: Option<bool>,
    pub available_countries: Option<Vec<String>>,
    pub is_unlisted: Option<bool>,
    pub has_ypc_metadata: Option<bool>,
    pub view_count: Option<String>,
    pub category: Option<String>,
    pub publish_date: Option<String>,
    pub owner_channel_name: Option<String>,
    pub upload_date: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Microformat {
    pub player_microformat_renderer: Option<PlayerMicroformatRenderer>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerResponseData {
    pub playability_status: Option<PlayabilityStatus>,
    pub streaming_data: Option<StreamingData>,
    pub video_details: Option<VideoDetails>,
    pub microformat: Option<Microformat>,
}

#[derive(Debug, Clone)]
pub struct Video {
    pub id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub hls_manifest_url: Option<String>,
    pub dash_manifest_url: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
    pub duration_seconds: Option<i32>,
    pub formats: FormatList,
}

impl Video {
    pub fn from_player_response(data: PlayerResponseData) -> Self {
        let duration_seconds = data
            .microformat
            .and_then(|mf| mf.player_microformat_renderer)
            .and_then(|mfr| mfr.length_seconds)
            .and_then(|sec| sec.parse::<i32>().ok());
        // let publish_date =
        let mut formats: Vec<Format> = data
            .streaming_data
            .as_ref()
            .and_then(|sd| sd.formats.as_ref())
            .unwrap_or(&Vec::new())
            .to_vec();
        formats.extend(
            data.streaming_data
                .as_ref()
                .and_then(|sd| sd.adaptive_formats.as_ref())
                .unwrap_or(&Vec::new())
                .to_vec(),
        );
        let formats: FormatList = formats.into();
        Self {
            id: data
                .video_details
                .as_ref()
                .and_then(|vd| vd.video_id.clone()),
            title: data.video_details.as_ref().and_then(|vd| vd.title.clone()),
            description: data
                .video_details
                .as_ref()
                .and_then(|vd| vd.short_description.clone()),
            author: data.video_details.as_ref().and_then(|vd| vd.author.clone()),
            hls_manifest_url: data
                .streaming_data
                .as_ref()
                .and_then(|vd| vd.hls_manifest_url.clone()),
            dash_manifest_url: data
                .streaming_data
                .as_ref()
                .and_then(|vd| vd.dash_manifest_url.clone()),
            thumbnails: data
                .video_details
                .and_then(|vd| vd.thumbnail)
                .map(|t| t.thumbnails)
                .unwrap_or(Vec::new()),
            duration_seconds,
            formats,
        }
    }
}
