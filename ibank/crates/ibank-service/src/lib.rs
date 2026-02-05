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
    CryptoWalletAggregationConnector, MockAchConnector, MockChainConnector, MockEvmBridgeAdapter,
    MockRailBridgeAdapter, OpenBankingAggregationConnector,
};
use ibank_core::{
    AssetPair, AttestationConstraint, AttestationDecision, BridgeExecutionRequest, BridgeLeg,
    CommitmentRecord, ComplianceProof, EscalationCase, EscalationWorkflowState, HandleRequest,
    HandleResponse, HandleStatus, HumanApproval, HumanAttestation, IBankEngine, IBankEngineConfig,
    LedgerEntry, LedgerEntryKind, LedgerStorageConfig, RiskPolicyConfig, RiskReport,
    UnifiedBridgeReceipt, UnifiedLedgerView,
};
use queue::{PendingApproval, PersistedApprovalQueue, QueueError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use uuid::Uuid;

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
            .register_chain_adapter(Arc::new(MockEvmBridgeAdapter))
            .await
            .map_err(ServiceError::Core)?;
        engine
            .register_rail_adapter(Arc::new(MockRailBridgeAdapter))
            .await
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
        .route("/v1/bridge/execute", post(execute_bridge))
        .route("/v1/bridge/receipts", get(list_bridge_receipts))
        .route("/v1/compliance/trace/:trace_id", get(get_compliance_trace))
        .route("/v1/ledger/entries", get(list_ledger_entries))
        .route("/v1/ledger/snapshot/latest", get(get_latest_snapshot))
        .route("/v1/approvals/pending", get(list_pending))
        .route("/v1/approvals/case/:trace_id", get(get_approval_case))
        .route("/v1/approvals/:trace_id/approve", post(approve_pending))
        .route("/v1/approvals/:trace_id/reject", post(reject_pending))
        .with_state(state)
}

pub async fn handle_with_queue(
    state: &ServiceState,
    mut request: HandleRequest,
) -> Result<HandleResponse, ApiError> {
    // Never allow direct approval bypass through the public handle endpoint.
    // Hybrid execution must resume only via explicit human attestation workflow.
    request.approval = None;
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
    let attestation = build_human_attestation(
        trace_id,
        approver_id,
        AttestationDecision::Approve,
        note,
        Vec::new(),
        None,
        None,
    );
    approve_pending_with_attestation(state, trace_id, attestation).await
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
    let attestation = build_human_attestation(
        trace_id,
        approver_id,
        AttestationDecision::Deny,
        note,
        Vec::new(),
        None,
        None,
    );
    reject_pending_with_attestation(state, trace_id, attestation).await?;

    Ok(RejectOutcome {
        trace_id: trace_id.to_string(),
        status: "rejected",
    })
}

async fn approve_pending_with_attestation(
    state: &ServiceState,
    trace_id: &str,
    attestation: HumanAttestation,
) -> Result<HandleResponse, ApiError> {
    let pending = {
        let queue = state.queue.lock().await;
        queue.get(trace_id).cloned()
    }
    .ok_or_else(|| ApiError::not_found(format!("pending approval '{}' not found", trace_id)))?;

    {
        let mut queue = state.queue.lock().await;
        queue.begin_review(trace_id)?;
        let _ = queue.record_attestation(trace_id, attestation.clone())?;
    }

    state
        .engine
        .record_human_attestation(trace_id, pending.commitment_id.clone(), &attestation)
        .await?;

    let mut request = pending.request;
    apply_attestation_constraints(&mut request, &attestation.constraints);
    request.approval = Some(HumanApproval {
        approved: true,
        approver_id: attestation.signer_id.clone(),
        note: attestation.note.clone(),
        approved_at: attestation.attested_at,
    });

    let response = state.engine.handle(request.clone()).await;

    let mut queue = state.queue.lock().await;
    match response.status {
        HandleStatus::PendingHumanApproval => {
            queue.upsert_from_response(request, &response)?;
        }
        HandleStatus::ExecutedAutonomous | HandleStatus::ExecutedHybrid => {
            queue.mark_executed(trace_id)?;
            let _ = queue.close_case(trace_id)?;
        }
        HandleStatus::Denied | HandleStatus::Failed => {
            let _ = queue.close_case(trace_id)?;
        }
    }

    Ok(response)
}

async fn reject_pending_with_attestation(
    state: &ServiceState,
    trace_id: &str,
    attestation: HumanAttestation,
) -> Result<(), ApiError> {
    let pending = {
        let queue = state.queue.lock().await;
        queue.get(trace_id).cloned()
    }
    .ok_or_else(|| ApiError::not_found(format!("pending approval '{}' not found", trace_id)))?;

    {
        let mut queue = state.queue.lock().await;
        queue.begin_review(trace_id)?;
        let _ = queue.record_attestation(trace_id, attestation.clone())?;
    }

    state
        .engine
        .record_human_attestation(trace_id, pending.commitment_id.clone(), &attestation)
        .await?;

    state
        .engine
        .record_hybrid_rejection(
            trace_id,
            pending.commitment_id,
            &attestation.signer_id,
            attestation.note.as_deref(),
        )
        .await?;

    let mut queue = state.queue.lock().await;
    let _ = queue.close_case(trace_id)?;
    Ok(())
}

fn apply_attestation_constraints(
    request: &mut HandleRequest,
    constraints: &[AttestationConstraint],
) {
    for constraint in constraints {
        match constraint.key.as_str() {
            "max_amount_minor" => {
                if let Ok(max_amount) = constraint.value.parse::<u64>() {
                    request.amount_minor = request.amount_minor.min(max_amount);
                }
            }
            "require_check" => {
                if !request.compliance_flags.contains(&constraint.value) {
                    request.compliance_flags.push(constraint.value.clone());
                }
            }
            key => {
                request.metadata.insert(
                    format!("attestation_constraint_{key}"),
                    constraint.value.clone(),
                );
            }
        }
    }
}

fn build_human_attestation(
    trace_id: &str,
    signer_id: String,
    decision: AttestationDecision,
    note: Option<String>,
    constraints: Vec<AttestationConstraint>,
    signature: Option<String>,
    anchor: Option<String>,
) -> HumanAttestation {
    let attested_at = Utc::now();
    let attestation_id = format!("attest-{}", Uuid::new_v4());
    let anchor_value =
        anchor.unwrap_or_else(|| format!("attestation://{trace_id}/{attestation_id}"));
    let signature_value = signature.unwrap_or_else(|| {
        let material = serde_json::json!({
            "trace_id": trace_id,
            "attestation_id": attestation_id,
            "signer_id": signer_id,
            "decision": decision,
            "attested_at": attested_at,
            "anchor": anchor_value,
            "constraints": constraints,
            "note": note,
        });
        blake3::hash(material.to_string().as_bytes())
            .to_hex()
            .to_string()
    });

    HumanAttestation {
        attestation_id,
        decision,
        signer_id,
        signature: signature_value,
        anchor: anchor_value,
        attested_at,
        constraints,
        note,
    }
}

fn parse_attestation_decision(
    value: Option<&str>,
    default_decision: AttestationDecision,
) -> Result<AttestationDecision, ApiError> {
    let Some(raw) = value else {
        return Ok(default_decision);
    };

    match raw.to_ascii_lowercase().as_str() {
        "approve" => Ok(AttestationDecision::Approve),
        "deny" => Ok(AttestationDecision::Deny),
        "modify" => Ok(AttestationDecision::Modify),
        other => Err(ApiError::bad_request(format!(
            "invalid decision '{}'; expected approve|deny|modify",
            other
        ))),
    }
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
struct BridgeExecuteRequest {
    trace_id: Option<String>,
    execution_id: Option<String>,
    commitment_id: String,
    origin_actor: String,
    counterparty_actor: String,
    legs: Vec<BridgeLeg>,
}

async fn execute_bridge(
    State(state): State<ServiceState>,
    Json(request): Json<BridgeExecuteRequest>,
) -> Result<Json<UnifiedBridgeReceipt>, ApiError> {
    if request.commitment_id.trim().is_empty() {
        return Err(ApiError::bad_request("commitment_id is required"));
    }

    if request.legs.is_empty() {
        return Err(ApiError::bad_request("at least one bridge leg is required"));
    }

    let bridge_request = BridgeExecutionRequest::new(
        request
            .execution_id
            .unwrap_or_else(|| format!("exec-{}", Uuid::new_v4())),
        request
            .trace_id
            .unwrap_or_else(|| format!("trace-{}", Uuid::new_v4())),
        request.commitment_id,
        request.origin_actor,
        request.counterparty_actor,
        request.legs,
    );

    Ok(Json(
        state
            .engine
            .execute_bridge_route(bridge_request)
            .await
            .map_err(ApiError::Core)?,
    ))
}

#[derive(Debug, Clone, Deserialize)]
struct BridgeReceiptsQuery {
    trace_id: Option<String>,
    execution_id: Option<String>,
    commitment_id: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    order: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BridgeReceiptsResponse {
    total: usize,
    returned: usize,
    items: Vec<UnifiedBridgeReceipt>,
}

async fn list_bridge_receipts(
    State(state): State<ServiceState>,
    Query(query): Query<BridgeReceiptsQuery>,
) -> Result<Json<BridgeReceiptsResponse>, ApiError> {
    let mut receipts = state.engine.bridge_receipts().await?;

    if let Some(trace_id) = query.trace_id.as_deref() {
        receipts.retain(|receipt| receipt.trace_id == trace_id);
    }

    if let Some(execution_id) = query.execution_id.as_deref() {
        receipts.retain(|receipt| receipt.execution_id == execution_id);
    }

    if let Some(commitment_id) = query.commitment_id.as_deref() {
        receipts.retain(|receipt| receipt.commitment_id == commitment_id);
    }

    if let Some(status) = query.status.as_deref() {
        let expected = status.to_ascii_lowercase();
        receipts.retain(|receipt| {
            let actual = match receipt.status {
                ibank_core::UnifiedBridgeStatus::Settled => "settled",
                ibank_core::UnifiedBridgeStatus::Failed => "failed",
            };
            actual == expected
        });
    }

    let order = query
        .order
        .as_deref()
        .unwrap_or("desc")
        .to_ascii_lowercase();
    if order == "desc" {
        receipts.reverse();
    } else if order != "asc" {
        return Err(ApiError::bad_request(format!(
            "invalid order '{}'; expected asc or desc",
            order
        )));
    }

    let total = receipts.len();
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(100).min(1000);
    let items = receipts
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let returned = items.len();

    Ok(Json(BridgeReceiptsResponse {
        total,
        returned,
        items,
    }))
}

#[derive(Debug, Clone, Serialize)]
struct ComplianceAuditRecord {
    entry_id: String,
    stage: String,
    detail: String,
    observed_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
struct ComplianceTraceResponse {
    trace_id: String,
    commitment_id: Option<String>,
    compliance_proof: Option<ComplianceProof>,
    regulatory_status: Option<String>,
    required_checks: Vec<String>,
    risk_score: Option<u8>,
    fraud_score: Option<u8>,
    compliance_audits: Vec<ComplianceAuditRecord>,
}

async fn get_compliance_trace(
    Path(trace_id): Path<String>,
    State(state): State<ServiceState>,
) -> Result<Json<ComplianceTraceResponse>, ApiError> {
    let entries = state.engine.ledger_entries().await?;
    let trace_entries = entries
        .into_iter()
        .filter(|entry| entry.trace_id == trace_id)
        .collect::<Vec<_>>();

    if trace_entries.is_empty() {
        return Err(ApiError::not_found(format!(
            "trace '{}' not found in ledger",
            trace_id
        )));
    }

    let commitment_entry = trace_entries
        .iter()
        .rev()
        .find(|entry| entry.kind == LedgerEntryKind::Commitment);

    let mut commitment_id = None;
    let mut compliance_proof = None;
    let mut regulatory_status = None;
    let mut required_checks = Vec::new();
    let mut risk_score = None;
    let mut fraud_score = None;

    if let Some(entry) = commitment_entry {
        commitment_id = entry.commitment_id.clone();
        let record: CommitmentRecord =
            serde_json::from_value(entry.payload.clone()).map_err(|error| {
                ApiError::Core(ibank_core::IBankError::Serialization(format!(
                    "failed to decode commitment record: {error}"
                )))
            })?;

        compliance_proof = Some(record.platform.compliance_proof.clone());
        regulatory_status = Some(record.platform.regulatory_compliance.status.clone());
        required_checks = record
            .platform
            .regulatory_compliance
            .required_checks
            .clone();
        risk_score = Some(record.platform.risk_assessment.score);
        fraud_score = Some(record.platform.risk_assessment.fraud_score);
    }

    let mut compliance_audits = trace_entries
        .iter()
        .filter(|entry| entry.kind == LedgerEntryKind::Audit)
        .filter_map(|entry| {
            let stage = entry
                .payload
                .get("stage")
                .and_then(|value| value.as_str())?;
            if !(stage.starts_with("compliance_") || stage == "risk_scored") {
                return None;
            }
            let detail = entry
                .payload
                .get("detail")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            Some(ComplianceAuditRecord {
                entry_id: entry.entry_id.clone(),
                stage: stage.to_string(),
                detail,
                observed_at: entry.timestamp,
            })
        })
        .collect::<Vec<_>>();

    compliance_audits.sort_by(|left, right| left.observed_at.cmp(&right.observed_at));

    Ok(Json(ComplianceTraceResponse {
        trace_id,
        commitment_id,
        compliance_proof,
        regulatory_status,
        required_checks,
        risk_score,
        fraud_score,
        compliance_audits,
    }))
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

#[derive(Debug, Clone, Serialize)]
struct ApprovalCaseResponse {
    trace_id: String,
    active: bool,
    workflow_state: EscalationWorkflowState,
    commitment_id: Option<String>,
    decision_reason: Option<String>,
    escalation_case: Option<EscalationCase>,
    risk_report: Option<RiskReport>,
    latest_attestation: Option<HumanAttestation>,
    attestation_history: Vec<HumanAttestation>,
    queued_at: Option<chrono::DateTime<Utc>>,
    updated_at: Option<chrono::DateTime<Utc>>,
}

async fn get_approval_case(
    Path(trace_id): Path<String>,
    State(state): State<ServiceState>,
) -> Result<Json<ApprovalCaseResponse>, ApiError> {
    let queue_case = {
        let queue = state.queue.lock().await;
        queue.get(&trace_id).cloned()
    };

    let ledger_entries = state.engine.ledger_entries().await?;
    let trace_entries = ledger_entries
        .into_iter()
        .filter(|entry| entry.trace_id == trace_id)
        .collect::<Vec<_>>();

    if queue_case.is_none() && trace_entries.is_empty() {
        return Err(ApiError::not_found(format!(
            "approval case '{}' not found",
            trace_id
        )));
    }

    let mut ledger_attestations = extract_attestation_history_from_entries(&trace_entries)?;

    if let Some(case) = queue_case {
        let mut attestation_history = case.attestation_history.clone();
        attestation_history.append(&mut ledger_attestations);
        attestation_history.sort_by(|left, right| left.attested_at.cmp(&right.attested_at));
        attestation_history.dedup_by(|left, right| left.attestation_id == right.attestation_id);
        let latest_attestation = attestation_history
            .last()
            .cloned()
            .or(case.human_attestation.clone());

        return Ok(Json(ApprovalCaseResponse {
            trace_id: case.trace_id,
            active: true,
            workflow_state: case.workflow_state,
            commitment_id: case.commitment_id,
            decision_reason: Some(case.decision_reason),
            escalation_case: case.escalation_case,
            risk_report: case.risk_report,
            latest_attestation,
            attestation_history,
            queued_at: Some(case.queued_at),
            updated_at: Some(case.updated_at),
        }));
    }

    let commitment_entry = trace_entries
        .iter()
        .rev()
        .find(|entry| entry.kind == LedgerEntryKind::Commitment);
    let commitment_id = commitment_entry.and_then(|entry| entry.commitment_id.clone());

    let latest_attestation = ledger_attestations.last().cloned();
    let has_outcome = trace_entries
        .iter()
        .any(|entry| entry.kind == LedgerEntryKind::Outcome);
    let has_success_outcome = trace_entries.iter().any(|entry| {
        entry.kind == LedgerEntryKind::Outcome
            && entry
                .payload
                .get("success")
                .and_then(|value| value.as_bool())
                == Some(true)
    });

    let decision_reason = trace_entries.iter().rev().find_map(|entry| {
        if entry.kind != LedgerEntryKind::Outcome {
            return None;
        }
        entry
            .payload
            .get("detail")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
    });

    let workflow_state = if has_success_outcome || has_outcome {
        EscalationWorkflowState::Closed
    } else if let Some(attestation) = &latest_attestation {
        match attestation.decision {
            AttestationDecision::Deny => EscalationWorkflowState::Denied,
            AttestationDecision::Approve | AttestationDecision::Modify => {
                EscalationWorkflowState::Approved
            }
        }
    } else {
        EscalationWorkflowState::Open
    };

    let queued_at = trace_entries.iter().map(|entry| entry.timestamp).min();
    let updated_at = trace_entries.iter().map(|entry| entry.timestamp).max();

    Ok(Json(ApprovalCaseResponse {
        trace_id,
        active: false,
        workflow_state,
        commitment_id,
        decision_reason,
        escalation_case: None,
        risk_report: None,
        latest_attestation,
        attestation_history: ledger_attestations,
        queued_at,
        updated_at,
    }))
}

fn extract_attestation_history_from_entries(
    entries: &[LedgerEntry],
) -> Result<Vec<HumanAttestation>, ApiError> {
    let mut attestations = entries
        .iter()
        .filter(|entry| entry.kind == LedgerEntryKind::Audit)
        .filter_map(|entry| {
            let stage = entry
                .payload
                .get("stage")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            if stage != "human_attestation_recorded" {
                return None;
            }
            entry.payload.get("detail").and_then(|value| value.as_str())
        })
        .map(|detail| {
            serde_json::from_str::<HumanAttestation>(detail).map_err(|error| {
                ApiError::Core(ibank_core::IBankError::Serialization(format!(
                    "failed to decode human attestation from audit: {error}"
                )))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    attestations.sort_by(|left, right| left.attested_at.cmp(&right.attested_at));
    attestations.dedup_by(|left, right| left.attestation_id == right.attestation_id);
    Ok(attestations)
}

#[derive(Debug, Clone, Deserialize)]
struct ApprovalRequest {
    approver_id: String,
    note: Option<String>,
    decision: Option<String>,
    signature: Option<String>,
    anchor: Option<String>,
    constraints: Option<Vec<AttestationConstraint>>,
}

async fn approve_pending(
    Path(trace_id): Path<String>,
    State(state): State<ServiceState>,
    Json(approval): Json<ApprovalRequest>,
) -> Result<Json<HandleResponse>, ApiError> {
    let decision =
        parse_attestation_decision(approval.decision.as_deref(), AttestationDecision::Approve)?;
    if decision == AttestationDecision::Deny {
        return Err(ApiError::bad_request(
            "deny decision is not valid for /approve; use /reject",
        ));
    }

    let attestation = build_human_attestation(
        &trace_id,
        approval.approver_id,
        decision,
        approval.note,
        approval.constraints.unwrap_or_default(),
        approval.signature,
        approval.anchor,
    );

    Ok(Json(
        approve_pending_with_attestation(&state, &trace_id, attestation).await?,
    ))
}

async fn reject_pending(
    Path(trace_id): Path<String>,
    State(state): State<ServiceState>,
    Json(rejection): Json<ApprovalRequest>,
) -> Result<Json<RejectOutcome>, ApiError> {
    let decision =
        parse_attestation_decision(rejection.decision.as_deref(), AttestationDecision::Deny)?;
    if decision != AttestationDecision::Deny {
        return Err(ApiError::bad_request(
            "only deny decision is valid for /reject",
        ));
    }
    let attestation = build_human_attestation(
        &trace_id,
        rejection.approver_id,
        decision,
        rejection.note,
        rejection.constraints.unwrap_or_default(),
        rejection.signature,
        rejection.anchor,
    );
    reject_pending_with_attestation(&state, &trace_id, attestation).await?;
    Ok(Json(RejectOutcome {
        trace_id,
        status: "rejected",
    }))
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
        assert_eq!(
            persisted
                .list()
                .first()
                .map(|item| item.workflow_state.clone()),
            Some(ibank_core::EscalationWorkflowState::Open)
        );
    }

    #[tokio::test]
    async fn high_risk_transfer_cannot_bypass_attestation_via_handle() {
        let path = std::env::temp_dir()
            .join(format!("ibank-no-bypass-{}", Uuid::new_v4()))
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
            "approval": {
              "approved": true,
              "approver_id": "should-not-bypass",
              "note": "attempt bypass",
              "approved_at": "2026-02-05T00:00:00Z"
            }
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

    #[tokio::test]
    async fn bridge_execute_endpoint_returns_unified_receipt() {
        let path = std::env::temp_dir()
            .join(format!("ibank-bridge-{}", Uuid::new_v4()))
            .join("approvals.json");
        let state = ServiceState::bootstrap(ServiceConfig {
            queue_path: path,
            ledger_storage: LedgerStorageConfig::Memory,
        })
        .await
        .unwrap();
        let app = build_router(state);

        let handle_payload = serde_json::json!({
            "origin_actor": "issuer-a",
            "counterparty_actor": "merchant-b",
            "transaction_type": "transfer",
            "amount_minor": 50000,
            "currency": "USD",
            "rail": "ach",
            "destination": "acct-123",
            "jurisdiction": "US",
            "user_intent": "seed bridge commitment",
            "ambiguity_hint": 0.1,
            "counterparty_risk": 10,
            "anomaly_score": 8,
            "model_uncertainty": 0.08,
            "compliance_flags": [],
            "metadata": {},
            "approval": null
        });

        let handle_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/handle")
                    .header("content-type", "application/json")
                    .body(Body::from(handle_payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(handle_response.status(), StatusCode::OK);
        let handle_bytes = to_bytes(handle_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let handle_body: HandleResponse = serde_json::from_slice(&handle_bytes).unwrap();
        let commitment_id = handle_body
            .commitment_id
            .expect("commitment should be present");

        let bridge_payload = serde_json::json!({
            "trace_id": "trace-bridge-1",
            "execution_id": "exec-bridge-1",
            "commitment_id": commitment_id,
            "origin_actor": "issuer-a",
            "counterparty_actor": "merchant-b",
            "legs": [
                {
                    "type": "chain",
                    "leg_id": "leg-chain-1",
                    "adapter_id": "evm-mock",
                    "network": "base-sepolia",
                    "asset": "USDC",
                    "asset_kind": "stablecoin",
                    "from_address": "0xaaa",
                    "to_address": "0xbbb",
                    "amount_minor": 25000,
                    "memo": "fiat->stablecoin"
                },
                {
                    "type": "rail",
                    "leg_id": "leg-rail-1",
                    "adapter_id": "rail-mock",
                    "rail": "ach",
                    "currency": "USD",
                    "from_account": "acct-a",
                    "to_account": "acct-b",
                    "amount_minor": 25000,
                    "memo": "stablecoin->bank rail"
                }
            ]
        });

        let bridge_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/bridge/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(bridge_payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bridge_response.status(), StatusCode::OK);

        let bridge_bytes = to_bytes(bridge_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let bridge_body: serde_json::Value = serde_json::from_slice(&bridge_bytes).unwrap();

        assert_eq!(
            bridge_body.get("status").and_then(|value| value.as_str()),
            Some("settled")
        );
        assert_eq!(
            bridge_body
                .get("route_type")
                .and_then(|value| value.as_str()),
            Some("hybrid")
        );
        assert_eq!(
            bridge_body
                .get("leg_receipts")
                .and_then(|value| value.as_array())
                .map(|items| items.len()),
            Some(2)
        );

        let receipts_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/bridge/receipts?trace_id=trace-bridge-1&status=settled")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(receipts_response.status(), StatusCode::OK);

        let receipts_bytes = to_bytes(receipts_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let receipts_body: serde_json::Value = serde_json::from_slice(&receipts_bytes).unwrap();
        assert_eq!(
            receipts_body
                .get("returned")
                .and_then(|value| value.as_u64()),
            Some(1)
        );
        assert_eq!(
            receipts_body
                .get("items")
                .and_then(|value| value.as_array())
                .and_then(|items| items.first())
                .and_then(|item| item.get("execution_id"))
                .and_then(|value| value.as_str()),
            Some("exec-bridge-1")
        );
    }

    #[tokio::test]
    async fn denial_attestation_records_failure_and_stops_execution() {
        let path = std::env::temp_dir()
            .join(format!("ibank-denial-{}", Uuid::new_v4()))
            .join("approvals.json");
        let state = ServiceState::bootstrap(ServiceConfig {
            queue_path: path.clone(),
            ledger_storage: LedgerStorageConfig::Memory,
        })
        .await
        .unwrap();
        let app = build_router(state.clone());

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

        let pending_response = app
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
        assert_eq!(pending_response.status(), StatusCode::OK);
        let pending_body: HandleResponse = serde_json::from_slice(
            &to_bytes(pending_response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(pending_body.status, HandleStatus::PendingHumanApproval);

        let reject_payload = serde_json::json!({
            "approver_id": "risk-officer-1",
            "decision": "deny",
            "note": "declined by compliance",
            "signature": "sig-deny-1",
            "anchor": "attestation://test/deny-1"
        });

        let reject_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/v1/approvals/{}/reject", pending_body.trace_id))
                    .header("content-type", "application/json")
                    .body(Body::from(reject_payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reject_response.status(), StatusCode::OK);

        let entries = state.engine.ledger_entries().await.unwrap();
        let has_attestation_audit = entries.iter().any(|entry| {
            entry.kind == LedgerEntryKind::Audit
                && entry.trace_id == pending_body.trace_id
                && entry
                    .payload
                    .get("stage")
                    .and_then(|v| v.as_str())
                    .map(|stage| stage == "human_attestation_recorded")
                    .unwrap_or(false)
        });
        assert!(has_attestation_audit);

        let has_failure_outcome = entries.iter().any(|entry| {
            entry.kind == LedgerEntryKind::Outcome
                && entry.trace_id == pending_body.trace_id
                && entry.payload.get("success").and_then(|v| v.as_bool()) == Some(false)
        });
        assert!(has_failure_outcome);

        let has_success_outcome = entries.iter().any(|entry| {
            entry.kind == LedgerEntryKind::Outcome
                && entry.trace_id == pending_body.trace_id
                && entry.payload.get("success").and_then(|v| v.as_bool()) == Some(true)
        });
        assert!(!has_success_outcome);
    }

    #[tokio::test]
    async fn approval_case_endpoint_returns_active_case_details() {
        let path = std::env::temp_dir()
            .join(format!("ibank-case-active-{}", Uuid::new_v4()))
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

        let pending_response = app
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
        assert_eq!(pending_response.status(), StatusCode::OK);
        let pending_body: HandleResponse = serde_json::from_slice(
            &to_bytes(pending_response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();

        let case_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/v1/approvals/case/{}", pending_body.trace_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(case_response.status(), StatusCode::OK);
        let body: serde_json::Value = serde_json::from_slice(
            &to_bytes(case_response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body.get("active").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            body.get("workflow_state").and_then(|v| v.as_str()),
            Some("open")
        );
        assert!(body
            .get("escalation_case")
            .and_then(|v| v.get("case_id"))
            .and_then(|v| v.as_str())
            .map(|v| !v.is_empty())
            .unwrap_or(false));
        assert_eq!(
            body.get("attestation_history")
                .and_then(|v| v.as_array())
                .map(|items| items.len()),
            Some(0)
        );
    }

    #[tokio::test]
    async fn approval_case_endpoint_reconstructs_closed_case_history() {
        let path = std::env::temp_dir()
            .join(format!("ibank-case-closed-{}", Uuid::new_v4()))
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

        let pending_response = app
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
        assert_eq!(pending_response.status(), StatusCode::OK);
        let pending_body: HandleResponse = serde_json::from_slice(
            &to_bytes(pending_response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();

        let reject_payload = serde_json::json!({
            "approver_id": "risk-officer-1",
            "decision": "deny",
            "note": "declined by compliance",
            "signature": "sig-deny-case",
            "anchor": "attestation://test/deny-case"
        });
        let reject_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/v1/approvals/{}/reject", pending_body.trace_id))
                    .header("content-type", "application/json")
                    .body(Body::from(reject_payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reject_response.status(), StatusCode::OK);

        let case_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/v1/approvals/case/{}", pending_body.trace_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(case_response.status(), StatusCode::OK);
        let body: serde_json::Value = serde_json::from_slice(
            &to_bytes(case_response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body.get("active").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            body.get("workflow_state").and_then(|v| v.as_str()),
            Some("closed")
        );
        assert_eq!(
            body.get("attestation_history")
                .and_then(|v| v.as_array())
                .map(|items| items.len()),
            Some(1)
        );
        assert_eq!(
            body.get("latest_attestation")
                .and_then(|v| v.get("decision"))
                .and_then(|v| v.as_str()),
            Some("deny")
        );
    }

    #[tokio::test]
    async fn compliance_trace_endpoint_returns_proof_and_audits() {
        let path = std::env::temp_dir()
            .join(format!("ibank-compliance-{}", Uuid::new_v4()))
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

        let handle_response = app
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
        assert_eq!(handle_response.status(), StatusCode::OK);

        let handle_bytes = to_bytes(handle_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let handle_body: HandleResponse = serde_json::from_slice(&handle_bytes).unwrap();

        let endpoint = format!("/v1/compliance/trace/{}", handle_body.trace_id);
        let compliance_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&endpoint)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(compliance_response.status(), StatusCode::OK);

        let body_bytes = to_bytes(compliance_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(
            body.get("compliance_proof")
                .and_then(|value| value.get("policy_version"))
                .and_then(|value| value.as_str()),
            Some("ibank-compliance-v1")
        );
        assert_eq!(
            body.get("compliance_proof")
                .and_then(|value| value.get("decision"))
                .and_then(|value| value.as_str()),
            Some("green")
        );
        assert!(body
            .get("compliance_audits")
            .and_then(|value| value.as_array())
            .map(|items| !items.is_empty())
            .unwrap_or(false));
    }

    #[tokio::test]
    async fn compliance_trace_endpoint_returns_404_for_missing_trace() {
        let path = std::env::temp_dir()
            .join(format!("ibank-compliance-missing-{}", Uuid::new_v4()))
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
                    .uri("/v1/compliance/trace/unknown-trace")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
