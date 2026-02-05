//! AgentKernel API handlers.

use crate::api::rest::state::AppState;
use crate::error::{ApiError, ApiResult};
use axum::{
    extract::{Query, State},
    Json,
};
use maple_runtime::{
    AgentHandleRequest, AgentKernelError, CapabilityExecution, ModelBackend, StructuredCognition,
};
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

#[derive(Debug, Serialize)]
pub struct AgentKernelHandleResponse {
    pub resonator_id: String,
    pub cognition: StructuredCognition,
    pub action: Option<CapabilityExecution>,
    pub audit_event_id: String,
    pub raw_model_output: String,
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

fn map_agent_error(err: AgentKernelError) -> ApiError {
    match err {
        AgentKernelError::MissingCommitment { .. }
        | AgentKernelError::ApprovalRequired(_)
        | AgentKernelError::CapabilityDenied => ApiError::PolicyDenied(err.to_string()),
        AgentKernelError::UnknownCapability(_)
        | AgentKernelError::ModelAdapterMissing(_)
        | AgentKernelError::CommitmentValidation(_) => ApiError::BadRequest(err.to_string()),
        _ => ApiError::Internal(err.to_string()),
    }
}
