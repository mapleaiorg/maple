// @generated
// Generated from: proto/ibank/v1/ibank.proto
// Manual check-in for offline builds.

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HealthRequest {}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HealthReply {
    #[prost(string, tag = "1")]
    pub status: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub service: ::prost::alloc::string::String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HumanApprovalMessage {
    #[prost(bool, tag = "1")]
    pub approved: bool,
    #[prost(string, tag = "2")]
    pub approver_id: ::prost::alloc::string::String,
    #[prost(string, optional, tag = "3")]
    pub note: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(int64, tag = "4")]
    pub approved_at_unix_ms: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HandleRpcRequest {
    #[prost(string, optional, tag = "1")]
    pub trace_id: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(string, tag = "2")]
    pub origin_actor: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub counterparty_actor: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub transaction_type: ::prost::alloc::string::String,
    #[prost(uint64, tag = "5")]
    pub amount_minor: u64,
    #[prost(string, tag = "6")]
    pub currency: ::prost::alloc::string::String,
    #[prost(string, tag = "7")]
    pub rail: ::prost::alloc::string::String,
    #[prost(string, tag = "8")]
    pub destination: ::prost::alloc::string::String,
    #[prost(string, tag = "9")]
    pub jurisdiction: ::prost::alloc::string::String,
    #[prost(string, tag = "10")]
    pub user_intent: ::prost::alloc::string::String,
    #[prost(float, optional, tag = "11")]
    pub ambiguity_hint: ::core::option::Option<f32>,
    #[prost(uint32, tag = "12")]
    pub counterparty_risk: u32,
    #[prost(uint32, tag = "13")]
    pub anomaly_score: u32,
    #[prost(float, tag = "14")]
    pub model_uncertainty: f32,
    #[prost(string, repeated, tag = "15")]
    pub compliance_flags: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(map = "string, string", tag = "16")]
    pub metadata:
        ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    #[prost(message, optional, tag = "17")]
    pub approval: ::core::option::Option<HumanApprovalMessage>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MeaningFieldMessage {
    #[prost(string, tag = "1")]
    pub summary: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub inferred_action: ::prost::alloc::string::String,
    #[prost(string, repeated, tag = "3")]
    pub ambiguity_notes: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(float, tag = "4")]
    pub ambiguity_score: f32,
    #[prost(float, tag = "5")]
    pub confidence: f32,
    #[prost(int64, tag = "6")]
    pub formed_at_unix_ms: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConfidenceProfileMessage {
    #[prost(float, tag = "1")]
    pub meaning_confidence: f32,
    #[prost(float, tag = "2")]
    pub model_confidence: f32,
    #[prost(float, tag = "3")]
    pub overall_confidence: f32,
    #[prost(bool, tag = "4")]
    pub blocking_ambiguity: bool,
    #[prost(string, repeated, tag = "5")]
    pub notes: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IntentRecordMessage {
    #[prost(string, tag = "1")]
    pub objective: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub rationale: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "3")]
    pub confidence: ::core::option::Option<ConfidenceProfileMessage>,
    #[prost(int64, tag = "4")]
    pub stabilized_at_unix_ms: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RiskBreakdownMessage {
    #[prost(uint32, tag = "1")]
    pub amount: u32,
    #[prost(uint32, tag = "2")]
    pub counterparty: u32,
    #[prost(uint32, tag = "3")]
    pub jurisdiction: u32,
    #[prost(uint32, tag = "4")]
    pub anomaly: u32,
    #[prost(uint32, tag = "5")]
    pub model_uncertainty: u32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RiskReportMessage {
    #[prost(uint32, tag = "1")]
    pub score: u32,
    #[prost(string, repeated, tag = "2")]
    pub reasons: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(message, optional, tag = "3")]
    pub factors: ::core::option::Option<RiskBreakdownMessage>,
    #[prost(uint32, tag = "4")]
    pub fraud_score: u32,
    #[prost(bool, tag = "5")]
    pub blocking_ambiguity: bool,
    #[prost(bool, tag = "6")]
    pub requires_hybrid: bool,
    #[prost(bool, tag = "7")]
    pub denied: bool,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RouteResultMessage {
    #[prost(string, tag = "1")]
    pub connector: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub external_reference: ::prost::alloc::string::String,
    #[prost(int64, tag = "3")]
    pub settled_at_unix_ms: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ::prost::Enumeration)]
#[repr(i32)]
pub enum HandleStatusProto {
    Unspecified = 0,
    ExecutedAutonomous = 1,
    ExecutedHybrid = 2,
    PendingHumanApproval = 3,
    Denied = 4,
    Failed = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ::prost::Enumeration)]
#[repr(i32)]
pub enum ExecutionModeProto {
    Unspecified = 0,
    PureAi = 1,
    Hybrid = 2,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HandleRpcResponse {
    #[prost(string, tag = "1")]
    pub trace_id: ::prost::alloc::string::String,
    #[prost(string, optional, tag = "2")]
    pub commitment_id: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(enumeration = "HandleStatusProto", tag = "3")]
    pub status: i32,
    #[prost(enumeration = "ExecutionModeProto", optional, tag = "4")]
    pub mode: ::core::option::Option<i32>,
    #[prost(string, tag = "5")]
    pub decision_reason: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "6")]
    pub meaning: ::core::option::Option<MeaningFieldMessage>,
    #[prost(message, optional, tag = "7")]
    pub intent: ::core::option::Option<IntentRecordMessage>,
    #[prost(message, optional, tag = "8")]
    pub risk_report: ::core::option::Option<RiskReportMessage>,
    #[prost(message, optional, tag = "9")]
    pub route: ::core::option::Option<RouteResultMessage>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PendingApprovalMessage {
    #[prost(string, tag = "1")]
    pub trace_id: ::prost::alloc::string::String,
    #[prost(string, optional, tag = "2")]
    pub commitment_id: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(string, tag = "3")]
    pub decision_reason: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "4")]
    pub risk_report: ::core::option::Option<RiskReportMessage>,
    #[prost(message, optional, tag = "5")]
    pub request: ::core::option::Option<HandleRpcRequest>,
    #[prost(int64, tag = "6")]
    pub queued_at_unix_ms: i64,
    #[prost(int64, tag = "7")]
    pub updated_at_unix_ms: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListPendingRequest {}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListPendingResponse {
    #[prost(message, repeated, tag = "1")]
    pub items: ::prost::alloc::vec::Vec<PendingApprovalMessage>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ApprovePendingRequest {
    #[prost(string, tag = "1")]
    pub trace_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub approver_id: ::prost::alloc::string::String,
    #[prost(string, optional, tag = "3")]
    pub note: ::core::option::Option<::prost::alloc::string::String>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RejectPendingRequest {
    #[prost(string, tag = "1")]
    pub trace_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub approver_id: ::prost::alloc::string::String,
    #[prost(string, optional, tag = "3")]
    pub note: ::core::option::Option<::prost::alloc::string::String>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RejectPendingResponse {
    #[prost(string, tag = "1")]
    pub trace_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub status: ::prost::alloc::string::String,
}

pub mod i_bank_service_client {
    #![allow(clippy::derive_partial_eq_without_eq)]
    use tonic::codegen::*;

    #[derive(Debug, Clone)]
    pub struct IBankServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }

    impl IBankServiceClient<tonic::transport::Channel> {
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }

    impl<T> IBankServiceClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::ResponseBody: Body + Send + 'static,
        T::Error: Into<StdError>,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
        <T::ResponseBody as Body>::Data: Into<Bytes> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }

        pub async fn health(
            &mut self,
            request: impl tonic::IntoRequest<super::HealthRequest>,
        ) -> Result<tonic::Response<super::HealthReply>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = tonic::codegen::http::uri::PathAndQuery::from_static(
                "/ibank.v1.IBankService/Health",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }

        pub async fn handle(
            &mut self,
            request: impl tonic::IntoRequest<super::HandleRpcRequest>,
        ) -> Result<tonic::Response<super::HandleRpcResponse>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = tonic::codegen::http::uri::PathAndQuery::from_static(
                "/ibank.v1.IBankService/Handle",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }

        pub async fn list_pending(
            &mut self,
            request: impl tonic::IntoRequest<super::ListPendingRequest>,
        ) -> Result<tonic::Response<super::ListPendingResponse>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = tonic::codegen::http::uri::PathAndQuery::from_static(
                "/ibank.v1.IBankService/ListPending",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }

        pub async fn approve_pending(
            &mut self,
            request: impl tonic::IntoRequest<super::ApprovePendingRequest>,
        ) -> Result<tonic::Response<super::HandleRpcResponse>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = tonic::codegen::http::uri::PathAndQuery::from_static(
                "/ibank.v1.IBankService/ApprovePending",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }

        pub async fn reject_pending(
            &mut self,
            request: impl tonic::IntoRequest<super::RejectPendingRequest>,
        ) -> Result<tonic::Response<super::RejectPendingResponse>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = tonic::codegen::http::uri::PathAndQuery::from_static(
                "/ibank.v1.IBankService/RejectPending",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}

pub mod i_bank_service_server {
    #![allow(clippy::derive_partial_eq_without_eq)]
    use tonic::codegen::*;

    #[tonic::async_trait]
    pub trait IBankService: Send + Sync + 'static {
        async fn health(
            &self,
            request: tonic::Request<super::HealthRequest>,
        ) -> Result<tonic::Response<super::HealthReply>, tonic::Status>;
        async fn handle(
            &self,
            request: tonic::Request<super::HandleRpcRequest>,
        ) -> Result<tonic::Response<super::HandleRpcResponse>, tonic::Status>;
        async fn list_pending(
            &self,
            request: tonic::Request<super::ListPendingRequest>,
        ) -> Result<tonic::Response<super::ListPendingResponse>, tonic::Status>;
        async fn approve_pending(
            &self,
            request: tonic::Request<super::ApprovePendingRequest>,
        ) -> Result<tonic::Response<super::HandleRpcResponse>, tonic::Status>;
        async fn reject_pending(
            &self,
            request: tonic::Request<super::RejectPendingRequest>,
        ) -> Result<tonic::Response<super::RejectPendingResponse>, tonic::Status>;
    }

    #[derive(Debug, Clone)]
    pub struct IBankServiceServer<T: IBankService> {
        inner: Arc<T>,
    }

    impl<T: IBankService> IBankServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self {
                inner: Arc::new(inner),
            }
        }
    }

    impl<T: IBankService> Service<http::Request<tonic::body::BoxBody>> for IBankServiceServer<T> {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: http::Request<tonic::body::BoxBody>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/ibank.v1.IBankService/Health" => {
                    struct HealthSvc<T: IBankService>(pub Arc<T>);
                    impl<T: IBankService> tonic::server::UnaryService<super::HealthRequest> for HealthSvc<T> {
                        type Response = super::HealthReply;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::HealthRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            Box::pin(async move { inner.health(request).await })
                        }
                    }
                    Box::pin(async move {
                        let method = HealthSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec);
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    })
                }
                "/ibank.v1.IBankService/Handle" => {
                    struct HandleSvc<T: IBankService>(pub Arc<T>);
                    impl<T: IBankService> tonic::server::UnaryService<super::HandleRpcRequest> for HandleSvc<T> {
                        type Response = super::HandleRpcResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::HandleRpcRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            Box::pin(async move { inner.handle(request).await })
                        }
                    }
                    Box::pin(async move {
                        let method = HandleSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec);
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    })
                }
                "/ibank.v1.IBankService/ListPending" => {
                    struct ListPendingSvc<T: IBankService>(pub Arc<T>);
                    impl<T: IBankService> tonic::server::UnaryService<super::ListPendingRequest> for ListPendingSvc<T> {
                        type Response = super::ListPendingResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListPendingRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            Box::pin(async move { inner.list_pending(request).await })
                        }
                    }
                    Box::pin(async move {
                        let method = ListPendingSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec);
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    })
                }
                "/ibank.v1.IBankService/ApprovePending" => {
                    struct ApprovePendingSvc<T: IBankService>(pub Arc<T>);
                    impl<T: IBankService> tonic::server::UnaryService<super::ApprovePendingRequest>
                        for ApprovePendingSvc<T>
                    {
                        type Response = super::HandleRpcResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ApprovePendingRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            Box::pin(async move { inner.approve_pending(request).await })
                        }
                    }
                    Box::pin(async move {
                        let method = ApprovePendingSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec);
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    })
                }
                "/ibank.v1.IBankService/RejectPending" => {
                    struct RejectPendingSvc<T: IBankService>(pub Arc<T>);
                    impl<T: IBankService> tonic::server::UnaryService<super::RejectPendingRequest>
                        for RejectPendingSvc<T>
                    {
                        type Response = super::RejectPendingResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::RejectPendingRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            Box::pin(async move { inner.reject_pending(request).await })
                        }
                    }
                    Box::pin(async move {
                        let method = RejectPendingSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec);
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    })
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(tonic::body::empty_body())
                        .unwrap())
                }),
            }
        }
    }

    impl<T: IBankService> tonic::server::NamedService for IBankServiceServer<T> {
        const NAME: &'static str = "ibank.v1.IBankService";
    }
}
