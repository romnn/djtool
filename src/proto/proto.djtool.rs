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
/// test
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PlaylistSyncRequest {}
/// }
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
/// Source source = 1;
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Track {
    /// PlaylistId playlist_id = 1;
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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TrackSyncDesc {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub source: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub sink: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PlaylistSyncDesc {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub source: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub sink: ::prost::alloc::string::String,
}
/// todo oneof
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SyncProgressUpdate {}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SyncRequest {
    #[prost(string, repeated, tag = "1")]
    pub sources: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(message, repeated, tag = "2")]
    pub tracks: ::prost::alloc::vec::Vec<TrackSyncDesc>,
    /// oneof request {
    #[prost(message, repeated, tag = "3")]
    pub playlists: ::prost::alloc::vec::Vec<PlaylistSyncDesc>,
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
/// Generated server implementations.
pub mod dj_tool_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with DjToolServer.
    #[async_trait]
    pub trait DjTool: Send + Sync + 'static {
        /// Server streaming response type for the Sync method.
        type SyncStream: futures_core::Stream<
                Item = std::result::Result<super::SyncProgressUpdate, tonic::Status>,
            >
            + Send
            + 'static;
        /// sync request
        async fn sync(
            &self,
            request: tonic::Request<super::SyncRequest>,
        ) -> std::result::Result<tonic::Response<Self::SyncStream>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct DjToolServer<T: DjTool> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: DjTool> DjToolServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for DjToolServer<T>
    where
        T: DjTool,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/proto.djtool.DjTool/Sync" => {
                    #[allow(non_camel_case_types)]
                    struct SyncSvc<T: DjTool>(pub Arc<T>);
                    impl<
                        T: DjTool,
                    > tonic::server::ServerStreamingService<super::SyncRequest>
                    for SyncSvc<T> {
                        type Response = super::SyncProgressUpdate;
                        type ResponseStream = T::SyncStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SyncRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move { (*inner).sync(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = SyncSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
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
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: DjTool> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: DjTool> tonic::server::NamedService for DjToolServer<T> {
        const NAME: &'static str = "proto.djtool.DjTool";
    }
}
