// enum Source {

//   SPOTIFY = 0;

//   SOUNDCLOUD = 1;

//   YOUTUBE = 2;

// }

// enum Sink {

//   YOUTUBE = 0;

//   SOUNDCLOUD = 1;

// }

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct UserId {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(enumeration = "Service", tag = "10")]
    pub source: i32,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct TrackId {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub playlist_id: ::core::option::Option<PlaylistId>,
    #[prost(enumeration = "Service", tag = "10")]
    pub source: i32,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct PlaylistId {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(enumeration = "Service", tag = "10")]
    pub source: i32,
}
/// test
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct PlaylistSyncRequest {}
// message SyncRequest {

// PlaylistSyncRequest playlist = 1;

// }

// message UserId {

//   oneof id {

//     SpotifyUserId spotify_user_id = 1;

//   }

/// }
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct SpotifyUserLoginCallbackPkce {
    #[prost(string, tag = "1")]
    pub code: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub state: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct SpotifyUserLoginCallback {
    #[prost(oneof = "spotify_user_login_callback::Method", tags = "1")]
    pub method: ::core::option::Option<spotify_user_login_callback::Method>,
}
/// Nested message and enum types in `SpotifyUserLoginCallback`.
pub mod spotify_user_login_callback {
    #[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Oneof)]
    pub enum Method {
        #[prost(message, tag = "1")]
        Pkce(super::SpotifyUserLoginCallbackPkce),
    }
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct UserLoginCallback {
    #[prost(oneof = "user_login_callback::Login", tags = "1")]
    pub login: ::core::option::Option<user_login_callback::Login>,
}
/// Nested message and enum types in `UserLoginCallback`.
pub mod user_login_callback {
    #[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Oneof)]
    pub enum Login {
        #[prost(message, tag = "1")]
        SpotifyLogin(super::SpotifyUserLoginCallback),
    }
}
// message UserId {

//   oneof id {

//     SpotifyUserId spotify_user_id = 1;

//   }

// }

// message DownloadedTrack {

//   Track track = 1;

//   string file_path = 2;

//   /1* uint64 content_length = 3; *1/

// }

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct Artwork {
    #[prost(string, tag = "1")]
    pub url: ::prost::alloc::string::String,
    #[prost(uint32, tag = "2")]
    pub width: u32,
    #[prost(uint32, tag = "3")]
    pub height: u32,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct TrackPreview {
    #[prost(string, tag = "1")]
    pub url: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct SpotifyTrack {}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct YoutubeTrack {}
/// Source source = 1;
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct Track {
    /// PlaylistId playlist_id = 1;
    #[prost(message, optional, tag = "1")]
    pub id: ::core::option::Option<TrackId>,
    #[prost(string, tag = "100")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "101")]
    pub artist: ::prost::alloc::string::String,
    #[prost(uint64, tag = "102")]
    pub duration_secs: u64,
    #[prost(message, optional, tag = "200")]
    pub artwork: ::core::option::Option<Artwork>,
    #[prost(message, optional, tag = "201")]
    pub preview: ::core::option::Option<TrackPreview>,
    // per source track options
    // e.g. youtube for better ranking
    #[prost(oneof = "track::Info", tags = "301, 302")]
    pub info: ::core::option::Option<track::Info>,
}
/// Nested message and enum types in `Track`.
pub mod track {
    // per source track options
    // e.g. youtube for better ranking

    #[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Oneof)]
    pub enum Info {
        #[prost(message, tag = "301")]
        SpotifyTrack(super::SpotifyTrack),
        #[prost(message, tag = "302")]
        YoutubeTrack(super::YoutubeTrack),
    }
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
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
// message SourcePlaylists {

//   Source source = 1;

//   repeated Playlist playlists = 100;

// }

// message Playlists { repeated Playlist playlists = 1; }

// message Library {

//   repeated SourcePlaylists sources = 1;

//   /1* map<string, Playlists> sources = 1; *1/

// }

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct TrackSyncDesc {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub source: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub sink: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct PlaylistSyncDesc {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub source: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub sink: ::prost::alloc::string::String,
}
/// todo oneof
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct SyncProgressUpdate {}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct SyncRequest {
    #[prost(string, repeated, tag = "1")]
    pub sources: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(message, repeated, tag = "2")]
    pub tracks: ::prost::alloc::vec::Vec<TrackSyncDesc>,
    /// oneof request {
    #[prost(message, repeated, tag = "3")]
    pub playlists: ::prost::alloc::vec::Vec<PlaylistSyncDesc>,
}
#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    ::prost::Enumeration,
)]
#[repr(i32)]
pub enum Service {
    Spotify = 0,
    Soundcloud = 1,
    Youtube = 2,
}
#[doc = r" Generated server implementations."]
pub mod dj_tool_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[doc = "Generated trait containing gRPC methods that should be implemented for use with DjToolServer."]
    #[async_trait]
    pub trait DjTool: Send + Sync + 'static {
        #[doc = "Server streaming response type for the Sync method."]
        type SyncStream: futures_core::Stream<Item = Result<super::SyncProgressUpdate, tonic::Status>>
            + Send
            + Sync
            + 'static;
        #[doc = " sync request"]
        async fn sync(
            &self,
            request: tonic::Request<super::SyncRequest>,
        ) -> Result<tonic::Response<Self::SyncStream>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct DjToolServer<T: DjTool> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: DjTool> DjToolServer<T> {
        pub fn new(inner: T) -> Self {
            let inner = Arc::new(inner);
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for DjToolServer<T>
    where
        T: DjTool,
        B: Body + Send + Sync + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = Never;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/proto.djtool.DjTool/Sync" => {
                    #[allow(non_camel_case_types)]
                    struct SyncSvc<T: DjTool>(pub Arc<T>);
                    impl<T: DjTool> tonic::server::ServerStreamingService<super::SyncRequest> for SyncSvc<T> {
                        type Response = super::SyncProgressUpdate;
                        type ResponseStream = T::SyncStream;
                        type Future =
                            BoxFuture<tonic::Response<Self::ResponseStream>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SyncRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).sync(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = SyncSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(empty_body())
                        .unwrap())
                }),
            }
        }
    }
    impl<T: DjTool> Clone for DjToolServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: DjTool> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: DjTool> tonic::transport::NamedService for DjToolServer<T> {
        const NAME: &'static str = "proto.djtool.DjTool";
    }
}
