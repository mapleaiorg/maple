//! Instance management handlers

use crate::api::rest::state::AppState;
use crate::error::{ApiError, ApiResult};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use palm_types::{instance::AgentInstance, DeploymentId, InstanceId};
use serde::{Deserialize, Serialize};

/// List instances query params
#[derive(Debug, Deserialize)]
pub struct ListInstancesQuery {
    pub deployment_id: Option<String>,
}

/// List instances
pub async fn list_instances(
    State(state): State<AppState>,
    Query(query): Query<ListInstancesQuery>,
) -> ApiResult<Json<Vec<AgentInstance>>> {
    let instances = if let Some(deployment_id) = query.deployment_id {
        let dep_id = parse_deployment_id(&deployment_id)?;
        state.storage.list_instances_for_deployment(&dep_id).await?
    } else {
        state.storage.list_instances().await?
    };

    Ok(Json(instances))
}

/// Get a specific instance
pub async fn get_instance(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<AgentInstance>> {
    let instance_id = parse_instance_id(&id)?;
    let instance = state
        .storage
        .get_instance(&instance_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Instance {} not found", id)))?;

    Ok(Json(instance))
}

/// Delete an instance
pub async fn delete_instance(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteInstanceResponse>> {
    let instance_id = parse_instance_id(&id)?;

    let deleted = state.storage.delete_instance(&instance_id).await?;

    if deleted {
        tracing::info!(instance_id = %id, "Deleted instance");
        // Trigger reconciliation to potentially create replacement
        state.scheduler.trigger_reconcile().await;
    }

    Ok(Json(DeleteInstanceResponse { deleted }))
}

/// Delete instance response
#[derive(Debug, Serialize)]
pub struct DeleteInstanceResponse {
    pub deleted: bool,
}

/// Instance health response
#[derive(Debug, Serialize)]
pub struct InstanceHealthResponse {
    pub instance_id: String,
    pub status: String,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub probes: Vec<ProbeResult>,
}

/// Probe result
#[derive(Debug, Serialize)]
pub struct ProbeResult {
    pub name: String,
    pub passed: bool,
    pub details: Option<String>,
}

/// Get instance health
pub async fn get_instance_health(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<InstanceHealthResponse>> {
    let instance_id = parse_instance_id(&id)?;
    let instance = state
        .storage
        .get_instance(&instance_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Instance {} not found", id)))?;

    let status = match &instance.health {
        palm_types::instance::HealthStatus::Healthy => "healthy".to_string(),
        palm_types::instance::HealthStatus::Degraded { factors } => {
            format!("degraded: {}", factors.join(", "))
        }
        palm_types::instance::HealthStatus::Unhealthy { reasons } => {
            format!("unhealthy: {}", reasons.join(", "))
        }
        palm_types::instance::HealthStatus::Unknown => "unknown".to_string(),
    };

    // Mock probes for now
    let probes = vec![
        ProbeResult {
            name: "liveness".to_string(),
            passed: instance.health.is_operational(),
            details: None,
        },
        ProbeResult {
            name: "readiness".to_string(),
            passed: instance.health.is_healthy(),
            details: None,
        },
    ];

    Ok(Json(InstanceHealthResponse {
        instance_id: id,
        status,
        last_check: instance.last_heartbeat,
        probes,
    }))
}

/// Create checkpoint response
#[derive(Debug, Serialize)]
pub struct CheckpointResponse {
    pub snapshot_id: String,
    pub created: bool,
}

/// Create a checkpoint for an instance
pub async fn create_checkpoint(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<CheckpointResponse>> {
    let instance_id = parse_instance_id(&id)?;

    // Verify instance exists
    if state.storage.get_instance(&instance_id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Instance {} not found", id)));
    }

    let snapshot_id = state
        .storage
        .create_snapshot(&instance_id, "manual_checkpoint")
        .await?;

    tracing::info!(instance_id = %id, snapshot_id = %snapshot_id, "Created checkpoint");

    Ok(Json(CheckpointResponse {
        snapshot_id,
        created: true,
    }))
}

/// Snapshot info response
#[derive(Debug, Serialize)]
pub struct SnapshotInfoResponse {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub reason: String,
    pub size_bytes: u64,
}

/// List snapshots for an instance
pub async fn list_snapshots(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<SnapshotInfoResponse>>> {
    let instance_id = parse_instance_id(&id)?;

    let snapshots = state.storage.list_snapshots(&instance_id).await?;

    let responses: Vec<_> = snapshots
        .into_iter()
        .map(|s| SnapshotInfoResponse {
            id: s.id,
            created_at: s.created_at,
            reason: s.reason,
            size_bytes: s.size_bytes,
        })
        .collect();

    Ok(Json(responses))
}

/// Restore snapshot request
#[derive(Debug, Deserialize)]
pub struct RestoreSnapshotRequest {
    pub snapshot_id: String,
}

/// Restore snapshot response
#[derive(Debug, Serialize)]
pub struct RestoreSnapshotResponse {
    pub restored: bool,
}

/// Restore an instance from a snapshot
pub async fn restore_snapshot(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<RestoreSnapshotRequest>,
) -> ApiResult<Json<RestoreSnapshotResponse>> {
    let instance_id = parse_instance_id(&id)?;

    // Verify instance exists
    if state.storage.get_instance(&instance_id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Instance {} not found", id)));
    }

    state
        .storage
        .restore_snapshot(&instance_id, &request.snapshot_id)
        .await?;

    tracing::info!(
        instance_id = %id,
        snapshot_id = %request.snapshot_id,
        "Restored from snapshot"
    );

    Ok(Json(RestoreSnapshotResponse { restored: true }))
}

/// Helper to parse instance ID from string (UUID-based)
fn parse_instance_id(id: &str) -> ApiResult<InstanceId> {
    let uuid_str = id.strip_prefix("instance:").unwrap_or(id);
    let uuid = uuid::Uuid::parse_str(uuid_str)
        .map_err(|_| ApiError::BadRequest(format!("Invalid instance ID: {}", id)))?;
    Ok(InstanceId::from_uuid(uuid))
}

/// Helper to parse deployment ID from string (UUID-based)
fn parse_deployment_id(id: &str) -> ApiResult<DeploymentId> {
    let uuid_str = id.strip_prefix("deploy:").unwrap_or(id);
    let uuid = uuid::Uuid::parse_str(uuid_str)
        .map_err(|_| ApiError::BadRequest(format!("Invalid deployment ID: {}", id)))?;
    Ok(DeploymentId::from_uuid(uuid))
}
