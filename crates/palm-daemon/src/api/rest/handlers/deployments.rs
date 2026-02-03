//! Deployment management handlers

use crate::api::rest::state::AppState;
use crate::error::{ApiError, ApiResult};
use axum::{
    extract::{Path, State},
    Json,
};
use palm_types::{
    AgentSpecId, Deployment, DeploymentId, DeploymentStatus, DeploymentStrategy, PlatformProfile,
    ReplicaConfig,
};
use serde::{Deserialize, Serialize};

/// Create deployment request
#[derive(Debug, Deserialize)]
pub struct CreateDeploymentRequest {
    pub spec_id: String,
    #[serde(default = "default_replicas")]
    pub replicas: u32,
    #[serde(default)]
    pub platform: Option<PlatformProfile>,
}

fn default_replicas() -> u32 {
    3
}

/// Create deployment response
#[derive(Debug, Serialize)]
pub struct CreateDeploymentResponse {
    pub id: String,
    pub created: bool,
}

/// List all deployments
pub async fn list_deployments(State(state): State<AppState>) -> ApiResult<Json<Vec<Deployment>>> {
    let deployments = state.storage.list_deployments().await?;
    Ok(Json(deployments))
}

/// Get a specific deployment
pub async fn get_deployment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<DeploymentWithInstances>> {
    // Parse the deployment ID - it's UUID-based
    let deployment_id = parse_deployment_id(&id)?;
    let deployment = state
        .storage
        .get_deployment(&deployment_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Deployment {} not found", id)))?;

    let instances = state
        .storage
        .list_instances_for_deployment(&deployment_id)
        .await?;

    Ok(Json(DeploymentWithInstances {
        deployment,
        instance_count: instances.len(),
        ready_count: instances
            .iter()
            .filter(|i| i.health.is_operational())
            .count(),
    }))
}

/// Deployment with instance info
#[derive(Debug, Serialize)]
pub struct DeploymentWithInstances {
    #[serde(flatten)]
    pub deployment: Deployment,
    pub instance_count: usize,
    pub ready_count: usize,
}

/// Create a new deployment
pub async fn create_deployment(
    State(state): State<AppState>,
    Json(request): Json<CreateDeploymentRequest>,
) -> ApiResult<Json<CreateDeploymentResponse>> {
    let spec_id = AgentSpecId::new(&request.spec_id);

    // Verify spec exists
    let spec = state
        .storage
        .get_spec(&spec_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Spec {} not found", request.spec_id)))?;

    let deployment_id = DeploymentId::generate();
    let platform = request.platform.unwrap_or(PlatformProfile::Development);

    let deployment = Deployment {
        id: deployment_id.clone(),
        agent_spec_id: spec_id,
        version: spec.version.clone(),
        platform,
        strategy: DeploymentStrategy::default(),
        status: DeploymentStatus::Pending,
        replicas: ReplicaConfig::new(request.replicas),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    state.storage.upsert_deployment(deployment).await?;

    // Trigger scheduler to reconcile
    state.scheduler.trigger_reconcile().await;

    tracing::info!(
        deployment_id = %deployment_id,
        spec_id = %request.spec_id,
        "Created deployment"
    );

    Ok(Json(CreateDeploymentResponse {
        id: deployment_id.to_string(),
        created: true,
    }))
}

/// Update deployment request
#[derive(Debug, Deserialize)]
pub struct UpdateDeploymentRequest {
    #[serde(default)]
    pub replicas: Option<u32>,
    #[serde(default)]
    pub spec_id: Option<String>,
}

/// Update an existing deployment
pub async fn update_deployment(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateDeploymentRequest>,
) -> ApiResult<Json<Deployment>> {
    let deployment_id = parse_deployment_id(&id)?;

    let mut deployment = state
        .storage
        .get_deployment(&deployment_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Deployment {} not found", id)))?;

    // Update replicas if provided
    if let Some(replicas) = request.replicas {
        deployment.replicas.desired = replicas;
    }

    // Update spec if provided
    if let Some(spec_id_str) = request.spec_id {
        let spec_id = AgentSpecId::new(&spec_id_str);
        let spec = state
            .storage
            .get_spec(&spec_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Spec {} not found", spec_id_str)))?;
        deployment.agent_spec_id = spec_id;
        deployment.version = spec.version;
    }

    deployment.updated_at = chrono::Utc::now();
    state.storage.upsert_deployment(deployment.clone()).await?;

    // Trigger reconciliation
    state.scheduler.trigger_reconcile().await;

    tracing::info!(deployment_id = %id, "Updated deployment");

    Ok(Json(deployment))
}

/// Delete a deployment
pub async fn delete_deployment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteDeploymentResponse>> {
    let deployment_id = parse_deployment_id(&id)?;

    // Delete all instances
    let instances = state
        .storage
        .list_instances_for_deployment(&deployment_id)
        .await?;
    for instance in instances {
        state.storage.delete_instance(&instance.id).await?;
    }

    // Delete deployment
    let deleted = state.storage.delete_deployment(&deployment_id).await?;

    if deleted {
        tracing::info!(deployment_id = %id, "Deleted deployment");
    }

    Ok(Json(DeleteDeploymentResponse { deleted }))
}

/// Delete deployment response
#[derive(Debug, Serialize)]
pub struct DeleteDeploymentResponse {
    pub deleted: bool,
}

/// Scale deployment request
#[derive(Debug, Deserialize)]
pub struct ScaleDeploymentRequest {
    pub replicas: u32,
}

/// Scale a deployment
pub async fn scale_deployment(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<ScaleDeploymentRequest>,
) -> ApiResult<Json<Deployment>> {
    let deployment_id = parse_deployment_id(&id)?;

    let mut deployment = state
        .storage
        .get_deployment(&deployment_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Deployment {} not found", id)))?;

    deployment.replicas.desired = request.replicas;
    deployment.updated_at = chrono::Utc::now();

    state.storage.upsert_deployment(deployment.clone()).await?;

    // Trigger reconciliation
    state.scheduler.trigger_reconcile().await;

    tracing::info!(
        deployment_id = %id,
        replicas = request.replicas,
        "Scaled deployment"
    );

    Ok(Json(deployment))
}

/// Pause a deployment
pub async fn pause_deployment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Deployment>> {
    let deployment_id = parse_deployment_id(&id)?;

    let mut deployment = state
        .storage
        .get_deployment(&deployment_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Deployment {} not found", id)))?;

    deployment.status = DeploymentStatus::Paused {
        reason: "Manual pause".to_string(),
        paused_at: chrono::Utc::now(),
    };
    deployment.updated_at = chrono::Utc::now();

    state.storage.upsert_deployment(deployment.clone()).await?;

    tracing::info!(deployment_id = %id, "Paused deployment");

    Ok(Json(deployment))
}

/// Resume a deployment
pub async fn resume_deployment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Deployment>> {
    let deployment_id = parse_deployment_id(&id)?;

    let mut deployment = state
        .storage
        .get_deployment(&deployment_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Deployment {} not found", id)))?;

    if !matches!(deployment.status, DeploymentStatus::Paused { .. }) {
        return Err(ApiError::BadRequest(format!(
            "Deployment {} is not paused",
            id
        )));
    }

    deployment.status = DeploymentStatus::InProgress {
        progress: 0,
        phase: "Resuming".to_string(),
    };
    deployment.updated_at = chrono::Utc::now();

    state.storage.upsert_deployment(deployment.clone()).await?;

    // Trigger reconciliation
    state.scheduler.trigger_reconcile().await;

    tracing::info!(deployment_id = %id, "Resumed deployment");

    Ok(Json(deployment))
}

/// Helper to parse deployment ID from string (UUID-based)
fn parse_deployment_id(id: &str) -> ApiResult<DeploymentId> {
    // Strip the "deploy:" prefix if present
    let uuid_str = id.strip_prefix("deploy:").unwrap_or(id);
    let uuid = uuid::Uuid::parse_str(uuid_str)
        .map_err(|_| ApiError::BadRequest(format!("Invalid deployment ID: {}", id)))?;
    Ok(DeploymentId::from_uuid(uuid))
}
