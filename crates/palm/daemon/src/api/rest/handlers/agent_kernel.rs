//! AgentKernel API handlers.

use crate::api::rest::state::AppState;
use crate::error::{ApiError, ApiResult};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use maple_runtime::{
    AgentHandleRequest, AgentKernelError, CapabilityExecution, ModelBackend, StructuredCognition,
};
use maple_storage::QueryWindow;
use rcf_commitment::CommitmentId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct AgentKernelStatusResponse {
    pub resonator_id: String,
    pub audit_events: usize,
    pub last_event: Option<maple_runtime::AgentAuditEvent>,
    pub backends: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentKernelHandlePayload {
    pub prompt: String,
    #[serde(default = "default_backend")]
    pub backend: String,
    pub tool: Option<String>,
    pub args: Option<Value>,
    #[serde(default)]
    pub with_commitment: bool,
    pub commitment_outcome: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentKernelAuditQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Deserialize)]
pub struct AgentKernelCommitmentsQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Serialize)]
pub struct AgentKernelHandleResponse {
    pub resonator_id: String,
    pub cognition: StructuredCognition,
    pub action: Option<CapabilityExecution>,
    pub audit_event_id: String,
    pub raw_model_output: String,
}

#[derive(Debug, Serialize)]
pub struct AgentKernelCommitmentResponse {
    pub commitment_id: String,
    pub lifecycle_status: String,
    pub principal: String,
    pub effect_domain: String,
    pub decision: String,
    pub declared_at: chrono::DateTime<chrono::Utc>,
    pub execution_started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub execution_completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub outcome: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct AgentKernelCommitmentSummaryResponse {
    pub commitment_id: String,
    pub lifecycle_status: String,
    pub decision: String,
    pub principal: String,
    pub effect_domain: String,
    pub declared_at: chrono::DateTime<chrono::Utc>,
    pub execution_started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub execution_completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn default_backend() -> String {
    "local_llama".to_string()
}

fn default_limit() -> usize {
    100
}

fn parse_backend(raw: &str) -> Result<ModelBackend, ApiError> {
    match raw {
        "local_llama" => Ok(ModelBackend::LocalLlama),
        "open_ai" => Ok(ModelBackend::OpenAi),
        "anthropic" => Ok(ModelBackend::Anthropic),
        "gemini" => Ok(ModelBackend::Gemini),
        "grok" => Ok(ModelBackend::Grok),
        other => Err(ApiError::BadRequest(format!(
            "unsupported backend '{}'; expected one of: local_llama, open_ai, anthropic, gemini, grok",
            other
        ))),
    }
}

/// Show daemon-managed AgentKernel status.
pub async fn agent_kernel_status(
    State(state): State<AppState>,
) -> ApiResult<Json<AgentKernelStatusResponse>> {
    let audits = state.agent_kernel.audit_events().await;

    Ok(Json(AgentKernelStatusResponse {
        resonator_id: state.agent_kernel_resonator_id.to_string(),
        audit_events: audits.len(),
        last_event: audits.last().cloned(),
        backends: vec![
            "local_llama".to_string(),
            "open_ai".to_string(),
            "anthropic".to_string(),
            "gemini".to_string(),
            "grok".to_string(),
        ],
    }))
}

/// Execute one AgentKernel step through daemon API.
pub async fn agent_kernel_handle(
    State(state): State<AppState>,
    Json(payload): Json<AgentKernelHandlePayload>,
) -> ApiResult<Json<AgentKernelHandleResponse>> {
    let backend = parse_backend(&payload.backend)?;

    let mut request = AgentHandleRequest::new(
        state.agent_kernel_resonator_id,
        backend,
        payload.prompt.clone(),
    );
    request.override_tool = payload.tool.clone();
    request.override_args = payload.args.clone();

    if payload.with_commitment {
        if let Some(ref tool) = payload.tool {
            let outcome = payload
                .commitment_outcome
                .clone()
                .unwrap_or_else(|| format!("Authorized execution for capability {}", tool));
            let commitment = state
                .agent_kernel
                .draft_commitment(state.agent_kernel_resonator_id, tool, outcome)
                .await
                .map_err(|err| ApiError::BadRequest(err.to_string()))?;
            request.commitment = Some(commitment);
        }
    }

    let result = state
        .agent_kernel
        .handle(request)
        .await
        .map_err(map_agent_error)?;

    Ok(Json(AgentKernelHandleResponse {
        resonator_id: result.resonator_id.to_string(),
        cognition: result.cognition,
        action: result.action,
        audit_event_id: result.audit_event_id,
        raw_model_output: result.raw_model_output,
    }))
}

/// List recent daemon AgentKernel audit events.
pub async fn agent_kernel_audit(
    State(state): State<AppState>,
    Query(query): Query<AgentKernelAuditQuery>,
) -> ApiResult<Json<Vec<maple_runtime::AgentAuditEvent>>> {
    let mut audits = state.agent_kernel.audit_events().await;

    if audits.len() > query.limit {
        let keep_from = audits.len() - query.limit;
        audits = audits.split_off(keep_from);
    }

    Ok(Json(audits))
}

/// List recent commitment lifecycle records from the shared durable ledger.
pub async fn agent_kernel_commitments(
    State(state): State<AppState>,
    Query(query): Query<AgentKernelCommitmentsQuery>,
) -> ApiResult<Json<Vec<AgentKernelCommitmentSummaryResponse>>> {
    let records = state
        .agent_kernel
        .storage()
        .list_commitments(QueryWindow {
            limit: query.limit,
            offset: 0,
        })
        .await
        .map_err(|err| ApiError::Internal(err.to_string()))?;

    let items = records
        .into_iter()
        .map(|record| AgentKernelCommitmentSummaryResponse {
            commitment_id: record.commitment_id.0,
            lifecycle_status: format!("{:?}", record.lifecycle_status),
            decision: format!("{:?}", record.decision.decision),
            principal: record.commitment.principal.id,
            effect_domain: format!("{}", record.commitment.effect_domain),
            declared_at: record.created_at,
            execution_started_at: record.execution_started_at,
            execution_completed_at: record.execution_completed_at,
            updated_at: record.updated_at,
        })
        .collect();

    Ok(Json(items))
}

/// Retrieve one commitment lifecycle record from the shared durable ledger.
pub async fn agent_kernel_commitment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<AgentKernelCommitmentResponse>> {
    let commitment_id = CommitmentId::new(id.clone());
    let record = state
        .agent_kernel
        .storage()
        .get_commitment(&commitment_id)
        .await
        .map_err(|err| ApiError::Internal(err.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("commitment '{}' not found", id)))?;

    let outcome = record
        .outcome
        .clone()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|err| ApiError::Internal(err.to_string()))?;

    Ok(Json(AgentKernelCommitmentResponse {
        commitment_id: record.commitment_id.0,
        lifecycle_status: format!("{:?}", record.lifecycle_status),
        principal: record.commitment.principal.id,
        effect_domain: format!("{}", record.commitment.effect_domain),
        decision: format!("{:?}", record.decision.decision),
        declared_at: record.created_at,
        execution_started_at: record.execution_started_at,
        execution_completed_at: record.execution_completed_at,
        updated_at: record.updated_at,
        outcome,
    }))
}

fn map_agent_error(err: AgentKernelError) -> ApiError {
    match err {
        AgentKernelError::ContractMissing { reason }
            if reason.contains("missing explicit commitment") =>
        {
            ApiError::PolicyDenied(format!("contract missing: {}", reason))
        }
        AgentKernelError::ContractMissing { reason } => {
            ApiError::BadRequest(format!("contract missing: {}", reason))
        }
        AgentKernelError::ApprovalRequired(_)
        | AgentKernelError::CapabilityDenied
        | AgentKernelError::PolicyDenied(_)
        | AgentKernelError::CapabilityDeniedDetail(_) => ApiError::PolicyDenied(err.to_string()),
        AgentKernelError::UnknownCapability(_)
        | AgentKernelError::CommitmentValidation(_)
        | AgentKernelError::CommitmentCapabilityMismatch { .. } => {
            ApiError::BadRequest(err.to_string())
        }
        AgentKernelError::ModelAdapterMissing(_) => ApiError::Internal(err.to_string()),
        _ => ApiError::Internal(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use crate::api::create_router;
    use crate::api::rest::state::AppState;
    use crate::config::SchedulerConfig;
    use crate::error::ApiError;
    use crate::playground::PlaygroundService;
    use crate::scheduler::Scheduler;
    use crate::storage::{InMemoryStorage, Storage};
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::response::IntoResponse;
    use maple_runtime::{
        config::RuntimeConfig, AgentKernel, AgentKernelError, AgentRegistration, MapleRuntime,
    };
    use palm_shared_state::Activity;
    use palm_types::{PalmEventEnvelope, PlatformProfile};
    use std::sync::Arc;
    use tokio::sync::{broadcast, watch};
    use tower::util::ServiceExt;

    async fn test_app() -> axum::Router {
        let storage: Arc<dyn Storage> = Arc::new(InMemoryStorage::new());
        let (event_tx, _) = broadcast::channel::<PalmEventEnvelope>(64);
        let (activity_tx, _) = broadcast::channel::<Activity>(64);
        let (scheduler, _reconcile_rx) = Scheduler::with_platform(
            SchedulerConfig::default(),
            Arc::clone(&storage),
            event_tx.clone(),
            activity_tx.clone(),
            PlatformProfile::Development,
        );

        let playground = PlaygroundService::new(Arc::clone(&storage), activity_tx.clone())
            .await
            .expect("playground should initialize");

        let runtime = MapleRuntime::bootstrap(RuntimeConfig::default())
            .await
            .expect("runtime should bootstrap");
        let agent_kernel = Arc::new(AgentKernel::new(runtime));
        let host = agent_kernel
            .register_agent(AgentRegistration::default())
            .await
            .expect("agent should register");

        let (shutdown_tx, _shutdown_rx) = watch::channel(false);
        let state = AppState::new(
            storage,
            scheduler,
            event_tx,
            activity_tx,
            playground,
            agent_kernel,
            host.resonator_id,
            shutdown_tx,
        );

        create_router(state)
    }

    #[tokio::test]
    async fn dangerous_handle_without_commitment_returns_policy_denied() {
        let app = test_app().await;
        let payload = serde_json::json!({
            "prompt": "transfer 500 usd to demo account",
            "backend": "local_llama",
            "tool": "simulate_transfer",
            "args": { "amount": 500, "to": "demo" },
            "with_commitment": false
        });

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/agent-kernel/handle")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .expect("request should build");

        let response = app.oneshot(request).await.expect("request should complete");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let error = serde_json::from_slice::<serde_json::Value>(&body)
            .expect("error response should deserialize");
        assert_eq!(error["code"], "POLICY_DENIED");
        assert!(
            error["error"]
                .as_str()
                .unwrap_or_default()
                .contains("contract missing"),
            "expected contract-missing error, got: {}",
            error["error"]
        );
    }

    #[tokio::test]
    async fn dangerous_alias_handle_without_commitment_returns_policy_denied() {
        let app = test_app().await;
        let payload = serde_json::json!({
            "prompt": "transfer 500 usd to demo account",
            "backend": "local_llama",
            "tool": "simulate_transfer",
            "args": { "amount": 500, "to": "demo" },
            "with_commitment": false
        });

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/agent/handle")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .expect("request should build");

        let response = app.oneshot(request).await.expect("request should complete");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let error = serde_json::from_slice::<serde_json::Value>(&body)
            .expect("error response should deserialize");
        assert_eq!(error["code"], "POLICY_DENIED");
        assert!(
            error["error"]
                .as_str()
                .unwrap_or_default()
                .contains("contract missing"),
            "expected contract-missing error, got: {}",
            error["error"]
        );
    }

    #[test]
    fn contract_missing_with_explicit_reference_maps_to_bad_request() {
        let api_error = super::map_agent_error(AgentKernelError::ContractMissing {
            reason: "contract contract:test does not exist".to_string(),
        });
        assert!(matches!(&api_error, ApiError::BadRequest(_)));

        let status = api_error.into_response().status();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}
