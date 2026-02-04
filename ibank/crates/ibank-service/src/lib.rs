#![deny(unsafe_code)]

pub mod grpc;
pub mod pb;
pub mod queue;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use ibank_adapters::{
    CryptoWalletAggregationConnector, MockAchConnector, MockChainConnector,
    OpenBankingAggregationConnector,
};
use ibank_core::{
    AssetPair, HandleRequest, HandleResponse, HandleStatus, HumanApproval, IBankEngine,
    IBankEngineConfig, LedgerEntry, LedgerEntryKind, LedgerStorageConfig, RiskPolicyConfig,
    UnifiedLedgerView,
};
use queue::{PendingApproval, PersistedApprovalQueue, QueueError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub queue_path: PathBuf,
    pub ledger_storage: LedgerStorageConfig,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            queue_path: PathBuf::from("ibank/data/approvals.json"),
            ledger_storage: LedgerStorageConfig::Memory,
        }
    }
}

#[derive(Clone)]
pub struct ServiceState {
    pub engine: Arc<IBankEngine>,
    pub queue: Arc<Mutex<PersistedApprovalQueue>>,
}

impl ServiceState {
    pub async fn bootstrap(config: ServiceConfig) -> Result<Self, ServiceError> {
        let ServiceConfig {
            queue_path,
            ledger_storage,
        } = config;
        let mut engine_config = IBankEngineConfig::default();
        engine_config.ledger_storage = ledger_storage;
        let engine = IBankEngine::bootstrap(RiskPolicyConfig::default(), engine_config)
            .await
            .map_err(ServiceError::Core)?;
        engine
            .register_connector(Arc::new(MockAchConnector))
            .map_err(ServiceError::Core)?;
        engine
            .register_connector(Arc::new(MockChainConnector))
            .map_err(ServiceError::Core)?;
        engine
            .register_aggregation_connector(Arc::new(OpenBankingAggregationConnector))
            .await
            .map_err(ServiceError::Core)?;
        engine
            .register_aggregation_connector(Arc::new(CryptoWalletAggregationConnector))
            .await
            .map_err(ServiceError::Core)?;

        let queue = PersistedApprovalQueue::load(queue_path)?;

        Ok(Self {
            engine: Arc::new(engine),
            queue: Arc::new(Mutex::new(queue)),
        })
    }
}

pub fn build_router(state: ServiceState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/handle", post(handle))
        .route("/v1/ledger/entries", get(list_ledger_entries))
        .route("/v1/ledger/snapshot/latest", get(get_latest_snapshot))
        .route("/v1/approvals/pending", get(list_pending))
        .route("/v1/approvals/:trace_id/approve", post(approve_pending))
        .route("/v1/approvals/:trace_id/reject", post(reject_pending))
        .with_state(state)
}

pub async fn handle_with_queue(
    state: &ServiceState,
    request: HandleRequest,
) -> Result<HandleResponse, ApiError> {
    let response = state.engine.handle(request.clone()).await;

    let mut queue = state.queue.lock().await;
    if response.status == HandleStatus::PendingHumanApproval {
        queue.upsert_from_response(request, &response)?;
    } else {
        let _ = queue.remove(&response.trace_id)?;
    }

    Ok(response)
}

pub async fn list_pending_items(state: &ServiceState) -> Result<Vec<PendingApproval>, ApiError> {
    let queue = state.queue.lock().await;
    Ok(queue.list())
}

pub async fn approve_pending_trace(
    state: &ServiceState,
    trace_id: &str,
    approver_id: String,
    note: Option<String>,
) -> Result<HandleResponse, ApiError> {
    let pending = {
        let queue = state.queue.lock().await;
        queue.get(trace_id).cloned()
    }
    .ok_or_else(|| ApiError::not_found(format!("pending approval '{}' not found", trace_id)))?;

    let mut request = pending.request;
    request.approval = Some(HumanApproval {
        approved: true,
        approver_id,
        note,
        approved_at: Utc::now(),
    });

    let response = state.engine.handle(request.clone()).await;

    let mut queue = state.queue.lock().await;
    if response.status == HandleStatus::PendingHumanApproval {
        queue.upsert_from_response(request, &response)?;
    } else {
        let _ = queue.remove(trace_id)?;
    }

    Ok(response)
}

#[derive(Debug, Clone, Serialize)]
pub struct RejectOutcome {
    pub trace_id: String,
    pub status: &'static str,
}

pub async fn reject_pending_trace(
    state: &ServiceState,
    trace_id: &str,
    approver_id: String,
    note: Option<String>,
) -> Result<RejectOutcome, ApiError> {
    let pending = {
        let queue = state.queue.lock().await;
        queue.get(trace_id).cloned()
    }
    .ok_or_else(|| ApiError::not_found(format!("pending approval '{}' not found", trace_id)))?;

    state
        .engine
        .record_hybrid_rejection(
            trace_id,
            pending.commitment_id,
            &approver_id,
            note.as_deref(),
        )
        .await?;

    let mut queue = state.queue.lock().await;
    let _ = queue.remove(trace_id)?;

    Ok(RejectOutcome {
        trace_id: trace_id.to_string(),
        status: "rejected",
    })
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("core engine error: {0}")]
    Core(#[from] ibank_core::IBankError),
    #[error("queue error: {0}")]
    Queue(#[from] QueueError),
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{message}")]
    Http { status: StatusCode, message: String },
    #[error(transparent)]
    Core(#[from] ibank_core::IBankError),
    #[error(transparent)]
    Queue(#[from] QueueError),
}

impl ApiError {
    fn not_found(message: impl Into<String>) -> Self {
        Self::Http {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::Http {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Http { status, message } => {
                (status, Json(serde_json::json!({ "error": message }))).into_response()
            }
            ApiError::Core(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
                .into_response(),
            ApiError::Queue(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
                .into_response(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    ledger_backend: String,
}

async fn health(State(state): State<ServiceState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "ibank-service",
        ledger_backend: state.engine.ledger_backend().await,
    })
}

async fn handle(
    State(state): State<ServiceState>,
    Json(request): Json<HandleRequest>,
) -> Result<Json<HandleResponse>, ApiError> {
    Ok(Json(handle_with_queue(&state, request).await?))
}

#[derive(Debug, Clone, Deserialize)]
struct LedgerEntriesQuery {
    trace_id: Option<String>,
    commitment_id: Option<String>,
    kind: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    order: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct LedgerEntriesResponse {
    backend: String,
    total: usize,
    returned: usize,
    items: Vec<LedgerEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct LatestSnapshotQuery {
    user_id: String,
    refresh: Option<bool>,
    base: Option<String>,
    quote: Option<String>,
    amount_minor: Option<u64>,
    window_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
struct LatestSnapshotResponse {
    source: String,
    user_id: String,
    source_trace_id: Option<String>,
    captured_at: chrono::DateTime<Utc>,
    snapshot_hash: String,
    snapshot: UnifiedLedgerView,
}

fn parse_kind_filter(kind: Option<&str>) -> Result<Option<LedgerEntryKind>, ApiError> {
    match kind.map(|value| value.to_ascii_lowercase()) {
        None => Ok(None),
        Some(value) if value == "commitment" => Ok(Some(LedgerEntryKind::Commitment)),
        Some(value) if value == "audit" => Ok(Some(LedgerEntryKind::Audit)),
        Some(value) if value == "outcome" => Ok(Some(LedgerEntryKind::Outcome)),
        Some(other) => Err(ApiError::bad_request(format!(
            "invalid kind '{}'; expected one of: commitment, audit, outcome",
            other
        ))),
    }
}

async fn list_ledger_entries(
    State(state): State<ServiceState>,
    Query(query): Query<LedgerEntriesQuery>,
) -> Result<Json<LedgerEntriesResponse>, ApiError> {
    let kind_filter = parse_kind_filter(query.kind.as_deref())?;

    let mut entries = state.engine.ledger_entries().await?;

    if let Some(trace_id) = query.trace_id.as_deref() {
        entries.retain(|entry| entry.trace_id == trace_id);
    }

    if let Some(commitment_id) = query.commitment_id.as_deref() {
        entries.retain(|entry| entry.commitment_id.as_deref() == Some(commitment_id));
    }

    if let Some(kind) = kind_filter {
        entries.retain(|entry| entry.kind == kind);
    }

    let order = query
        .order
        .as_deref()
        .unwrap_or("desc")
        .to_ascii_lowercase();
    if order == "desc" {
        entries.reverse();
    } else if order != "asc" {
        return Err(ApiError::bad_request(format!(
            "invalid order '{}'; expected asc or desc",
            order
        )));
    }

    let total = entries.len();
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(100).min(1000);
    let items = entries
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let returned = items.len();

    Ok(Json(LedgerEntriesResponse {
        backend: state.engine.ledger_backend().await,
        total,
        returned,
        items,
    }))
}

async fn get_latest_snapshot(
    State(state): State<ServiceState>,
    Query(query): Query<LatestSnapshotQuery>,
) -> Result<Json<LatestSnapshotResponse>, ApiError> {
    if query.user_id.trim().is_empty() {
        return Err(ApiError::bad_request("user_id is required"));
    }

    let refresh = query.refresh.unwrap_or(false);
    if refresh {
        let base = query.base.unwrap_or_else(|| "USD".to_string());
        let quote = query.quote.unwrap_or_else(|| base.clone());
        let amount_minor = query.amount_minor.unwrap_or(100_000);
        let window_days = query.window_days.unwrap_or(30);
        let snapshot = state
            .engine
            .refresh_unified_snapshot(
                &query.user_id,
                AssetPair::new(base, quote),
                amount_minor,
                window_days,
            )
            .await?;

        return Ok(Json(LatestSnapshotResponse {
            source: "live_refresh".to_string(),
            user_id: snapshot.user_id,
            source_trace_id: snapshot.source_trace_id,
            captured_at: snapshot.captured_at,
            snapshot_hash: snapshot.view.snapshot_hash.clone(),
            snapshot: snapshot.view,
        }));
    }

    let cached = state
        .engine
        .latest_unified_snapshot(&query.user_id)
        .await
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "no cached snapshot for user '{}'; retry with refresh=true",
                query.user_id
            ))
        })?;

    Ok(Json(LatestSnapshotResponse {
        source: "cache".to_string(),
        user_id: cached.user_id,
        source_trace_id: cached.source_trace_id,
        captured_at: cached.captured_at,
        snapshot_hash: cached.view.snapshot_hash.clone(),
        snapshot: cached.view,
    }))
}

#[derive(Debug, Clone, Serialize)]
struct PendingListResponse {
    items: Vec<PendingApproval>,
}

async fn list_pending(
    State(state): State<ServiceState>,
) -> Result<Json<PendingListResponse>, ApiError> {
    Ok(Json(PendingListResponse {
        items: list_pending_items(&state).await?,
    }))
}

#[derive(Debug, Clone, Deserialize)]
struct ApprovalRequest {
    approver_id: String,
    note: Option<String>,
}

async fn approve_pending(
    Path(trace_id): Path<String>,
    State(state): State<ServiceState>,
    Json(approval): Json<ApprovalRequest>,
) -> Result<Json<HandleResponse>, ApiError> {
    Ok(Json(
        approve_pending_trace(&state, &trace_id, approval.approver_id, approval.note).await?,
    ))
}

async fn reject_pending(
    Path(trace_id): Path<String>,
    State(state): State<ServiceState>,
    Json(rejection): Json<ApprovalRequest>,
) -> Result<Json<RejectOutcome>, ApiError> {
    Ok(Json(
        reject_pending_trace(&state, &trace_id, rejection.approver_id, rejection.note).await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;
    use uuid::Uuid;

    #[tokio::test]
    async fn pending_approval_is_persisted_by_handle_endpoint() {
        let path = std::env::temp_dir()
            .join(format!("ibank-service-{}", Uuid::new_v4()))
            .join("approvals.json");
        let state = ServiceState::bootstrap(ServiceConfig {
            queue_path: path.clone(),
            ledger_storage: LedgerStorageConfig::Memory,
        })
        .await
        .unwrap();
        let app = build_router(state);

        let payload = serde_json::json!({
            "origin_actor": "issuer-a",
            "counterparty_actor": "merchant-b",
            "transaction_type": "transfer",
            "amount_minor": 1_500_000,
            "currency": "USD",
            "rail": "ach",
            "destination": "acct-123",
            "jurisdiction": "US",
            "user_intent": "move treasury funds",
            "ambiguity_hint": 0.1,
            "counterparty_risk": 10,
            "anomaly_score": 10,
            "model_uncertainty": 0.1,
            "compliance_flags": [],
            "metadata": {},
            "approval": null
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/handle")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: HandleResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.status, HandleStatus::PendingHumanApproval);

        let persisted = PersistedApprovalQueue::load(path).unwrap();
        assert_eq!(persisted.list().len(), 1);
    }

    #[tokio::test]
    async fn ledger_entries_endpoint_supports_kind_filter() {
        let path = std::env::temp_dir()
            .join(format!("ibank-ledger-{}", Uuid::new_v4()))
            .join("approvals.json");
        let state = ServiceState::bootstrap(ServiceConfig {
            queue_path: path.clone(),
            ledger_storage: LedgerStorageConfig::Memory,
        })
        .await
        .unwrap();
        let app = build_router(state);

        let payload = serde_json::json!({
            "origin_actor": "issuer-a",
            "counterparty_actor": "merchant-b",
            "transaction_type": "transfer",
            "amount_minor": 50000,
            "currency": "USD",
            "rail": "ach",
            "destination": "acct-123",
            "jurisdiction": "US",
            "user_intent": "pay invoice 889",
            "ambiguity_hint": 0.1,
            "counterparty_risk": 10,
            "anomaly_score": 8,
            "model_uncertainty": 0.08,
            "compliance_flags": [],
            "metadata": {},
            "approval": null
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/handle")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/ledger/entries?kind=commitment&order=asc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let items = value
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        assert!(!items.is_empty());
        assert!(items.iter().all(|item| {
            item.get("kind")
                .and_then(|k| k.as_str())
                .map(|k| k == "commitment")
                .unwrap_or(false)
        }));
    }

    #[tokio::test]
    async fn ledger_entries_endpoint_rejects_invalid_kind_filter() {
        let path = std::env::temp_dir()
            .join(format!("ibank-ledger-invalid-{}", Uuid::new_v4()))
            .join("approvals.json");
        let state = ServiceState::bootstrap(ServiceConfig {
            queue_path: path,
            ledger_storage: LedgerStorageConfig::Memory,
        })
        .await
        .unwrap();
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/ledger/entries?kind=bad-kind")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn latest_snapshot_endpoint_returns_cached_snapshot_after_handle() {
        let path = std::env::temp_dir()
            .join(format!("ibank-snapshot-cache-{}", Uuid::new_v4()))
            .join("approvals.json");
        let state = ServiceState::bootstrap(ServiceConfig {
            queue_path: path,
            ledger_storage: LedgerStorageConfig::Memory,
        })
        .await
        .unwrap();
        let app = build_router(state);

        let payload = serde_json::json!({
            "origin_actor": "issuer-a",
            "counterparty_actor": "merchant-b",
            "transaction_type": "transfer",
            "amount_minor": 50000,
            "currency": "USD",
            "rail": "ach",
            "destination": "acct-123",
            "jurisdiction": "US",
            "user_intent": "pay invoice 889",
            "ambiguity_hint": 0.1,
            "counterparty_risk": 10,
            "anomaly_score": 8,
            "model_uncertainty": 0.08,
            "compliance_flags": [],
            "metadata": {},
            "approval": null
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/handle")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/ledger/snapshot/latest?user_id=issuer-a")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.get("source").and_then(|v| v.as_str()), Some("cache"));
        assert!(body
            .get("snapshot_hash")
            .and_then(|v| v.as_str())
            .map(|v| !v.is_empty())
            .unwrap_or(false));
        assert_eq!(
            body.get("snapshot")
                .and_then(|v| v.get("user_id"))
                .and_then(|v| v.as_str()),
            Some("issuer-a")
        );
    }

    #[tokio::test]
    async fn latest_snapshot_endpoint_can_refresh_when_cache_missing() {
        let path = std::env::temp_dir()
            .join(format!("ibank-snapshot-refresh-{}", Uuid::new_v4()))
            .join("approvals.json");
        let state = ServiceState::bootstrap(ServiceConfig {
            queue_path: path,
            ledger_storage: LedgerStorageConfig::Memory,
        })
        .await
        .unwrap();
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/ledger/snapshot/latest?user_id=ops-user&refresh=true&base=USD&quote=USD&amount_minor=50000&window_days=7")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body.get("source").and_then(|v| v.as_str()),
            Some("live_refresh")
        );
        assert!(body
            .get("snapshot")
            .and_then(|v| v.get("connector_attestations"))
            .and_then(|v| v.as_array())
            .map(|items| items.len() >= 2)
            .unwrap_or(false));
    }
}
