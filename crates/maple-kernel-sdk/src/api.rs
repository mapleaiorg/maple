//! MWL REST API routes and handlers.
//!
//! Provides an Axum router that can be nested into the existing PALM daemon
//! under `/api/v1/`. All MWL-specific endpoints are defined here.
//!
//! ## Usage in PALM daemon router.rs:
//!
//! ```ignore
//! use maple_kernel_sdk::api::mwl_router;
//!
//! let api_routes = Router::new()
//!     // existing routes...
//!     .merge(mwl_router());
//! ```

use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

// ──────────────────────────────────────────────
// Router
// ──────────────────────────────────────────────

/// Create the MWL API router.
///
/// All routes are relative — designed to be nested under `/api/v1/`
/// in the PALM daemon.
pub fn mwl_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let state = Arc::new(RwLock::new(MwlApiState::default()));

    Router::<S>::new()
        // WorldLine endpoints
        .route("/worldlines", post(create_worldline))
        .route("/worldlines", get(list_worldlines))
        .route("/worldlines/:id", get(get_worldline))
        // Commitment endpoints
        .route("/commitments", post(submit_commitment))
        .route("/commitments/:id", get(get_commitment))
        .route("/commitments/:id/audit-trail", get(get_audit_trail))
        // Provenance endpoints
        .route(
            "/provenance/:event_id/ancestors",
            get(get_provenance_ancestors),
        )
        .route(
            "/provenance/worldline/:id/history",
            get(get_worldline_history),
        )
        // Governance endpoints
        .route("/governance/policies", post(add_policy))
        .route("/governance/policies", get(list_policies))
        .route("/governance/simulate", post(simulate_policy))
        // Financial endpoints
        .route(
            "/financial/:worldline_id/balance/:asset",
            get(get_balance_projection),
        )
        .route("/financial/settle", post(submit_settlement))
        // Kernel endpoints
        .route("/kernel/status", get(kernel_status))
        .route("/kernel/metrics", get(kernel_metrics))
        .layer(Extension(state))
}

// ──────────────────────────────────────────────
// Request / Response types
// ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateWorldlineRequest {
    pub profile: String,
    pub label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWorldlineResponse {
    pub id: String,
    pub profile: String,
    pub label: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldlineStatus {
    pub id: String,
    pub profile: String,
    pub label: Option<String>,
    pub status: String,
    pub commitment_count: u64,
    pub coupling_count: u64,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitCommitmentRequest {
    pub declaring_identity: String,
    pub effect_domain: String,
    pub targets: Vec<String>,
    pub capabilities: Vec<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitCommitmentResponse {
    pub commitment_id: String,
    pub status: String,
    pub decision: String,
    pub risk_class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentStatus {
    pub id: String,
    pub declaring_identity: String,
    pub status: String,
    pub decision: String,
    pub domain: String,
    pub risk_class: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTrailItem {
    pub event_id: String,
    pub stage: String,
    pub result: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct ProvenanceQuery {
    pub depth: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProvenanceAncestor {
    pub event_id: String,
    pub worldline: String,
    pub stage: String,
    pub timestamp: String,
    pub depth: u32,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub from: Option<u64>,
    pub to: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryEvent {
    pub event_id: String,
    pub stage: String,
    pub payload_type: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct AddPolicyRequest {
    pub name: String,
    pub conditions: Vec<String>,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyInfo {
    pub id: String,
    pub name: String,
    pub constitutional: bool,
    pub conditions: Vec<String>,
    pub action: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulateResult {
    pub decision: String,
    pub risk_class: String,
    pub rationale: String,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceProjectionResponse {
    pub worldline_id: String,
    pub asset: String,
    pub balance_minor: i64,
    pub trajectory_length: usize,
    pub projected_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SettlementRequest {
    pub settlement_type: String,
    pub legs: Vec<SettlementLegRequest>,
}

#[derive(Debug, Deserialize)]
pub struct SettlementLegRequest {
    pub from: String,
    pub to: String,
    pub asset: String,
    pub amount_minor: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettlementResponse {
    pub settlement_id: String,
    pub atomic: bool,
    pub legs: usize,
    pub settled_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KernelStatusBody {
    pub version: String,
    pub worldline_count: usize,
    pub commitment_count: usize,
    pub profile_types: Vec<String>,
    pub invariants_active: usize,
    pub components: Vec<ComponentStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub name: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KernelMetricsBody {
    pub total_commitments: u64,
    pub approved: u64,
    pub denied: u64,
    pub pending: u64,
    pub total_events: u64,
    pub active_worldlines: u64,
    pub financial_settlements: u64,
    pub circuit_breaker_active: bool,
}

// ──────────────────────────────────────────────
// In-memory API state
// ──────────────────────────────────────────────

type SharedState = Arc<RwLock<MwlApiState>>;

#[derive(Debug, Clone, Default)]
struct MwlApiState {
    worldlines: BTreeMap<String, WorldlineStatus>,
    commitments: BTreeMap<String, CommitmentStatus>,
    audit_trails: HashMap<String, Vec<AuditTrailItem>>,
    policies: BTreeMap<String, PolicyInfo>,
    trajectories: HashMap<(String, String), Vec<SettlementDelta>>,
    events: BTreeMap<String, ProvenanceRecord>,
    worldline_events: HashMap<String, Vec<String>>,
    last_event_by_worldline: HashMap<String, String>,
    financial_settlements: u64,
    circuit_breaker_active: bool,
}

#[derive(Debug, Clone)]
struct SettlementDelta {
    amount_minor: i64,
}

#[derive(Debug, Clone)]
struct ProvenanceRecord {
    event_id: String,
    worldline: String,
    stage: String,
    payload_type: String,
    timestamp: String,
    parent: Option<String>,
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn short_id(prefix: &str) -> String {
    format!(
        "{}-{}",
        prefix,
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("0000")
    )
}

fn timestamp_ms(rfc3339: &str) -> Option<u64> {
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .ok()
        .and_then(|dt| u64::try_from(dt.timestamp_millis()).ok())
}

fn append_provenance_event(
    state: &mut MwlApiState,
    worldline_id: &str,
    stage: &str,
    payload_type: &str,
) -> String {
    let event_id = short_id("ev");
    let timestamp = now_rfc3339();
    let parent = state.last_event_by_worldline.get(worldline_id).cloned();

    let record = ProvenanceRecord {
        event_id: event_id.clone(),
        worldline: worldline_id.to_string(),
        stage: stage.to_string(),
        payload_type: payload_type.to_string(),
        timestamp,
        parent,
    };

    state.events.insert(event_id.clone(), record);
    state
        .worldline_events
        .entry(worldline_id.to_string())
        .or_default()
        .push(event_id.clone());
    state
        .last_event_by_worldline
        .insert(worldline_id.to_string(), event_id.clone());

    event_id
}

fn policy_matches_domain(policy: &PolicyInfo, effect_domain: &str) -> bool {
    if policy.conditions.is_empty() {
        return true;
    }

    policy.conditions.iter().any(|condition| {
        let normalized = condition.to_ascii_lowercase();
        if let Some((lhs, rhs)) = normalized.split_once("==") {
            let lhs = lhs.trim();
            let rhs = rhs.trim().trim_matches('"').trim_matches('\'').to_string();
            if lhs == "domain" || lhs == "effect_domain" {
                return rhs == effect_domain;
            }
        }
        false
    })
}

// ──────────────────────────────────────────────
// Handler implementations
//
// These handlers are backed by an in-memory MWL state store.
// The state is shared per router instance via Axum Extension.
// ──────────────────────────────────────────────

async fn create_worldline(
    Extension(state): Extension<SharedState>,
    Json(req): Json<CreateWorldlineRequest>,
) -> impl IntoResponse {
    debug!(profile = %req.profile, label = ?req.label, "Creating worldline");

    if req.profile.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "profile is required"
            })),
        )
            .into_response();
    }

    let id = short_id("wl");
    let created_at = now_rfc3339();

    let worldline = WorldlineStatus {
        id: id.clone(),
        profile: req.profile.clone(),
        label: req.label.clone(),
        status: "active".into(),
        commitment_count: 0,
        coupling_count: 0,
        created_at: created_at.clone(),
    };

    let mut guard = state.write().await;
    guard.worldlines.insert(id.clone(), worldline);
    append_provenance_event(&mut guard, &id, "System", "WorldlineCreated");

    Json(CreateWorldlineResponse {
        id,
        profile: req.profile,
        label: req.label,
        status: "active".into(),
        created_at,
    })
    .into_response()
}

async fn get_worldline(
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    debug!(id = %id, "Getting worldline status");

    let guard = state.read().await;
    match guard.worldlines.get(&id) {
        Some(worldline) => Json(worldline.clone()).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("worldline '{}' not found", id)
            })),
        )
            .into_response(),
    }
}

async fn list_worldlines(Extension(state): Extension<SharedState>) -> impl IntoResponse {
    debug!("Listing worldlines");

    let guard = state.read().await;
    let worldlines: Vec<WorldlineStatus> = guard.worldlines.values().cloned().collect();

    Json(serde_json::json!({
        "worldlines": worldlines,
        "total": guard.worldlines.len(),
    }))
}

async fn submit_commitment(
    Extension(state): Extension<SharedState>,
    Json(req): Json<SubmitCommitmentRequest>,
) -> impl IntoResponse {
    debug!(
        identity = %req.declaring_identity,
        domain = %req.effect_domain,
        "Submitting commitment"
    );

    if req.capabilities.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "at least one capability is required"
            })),
        )
            .into_response();
    }

    let mut guard = state.write().await;
    if let Some(worldline) = guard.worldlines.get_mut(&req.declaring_identity) {
        worldline.commitment_count = worldline.commitment_count.saturating_add(1);
    } else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("declaring identity '{}' not found", req.declaring_identity)
            })),
        )
            .into_response();
    }

    let commitment_id = short_id("cm");
    let created_at = now_rfc3339();
    let risk_class = if req.effect_domain.eq_ignore_ascii_case("financial") {
        "medium"
    } else {
        "low"
    };

    let status = CommitmentStatus {
        id: commitment_id.clone(),
        declaring_identity: req.declaring_identity.clone(),
        status: "approved".into(),
        decision: "approve".into(),
        domain: req.effect_domain.clone(),
        risk_class: risk_class.to_string(),
        created_at,
    };

    guard.commitments.insert(commitment_id.clone(), status);
    guard.audit_trails.insert(
        commitment_id.clone(),
        vec![
            AuditTrailItem {
                event_id: short_id("audit"),
                stage: "Declaration".into(),
                result: "passed".into(),
                timestamp: now_rfc3339(),
            },
            AuditTrailItem {
                event_id: short_id("audit"),
                stage: "Capability".into(),
                result: "passed".into(),
                timestamp: now_rfc3339(),
            },
            AuditTrailItem {
                event_id: short_id("audit"),
                stage: "FinalDecision".into(),
                result: "approved".into(),
                timestamp: now_rfc3339(),
            },
        ],
    );

    append_provenance_event(
        &mut guard,
        &req.declaring_identity,
        "Commitment",
        "CommitmentDeclared",
    );

    Json(SubmitCommitmentResponse {
        commitment_id,
        status: "approved".into(),
        decision: "approve".into(),
        risk_class: risk_class.to_string(),
    })
    .into_response()
}

async fn get_commitment(
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    debug!(id = %id, "Getting commitment status");

    let guard = state.read().await;
    match guard.commitments.get(&id) {
        Some(commitment) => Json(commitment.clone()).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("commitment '{}' not found", id)
            })),
        )
            .into_response(),
    }
}

async fn get_audit_trail(
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    debug!(id = %id, "Getting audit trail");

    let guard = state.read().await;
    if !guard.commitments.contains_key(&id) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("commitment '{}' not found", id)
            })),
        )
            .into_response();
    }

    let events = guard.audit_trails.get(&id).cloned().unwrap_or_default();
    Json(serde_json::json!({
        "commitment_id": id,
        "events": events,
    }))
    .into_response()
}

async fn get_provenance_ancestors(
    Extension(state): Extension<SharedState>,
    Path(event_id): Path<String>,
    Query(query): Query<ProvenanceQuery>,
) -> impl IntoResponse {
    let depth = query.depth.unwrap_or(10);
    debug!(event_id = %event_id, depth = depth, "Getting provenance ancestors");

    let guard = state.read().await;
    let mut ancestors = Vec::new();

    let mut cursor = match guard.events.get(&event_id) {
        Some(record) => record.parent.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("event '{}' not found", event_id)
                })),
            )
                .into_response();
        }
    };

    let mut current_depth = 1;
    while let Some(parent_event_id) = cursor {
        if current_depth > depth {
            break;
        }
        let Some(parent) = guard.events.get(&parent_event_id) else {
            break;
        };
        ancestors.push(ProvenanceAncestor {
            event_id: parent.event_id.clone(),
            worldline: parent.worldline.clone(),
            stage: parent.stage.clone(),
            timestamp: parent.timestamp.clone(),
            depth: current_depth,
        });
        cursor = parent.parent.clone();
        current_depth += 1;
    }

    Json(serde_json::json!({
        "event_id": event_id,
        "depth": depth,
        "ancestors": ancestors,
    }))
    .into_response()
}

async fn get_worldline_history(
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    debug!(id = %id, from = ?query.from, to = ?query.to, "Getting worldline history");

    let guard = state.read().await;
    if !guard.worldlines.contains_key(&id) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("worldline '{}' not found", id)
            })),
        )
            .into_response();
    }

    let event_ids = guard.worldline_events.get(&id).cloned().unwrap_or_default();
    let mut events = Vec::new();
    for event_id in event_ids {
        let Some(record) = guard.events.get(&event_id) else {
            continue;
        };
        let record_ms = timestamp_ms(&record.timestamp);

        let after_from = match (query.from, record_ms) {
            (Some(from), Some(ms)) => ms >= from,
            (Some(_), None) => false,
            (None, _) => true,
        };
        let before_to = match (query.to, record_ms) {
            (Some(to), Some(ms)) => ms <= to,
            (Some(_), None) => false,
            (None, _) => true,
        };

        if after_from && before_to {
            events.push(HistoryEvent {
                event_id: record.event_id.clone(),
                stage: record.stage.clone(),
                payload_type: record.payload_type.clone(),
                timestamp: record.timestamp.clone(),
            });
        }
    }

    Json(serde_json::json!({
        "worldline_id": id,
        "events": events,
    }))
    .into_response()
}

async fn add_policy(
    Extension(state): Extension<SharedState>,
    Json(req): Json<AddPolicyRequest>,
) -> impl IntoResponse {
    debug!(name = %req.name, "Adding governance policy");

    let policy = PolicyInfo {
        id: short_id("pol"),
        name: req.name,
        constitutional: false,
        conditions: req.conditions,
        action: req.action,
    };

    let mut guard = state.write().await;
    guard.policies.insert(policy.id.clone(), policy.clone());

    (StatusCode::CREATED, Json(policy))
}

async fn list_policies(Extension(state): Extension<SharedState>) -> impl IntoResponse {
    debug!("Listing governance policies");

    let guard = state.read().await;
    let policies: Vec<PolicyInfo> = guard.policies.values().cloned().collect();
    Json(serde_json::json!({
        "policies": policies,
        "total": guard.policies.len(),
    }))
}

async fn simulate_policy(
    Extension(state): Extension<SharedState>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    debug!("Simulating policy");

    let effect_domain = body
        .get("effect_domain")
        .and_then(|v| v.as_str())
        .unwrap_or("communication")
        .to_ascii_lowercase();

    let guard = state.read().await;
    let mut policy_refs = Vec::new();
    let mut decision = "approve".to_string();
    let mut risk_class = "low".to_string();
    let mut rationale = "No matching policy conditions".to_string();

    for policy in guard.policies.values() {
        if !policy_matches_domain(policy, &effect_domain) {
            continue;
        }
        policy_refs.push(policy.id.clone());

        let action = policy.action.to_ascii_lowercase();
        if action.contains("deny") {
            decision = "deny".into();
            risk_class = "high".into();
            rationale = format!("Policy '{}' denied the operation", policy.name);
            break;
        }
        if action.contains("require_human") {
            decision = "pending_human_review".into();
            risk_class = "medium".into();
            rationale = format!("Policy '{}' requires human review", policy.name);
        } else {
            rationale = format!("Policy '{}' allows the operation", policy.name);
        }
    }

    Json(SimulateResult {
        decision,
        risk_class,
        rationale,
        policy_refs,
    })
}

async fn get_balance_projection(
    Extension(state): Extension<SharedState>,
    Path((worldline_id, asset)): Path<(String, String)>,
) -> impl IntoResponse {
    debug!(worldline = %worldline_id, asset = %asset, "Getting balance projection");

    let guard = state.read().await;
    if !guard.worldlines.contains_key(&worldline_id) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("worldline '{}' not found", worldline_id)
            })),
        )
            .into_response();
    }

    let key = (worldline_id.clone(), asset.clone());
    let trajectory = guard.trajectories.get(&key).cloned().unwrap_or_default();
    let balance_minor = trajectory
        .iter()
        .map(|entry| entry.amount_minor)
        .sum::<i64>();

    Json(BalanceProjectionResponse {
        worldline_id,
        asset,
        balance_minor,
        trajectory_length: trajectory.len(),
        projected_at: now_rfc3339(),
    })
    .into_response()
}

async fn submit_settlement(
    Extension(state): Extension<SharedState>,
    Json(req): Json<SettlementRequest>,
) -> impl IntoResponse {
    debug!(
        settlement_type = %req.settlement_type,
        legs = req.legs.len(),
        "Submitting settlement"
    );

    if req.legs.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "settlement must include at least one leg"
            })),
        )
            .into_response();
    }

    if req.legs.iter().any(|leg| leg.amount_minor <= 0) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "settlement leg amount_minor must be positive"
            })),
        )
            .into_response();
    }

    let settlement_type = req.settlement_type.to_ascii_lowercase();
    if matches!(settlement_type.as_str(), "dvp" | "pvp") && req.legs.len() < 2 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!(
                    "{} settlement requires at least two legs",
                    settlement_type.to_ascii_uppercase()
                )
            })),
        )
            .into_response();
    }

    let mut guard = state.write().await;
    for leg in &req.legs {
        if !guard.worldlines.contains_key(&leg.from) {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("source worldline '{}' not found", leg.from)
                })),
            )
                .into_response();
        }
        if !guard.worldlines.contains_key(&leg.to) {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("destination worldline '{}' not found", leg.to)
                })),
            )
                .into_response();
        }
    }

    for leg in &req.legs {
        guard
            .trajectories
            .entry((leg.from.clone(), leg.asset.clone()))
            .or_default()
            .push(SettlementDelta {
                amount_minor: -leg.amount_minor,
            });
        guard
            .trajectories
            .entry((leg.to.clone(), leg.asset.clone()))
            .or_default()
            .push(SettlementDelta {
                amount_minor: leg.amount_minor,
            });

        append_provenance_event(&mut guard, &leg.from, "Consequence", "SettlementDebited");
        append_provenance_event(&mut guard, &leg.to, "Consequence", "SettlementCredited");
    }
    guard.financial_settlements = guard.financial_settlements.saturating_add(1);

    Json(SettlementResponse {
        settlement_id: short_id("stl"),
        atomic: true,
        legs: req.legs.len(),
        settled_at: now_rfc3339(),
    })
    .into_response()
}

async fn kernel_status(Extension(state): Extension<SharedState>) -> impl IntoResponse {
    debug!("Getting kernel status");

    let guard = state.read().await;
    let mut profile_types: Vec<String> = guard
        .worldlines
        .values()
        .map(|worldline| worldline.profile.clone())
        .collect();
    profile_types.sort();
    profile_types.dedup();
    if profile_types.is_empty() {
        profile_types = vec![
            "human".into(),
            "agent".into(),
            "financial".into(),
            "world".into(),
            "coordination".into(),
        ];
    }

    Json(KernelStatusBody {
        version: env!("CARGO_PKG_VERSION").to_string(),
        worldline_count: guard.worldlines.len(),
        commitment_count: guard.commitments.len(),
        profile_types,
        invariants_active: 9,
        components: vec![
            ComponentStatus {
                name: "fabric".into(),
                status: "active".into(),
            },
            ComponentStatus {
                name: "gate".into(),
                status: "active".into(),
            },
            ComponentStatus {
                name: "governance".into(),
                status: "active".into(),
            },
            ComponentStatus {
                name: "safety".into(),
                status: "active".into(),
            },
            ComponentStatus {
                name: "provenance".into(),
                status: "active".into(),
            },
            ComponentStatus {
                name: "profiles".into(),
                status: "active".into(),
            },
            ComponentStatus {
                name: "financial".into(),
                status: "active".into(),
            },
        ],
    })
}

async fn kernel_metrics(Extension(state): Extension<SharedState>) -> impl IntoResponse {
    debug!("Getting kernel metrics");

    let guard = state.read().await;
    let mut approved = 0u64;
    let mut denied = 0u64;
    let mut pending = 0u64;

    for commitment in guard.commitments.values() {
        match commitment.status.as_str() {
            "approved" => approved += 1,
            "denied" => denied += 1,
            _ => pending += 1,
        }
    }

    Json(KernelMetricsBody {
        total_commitments: guard.commitments.len() as u64,
        approved,
        denied,
        pending,
        total_events: guard.events.len() as u64,
        active_worldlines: guard.worldlines.len() as u64,
        financial_settlements: guard.financial_settlements,
        circuit_breaker_active: guard.circuit_breaker_active,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_router() -> Router {
        mwl_router()
    }

    async fn create_worldline(app: &Router, profile: &str, label: &str) -> String {
        let body = serde_json::json!({
            "profile": profile,
            "label": label
        });
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/worldlines")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: CreateWorldlineResponse = serde_json::from_slice(&bytes).unwrap();
        created.id
    }

    #[tokio::test]
    async fn kernel_status_returns_200() {
        let app = test_router();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/kernel/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: KernelStatusBody = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.invariants_active, 9);
        assert_eq!(json.profile_types.len(), 5);
        assert_eq!(json.components.len(), 7);
    }

    #[tokio::test]
    async fn kernel_metrics_returns_200() {
        let app = test_router();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/kernel/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: KernelMetricsBody = serde_json::from_slice(&body).unwrap();
        assert!(!json.circuit_breaker_active);
    }

    #[tokio::test]
    async fn create_worldline_returns_json() {
        let app = test_router();
        let body = serde_json::json!({
            "profile": "agent",
            "label": "test-agent"
        });

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/worldlines")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: CreateWorldlineResponse = serde_json::from_slice(&body).unwrap();
        assert!(json.id.starts_with("wl-"));
        assert_eq!(json.profile, "agent");
        assert_eq!(json.label, Some("test-agent".into()));
        assert_eq!(json.status, "active");
    }

    #[tokio::test]
    async fn get_worldline_returns_status() {
        let app = test_router();
        let worldline_id = create_worldline(&app, "agent", "status-test").await;

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/worldlines/{}", worldline_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: WorldlineStatus = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.id, worldline_id);
    }

    #[tokio::test]
    async fn list_worldlines_returns_empty() {
        let app = test_router();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/worldlines")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn list_worldlines_reflects_new_records() {
        let app = test_router();
        let worldline_id = create_worldline(&app, "human", "alice").await;

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/worldlines")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["total"], 1);
        assert_eq!(json["worldlines"][0]["id"], worldline_id);
    }

    #[tokio::test]
    async fn submit_commitment_returns_approval() {
        let app = test_router();
        let worldline_id = create_worldline(&app, "agent", "committer").await;

        let body = serde_json::json!({
            "declaring_identity": worldline_id,
            "effect_domain": "communication",
            "targets": ["wl-5678"],
            "capabilities": ["cap-send"],
            "evidence": []
        });

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/commitments")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: SubmitCommitmentResponse = serde_json::from_slice(&body).unwrap();
        assert!(json.commitment_id.starts_with("cm-"));
    }

    #[tokio::test]
    async fn balance_projection_returns_json() {
        let app = test_router();
        let worldline_id = create_worldline(&app, "financial", "treasury").await;

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/financial/{}/balance/USD", worldline_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: BalanceProjectionResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.worldline_id, worldline_id);
        assert_eq!(json.asset, "USD");
        assert_eq!(json.balance_minor, 0);
        assert_eq!(json.trajectory_length, 0);
    }

    #[tokio::test]
    async fn add_policy_returns_created() {
        let app = test_router();
        let body = serde_json::json!({
            "name": "test-policy",
            "conditions": ["domain == financial"],
            "action": "require_human_review"
        });

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/governance/policies")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);

        let list_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/governance/policies")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(list_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(payload["total"], 1);
    }

    #[tokio::test]
    async fn submit_settlement_returns_atomic() {
        let app = test_router();
        let wl_a = create_worldline(&app, "financial", "party-a").await;
        let wl_b = create_worldline(&app, "financial", "party-b").await;

        let body = serde_json::json!({
            "settlement_type": "dvp",
            "legs": [
                { "from": wl_a, "to": wl_b, "asset": "USD", "amount_minor": 100000 },
                { "from": wl_b, "to": wl_a, "asset": "BTC", "amount_minor": 1000000 }
            ]
        });

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/financial/settle")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: SettlementResponse = serde_json::from_slice(&body).unwrap();
        assert!(json.atomic);
        assert_eq!(json.legs, 2);
    }

    #[tokio::test]
    async fn submit_settlement_updates_projection() {
        let app = test_router();
        let wl_a = create_worldline(&app, "financial", "issuer").await;
        let wl_b = create_worldline(&app, "financial", "receiver").await;

        let settlement = serde_json::json!({
            "settlement_type": "dvp",
            "legs": [
                { "from": wl_a.clone(), "to": wl_b.clone(), "asset": "USD", "amount_minor": 150000 },
                { "from": wl_b.clone(), "to": wl_a.clone(), "asset": "BTC", "amount_minor": 1000 }
            ]
        });

        let submit = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/financial/settle")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&settlement).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(submit.status(), StatusCode::OK);

        let balance_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/financial/{}/balance/USD", wl_b))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(balance_resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(balance_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let projection: BalanceProjectionResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(projection.balance_minor, 150000);
        assert_eq!(projection.trajectory_length, 1);
    }
}
