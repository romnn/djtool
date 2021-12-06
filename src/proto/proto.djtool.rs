#[derive(Serialize, Deserialize, Hash, Eq, Clone, PartialEq, ::prost::Message)]
pub struct Track {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Serialize, Deserialize, Hash, Eq, Clone, PartialEq, ::prost::Message)]
pub struct Playlist {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "2")]
    pub tracks: ::prost::alloc::vec::Vec<Track>,
}
#[derive(Serialize, Deserialize, Hash, Eq, Clone, PartialEq, ::prost::Message)]
pub struct SourcePlaylists {
    #[prost(enumeration = "Source", tag = "1")]
    pub source: i32,
    #[prost(message, repeated, tag = "2")]
    pub playlists: ::prost::alloc::vec::Vec<Playlist>,
}
#[derive(Serialize, Deserialize, Hash, Eq, Clone, PartialEq, ::prost::Message)]
pub struct Library {
    #[prost(message, repeated, tag = "1")]
    pub sources: ::prost::alloc::vec::Vec<SourcePlaylists>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TrackSyncDesc {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(enumeration = "Source", tag = "2")]
    pub source: i32,
    #[prost(enumeration = "Sink", tag = "3")]
    pub sink: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PlaylistSyncDesc {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(enumeration = "Source", tag = "2")]
    pub source: i32,
    #[prost(enumeration = "Sink", tag = "3")]
    pub sink: i32,
}
/// todo oneof
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SyncProgressUpdate {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SyncRequest {
    #[prost(enumeration = "Source", repeated, tag = "1")]
    pub sources: ::prost::alloc::vec::Vec<i32>,
    #[prost(message, repeated, tag = "2")]
    pub tracks: ::prost::alloc::vec::Vec<TrackSyncDesc>,
    /// oneof request {
    #[prost(message, repeated, tag = "3")]
    pub playlists: ::prost::alloc::vec::Vec<PlaylistSyncDesc>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Source {
    Spotify = 0,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Sink {
    Youtube = 0,
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
