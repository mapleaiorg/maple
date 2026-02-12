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
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

// ──────────────────────────────────────────────
// Router
// ──────────────────────────────────────────────

/// Create the MWL API router.
///
/// All routes are relative — designed to be nested under `/api/v1/`
/// in the PALM daemon.
pub fn mwl_router() -> Router {
    Router::new()
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitmentStatus {
    pub id: String,
    pub declaring_identity: String,
    pub status: String,
    pub decision: String,
    pub domain: String,
    pub risk_class: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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
// Handler implementations
//
// These are placeholder handlers that return well-formed JSON
// responses. In production, they would be backed by the actual
// kernel state (passed via Axum state extractor).
// ──────────────────────────────────────────────

async fn create_worldline(
    Json(req): Json<CreateWorldlineRequest>,
) -> impl IntoResponse {
    debug!(profile = %req.profile, label = ?req.label, "Creating worldline");

    let id = format!("wl-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0000"));

    Json(CreateWorldlineResponse {
        id,
        profile: req.profile,
        label: req.label,
        status: "active".into(),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn get_worldline(Path(id): Path<String>) -> impl IntoResponse {
    debug!(id = %id, "Getting worldline status");

    Json(WorldlineStatus {
        id: id.clone(),
        profile: "agent".into(),
        label: None,
        status: "active".into(),
        commitment_count: 0,
        coupling_count: 0,
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn list_worldlines() -> impl IntoResponse {
    debug!("Listing worldlines");
    Json(serde_json::json!({
        "worldlines": [],
        "total": 0,
    }))
}

async fn submit_commitment(
    Json(req): Json<SubmitCommitmentRequest>,
) -> impl IntoResponse {
    debug!(
        identity = %req.declaring_identity,
        domain = %req.effect_domain,
        "Submitting commitment"
    );

    let commitment_id = format!(
        "cm-{}",
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0000")
    );

    Json(SubmitCommitmentResponse {
        commitment_id,
        status: "approved".into(),
        decision: "approve".into(),
        risk_class: "low".into(),
    })
}

async fn get_commitment(Path(id): Path<String>) -> impl IntoResponse {
    debug!(id = %id, "Getting commitment status");

    Json(CommitmentStatus {
        id: id.clone(),
        declaring_identity: "unknown".into(),
        status: "approved".into(),
        decision: "approve".into(),
        domain: "communication".into(),
        risk_class: "low".into(),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn get_audit_trail(Path(id): Path<String>) -> impl IntoResponse {
    debug!(id = %id, "Getting audit trail");
    Json(serde_json::json!({
        "commitment_id": id,
        "events": [],
    }))
}

async fn get_provenance_ancestors(
    Path(event_id): Path<String>,
    Query(query): Query<ProvenanceQuery>,
) -> impl IntoResponse {
    let depth = query.depth.unwrap_or(10);
    debug!(event_id = %event_id, depth = depth, "Getting provenance ancestors");
    Json(serde_json::json!({
        "event_id": event_id,
        "depth": depth,
        "ancestors": [],
    }))
}

async fn get_worldline_history(
    Path(id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    debug!(id = %id, from = ?query.from, to = ?query.to, "Getting worldline history");
    Json(serde_json::json!({
        "worldline_id": id,
        "events": [],
    }))
}

async fn add_policy(Json(req): Json<AddPolicyRequest>) -> impl IntoResponse {
    debug!(name = %req.name, "Adding governance policy");

    let id = format!(
        "pol-{}",
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0000")
    );

    (
        StatusCode::CREATED,
        Json(PolicyInfo {
            id,
            name: req.name,
            constitutional: false,
            conditions: req.conditions,
            action: req.action,
        }),
    )
}

async fn list_policies() -> impl IntoResponse {
    debug!("Listing governance policies");
    Json(serde_json::json!({
        "policies": [],
        "total": 0,
    }))
}

async fn simulate_policy(Json(_body): Json<serde_json::Value>) -> impl IntoResponse {
    debug!("Simulating policy");
    Json(SimulateResult {
        decision: "approve".into(),
        risk_class: "low".into(),
        rationale: "All policies passed (simulation)".into(),
        policy_refs: vec![],
    })
}

async fn get_balance_projection(
    Path((worldline_id, asset)): Path<(String, String)>,
) -> impl IntoResponse {
    debug!(worldline = %worldline_id, asset = %asset, "Getting balance projection");

    Json(BalanceProjectionResponse {
        worldline_id,
        asset,
        balance_minor: 0,
        trajectory_length: 0,
        projected_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn submit_settlement(Json(req): Json<SettlementRequest>) -> impl IntoResponse {
    debug!(
        settlement_type = %req.settlement_type,
        legs = req.legs.len(),
        "Submitting settlement"
    );

    Json(SettlementResponse {
        settlement_id: format!(
            "stl-{}",
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0000")
        ),
        atomic: true,
        legs: req.legs.len(),
        settled_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn kernel_status() -> impl IntoResponse {
    debug!("Getting kernel status");

    Json(KernelStatusBody {
        version: env!("CARGO_PKG_VERSION").to_string(),
        worldline_count: 0,
        commitment_count: 0,
        profile_types: vec![
            "human".into(),
            "agent".into(),
            "financial".into(),
            "world".into(),
            "coordination".into(),
        ],
        invariants_active: 8,
        components: vec![
            ComponentStatus { name: "fabric".into(), status: "active".into() },
            ComponentStatus { name: "gate".into(), status: "active".into() },
            ComponentStatus { name: "governance".into(), status: "active".into() },
            ComponentStatus { name: "safety".into(), status: "active".into() },
            ComponentStatus { name: "provenance".into(), status: "active".into() },
            ComponentStatus { name: "profiles".into(), status: "active".into() },
            ComponentStatus { name: "financial".into(), status: "active".into() },
        ],
    })
}

async fn kernel_metrics() -> impl IntoResponse {
    debug!("Getting kernel metrics");

    Json(KernelMetricsBody {
        total_commitments: 0,
        approved: 0,
        denied: 0,
        pending: 0,
        total_events: 0,
        active_worldlines: 0,
        financial_settlements: 0,
        circuit_breaker_active: false,
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
        assert_eq!(json.invariants_active, 8);
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
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/worldlines/wl-1234")
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
        assert_eq!(json.id, "wl-1234");
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
    async fn submit_commitment_returns_approval() {
        let app = test_router();
        let body = serde_json::json!({
            "declaring_identity": "wl-1234",
            "effect_domain": "communication",
            "targets": ["wl-5678"],
            "capabilities": ["cap-send"],
            "evidence": []
        });

        let resp = app
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
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/financial/wl-1234/balance/USD")
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
        assert_eq!(json.worldline_id, "wl-1234");
        assert_eq!(json.asset, "USD");
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
    }

    #[tokio::test]
    async fn submit_settlement_returns_atomic() {
        let app = test_router();
        let body = serde_json::json!({
            "settlement_type": "dvp",
            "legs": [
                { "from": "wl-a", "to": "wl-b", "asset": "USD", "amount_minor": 100000 },
                { "from": "wl-b", "to": "wl-a", "asset": "BTC", "amount_minor": 1000000 }
            ]
        });

        let resp = app
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
}
