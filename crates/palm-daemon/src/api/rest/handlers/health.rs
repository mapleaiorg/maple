//! Health and status handlers

use crate::api::rest::state::AppState;
use crate::error::ApiResult;
use crate::storage::{DeploymentStorage, InstanceStorage, SpecStorage};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use palm_types::instance::HealthStatus;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub version: String,
    pub uptime: String,
}

/// Health check endpoint
pub async fn health_check(State(state): State<AppState>) -> Json<HealthCheckResponse> {
    Json(HealthCheckResponse {
        status: "healthy".to_string(),
        version: state.version.clone(),
        uptime: state.uptime(),
    })
}

/// Daemon status response
#[derive(Debug, Serialize)]
pub struct DaemonStatusResponse {
    pub status: String,
    pub version: String,
    pub uptime: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub stats: DaemonStats,
}

/// Daemon statistics
#[derive(Debug, Serialize)]
pub struct DaemonStats {
    pub total_specs: usize,
    pub total_deployments: usize,
    pub total_instances: usize,
    pub healthy_instances: usize,
    pub unhealthy_instances: usize,
}

/// Daemon status endpoint
pub async fn daemon_status(State(state): State<AppState>) -> ApiResult<Json<DaemonStatusResponse>> {
    let specs = state.storage.list_specs().await?;
    let deployments = state.storage.list_deployments().await?;
    let instances = state.storage.list_instances().await?;

    let healthy = instances.iter().filter(|i| i.health.is_healthy()).count();
    let unhealthy = instances.iter().filter(|i| matches!(i.health, HealthStatus::Unhealthy { .. })).count();

    Ok(Json(DaemonStatusResponse {
        status: "healthy".to_string(),
        version: state.version.clone(),
        uptime: state.uptime(),
        started_at: state.started_at,
        stats: DaemonStats {
            total_specs: specs.len(),
            total_deployments: deployments.len(),
            total_instances: instances.len(),
            healthy_instances: healthy,
            unhealthy_instances: unhealthy,
        },
    }))
}

/// Health summary response
#[derive(Debug, Serialize)]
pub struct HealthSummaryResponse {
    pub total: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub unhealthy: usize,
    pub unknown: usize,
}

/// Get fleet health summary
pub async fn health_summary(State(state): State<AppState>) -> ApiResult<Json<HealthSummaryResponse>> {
    let instances = state.storage.list_instances().await?;

    let mut healthy = 0;
    let mut degraded = 0;
    let mut unhealthy = 0;
    let mut unknown = 0;

    for instance in &instances {
        match &instance.health {
            HealthStatus::Healthy => healthy += 1,
            HealthStatus::Degraded { .. } => degraded += 1,
            HealthStatus::Unhealthy { .. } => unhealthy += 1,
            HealthStatus::Unknown => unknown += 1,
        }
    }

    Ok(Json(HealthSummaryResponse {
        total: instances.len(),
        healthy,
        degraded,
        unhealthy,
        unknown,
    }))
}

/// List unhealthy instances
pub async fn list_unhealthy(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<UnhealthyInstanceInfo>>> {
    let instances = state.storage.list_unhealthy_instances().await?;

    let infos: Vec<_> = instances
        .into_iter()
        .map(|i| UnhealthyInstanceInfo {
            id: i.id.to_string(),
            deployment_id: i.deployment_id.to_string(),
            health: format!("{:?}", i.health),
            last_heartbeat: i.last_heartbeat,
        })
        .collect();

    Ok(Json(infos))
}

/// Unhealthy instance info
#[derive(Debug, Serialize, Deserialize)]
pub struct UnhealthyInstanceInfo {
    pub id: String,
    pub deployment_id: String,
    pub health: String,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}
