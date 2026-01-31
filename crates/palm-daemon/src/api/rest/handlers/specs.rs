//! Spec management handlers

use crate::api::rest::state::AppState;
use crate::error::{ApiError, ApiResult};
use crate::storage::{DeploymentStorage, SpecStorage};
use axum::{
    extract::{Path, State},
    Json,
};
use palm_types::{AgentSpec, AgentSpecId};
use serde::{Deserialize, Serialize};

/// Create spec request
#[derive(Debug, Deserialize)]
pub struct CreateSpecRequest {
    pub spec: AgentSpec,
}

/// Create spec response
#[derive(Debug, Serialize)]
pub struct CreateSpecResponse {
    pub id: String,
    pub created: bool,
}

/// List all specs
pub async fn list_specs(State(state): State<AppState>) -> ApiResult<Json<Vec<AgentSpec>>> {
    let specs = state.storage.list_specs().await?;
    Ok(Json(specs))
}

/// Get a specific spec
pub async fn get_spec(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<AgentSpec>> {
    let spec_id = AgentSpecId::new(&id);
    let spec = state
        .storage
        .get_spec(&spec_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Spec {} not found", id)))?;

    Ok(Json(spec))
}

/// Create a new spec
pub async fn create_spec(
    State(state): State<AppState>,
    Json(request): Json<CreateSpecRequest>,
) -> ApiResult<Json<CreateSpecResponse>> {
    // Check if spec already exists
    if state.storage.get_spec(&request.spec.id).await?.is_some() {
        return Err(ApiError::Conflict(format!(
            "Spec {} already exists",
            request.spec.id
        )));
    }

    state.storage.upsert_spec(request.spec.clone()).await?;

    tracing::info!(spec_id = %request.spec.id, "Created spec");

    Ok(Json(CreateSpecResponse {
        id: request.spec.id.to_string(),
        created: true,
    }))
}

/// Update spec request
#[derive(Debug, Deserialize)]
pub struct UpdateSpecRequest {
    pub spec: AgentSpec,
}

/// Update an existing spec
pub async fn update_spec(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateSpecRequest>,
) -> ApiResult<Json<AgentSpec>> {
    let spec_id = AgentSpecId::new(&id);

    // Check spec exists
    if state.storage.get_spec(&spec_id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Spec {} not found", id)));
    }

    // Ensure ID matches
    if request.spec.id != spec_id {
        return Err(ApiError::BadRequest(
            "Spec ID in body does not match path".to_string(),
        ));
    }

    state.storage.upsert_spec(request.spec.clone()).await?;

    tracing::info!(spec_id = %id, "Updated spec");

    Ok(Json(request.spec))
}

/// Delete a spec
pub async fn delete_spec(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteResponse>> {
    let spec_id = AgentSpecId::new(&id);

    // Check if there are active deployments using this spec
    let deployments = state.storage.list_deployments_for_spec(&spec_id).await?;
    if !deployments.is_empty() {
        return Err(ApiError::Conflict(format!(
            "Cannot delete spec {} with {} active deployments",
            id,
            deployments.len()
        )));
    }

    let deleted = state.storage.delete_spec(&spec_id).await?;

    if deleted {
        tracing::info!(spec_id = %id, "Deleted spec");
    }

    Ok(Json(DeleteResponse { deleted }))
}

/// Delete response
#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub deleted: bool,
}
