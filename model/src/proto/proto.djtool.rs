#[derive(serde::Serialize, serde::Deserialize, Hash, Eq)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UserId {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(enumeration = "Service", tag = "10")]
    pub source: i32,
}
#[derive(serde::Serialize, serde::Deserialize, Hash, Eq)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TrackId {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub playlist_id: ::core::option::Option<PlaylistId>,
    #[prost(enumeration = "Service", tag = "10")]
    pub source: i32,
}
#[derive(serde::Serialize, serde::Deserialize, Hash, Eq)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PlaylistId {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(enumeration = "Service", tag = "10")]
    pub source: i32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SpotifyUserLoginCallbackPkce {
    #[prost(string, tag = "1")]
    pub code: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub state: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SpotifyUserLoginCallback {
    #[prost(oneof = "spotify_user_login_callback::Method", tags = "1")]
    pub method: ::core::option::Option<spotify_user_login_callback::Method>,
}
/// Nested message and enum types in `SpotifyUserLoginCallback`.
pub mod spotify_user_login_callback {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Method {
        #[prost(message, tag = "1")]
        Pkce(super::SpotifyUserLoginCallbackPkce),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UserLoginCallback {
    #[prost(oneof = "user_login_callback::Login", tags = "1")]
    pub login: ::core::option::Option<user_login_callback::Login>,
}
/// Nested message and enum types in `UserLoginCallback`.
pub mod user_login_callback {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Login {
        #[prost(message, tag = "1")]
        SpotifyLogin(super::SpotifyUserLoginCallback),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Artwork {
    #[prost(string, tag = "1")]
    pub url: ::prost::alloc::string::String,
    #[prost(uint32, tag = "2")]
    pub width: u32,
    #[prost(uint32, tag = "3")]
    pub height: u32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TrackPreview {
    #[prost(string, tag = "1")]
    pub url: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SpotifyTrack {}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct YoutubeTrack {}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Track {
    #[prost(message, optional, tag = "1")]
    pub id: ::core::option::Option<TrackId>,
    #[prost(string, tag = "100")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "101")]
    pub artist: ::prost::alloc::string::String,
    #[prost(uint64, tag = "102")]
    pub duration_millis: u64,
    #[prost(message, optional, tag = "200")]
    pub artwork: ::core::option::Option<Artwork>,
    #[prost(message, optional, tag = "201")]
    pub preview: ::core::option::Option<TrackPreview>,
    #[prost(oneof = "track::Info", tags = "301, 302")]
    pub info: ::core::option::Option<track::Info>,
}
/// Nested message and enum types in `Track`.
pub mod track {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Info {
        #[prost(message, tag = "301")]
        SpotifyTrack(super::SpotifyTrack),
        #[prost(message, tag = "302")]
        YoutubeTrack(super::YoutubeTrack),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Playlist {
    #[prost(message, optional, tag = "1")]
    pub id: ::core::option::Option<PlaylistId>,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
    #[prost(uint32, tag = "3")]
    pub total: u32,
    #[prost(message, repeated, tag = "100")]
    pub tracks: ::prost::alloc::vec::Vec<Track>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Service {
    Spotify = 0,
    Soundcloud = 1,
    Youtube = 2,
}
impl Service {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Service::Spotify => "SPOTIFY",
            Service::Soundcloud => "SOUNDCLOUD",
            Service::Youtube => "YOUTUBE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "SPOTIFY" => Some(Self::Spotify),
            "SOUNDCLOUD" => Some(Self::Soundcloud),
            "YOUTUBE" => Some(Self::Youtube),
            _ => None,
        }
    }
}
