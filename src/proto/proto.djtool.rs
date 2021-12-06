#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Empty {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConnectRequest {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DisconnectRequest {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Heartbeat {
    #[prost(uint64, tag = "1")]
    pub seq: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Update {
    #[prost(oneof = "update::Update", tags = "1")]
    pub update: ::core::option::Option<update::Update>,
}
/// Nested message and enum types in `Update`.
pub mod update {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Update {
        /// todo: add status messages or notifications
        ///
        /// Assignment assignment = 2;
        #[prost(message, tag = "1")]
        Heartbeat(super::Heartbeat),
    }
}
#[doc = r" Generated server implementations."]
pub mod djtool_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[doc = "Generated trait containing gRPC methods that should be implemented for use with DjtoolServer."]
    #[async_trait]
    pub trait Djtool: Send + Sync + 'static {
        #[doc = "Server streaming response type for the Connect method."]
        type ConnectStream: futures_core::Stream<Item = Result<super::Update, tonic::Status>>
            + Send
            + Sync
            + 'static;
        #[doc = " connect and disconnect"]
        async fn connect(
            &self,
            request: tonic::Request<super::ConnectRequest>,
        ) -> Result<tonic::Response<Self::ConnectStream>, tonic::Status>;
        async fn disconnect(
            &self,
            request: tonic::Request<super::DisconnectRequest>,
        ) -> Result<tonic::Response<super::Empty>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct DjtoolServer<T: Djtool> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Djtool> DjtoolServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>> for DjtoolServer<T>
    where
        T: Djtool,
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
                "/proto.djtool.Djtool/Connect" => {
                    #[allow(non_camel_case_types)]
                    struct ConnectSvc<T: Djtool>(pub Arc<T>);
                    impl<T: Djtool> tonic::server::ServerStreamingService<super::ConnectRequest> for ConnectSvc<T> {
                        type Response = super::Update;
                        type ResponseStream = T::ConnectStream;
                        type Future =
                            BoxFuture<tonic::Response<Self::ResponseStream>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ConnectRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).connect(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ConnectSvc(inner);
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
                "/proto.djtool.Djtool/Disconnect" => {
                    #[allow(non_camel_case_types)]
                    struct DisconnectSvc<T: Djtool>(pub Arc<T>);
                    impl<T: Djtool> tonic::server::UnaryService<super::DisconnectRequest> for DisconnectSvc<T> {
                        type Response = super::Empty;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DisconnectRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).disconnect(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DisconnectSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
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
    impl<T: Djtool> Clone for DjtoolServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: Djtool> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Djtool> tonic::transport::NamedService for DjtoolServer<T> {
        const NAME: &'static str = "proto.djtool.Djtool";
    }
}
