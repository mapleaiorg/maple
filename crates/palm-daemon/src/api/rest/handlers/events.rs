//! Event streaming handlers

use crate::api::rest::state::AppState;
use crate::error::{ApiError, ApiResult};
use crate::storage::EventStorage;
use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures_util::stream::{self, Stream};
use palm_types::{DeploymentId, InstanceId, PalmEventEnvelope};
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;

/// Get events query params
#[derive(Debug, Deserialize)]
pub struct GetEventsQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub deployment_id: Option<String>,
    pub instance_id: Option<String>,
}

fn default_limit() -> usize {
    20
}

/// Get recent events
pub async fn get_events(
    State(state): State<AppState>,
    Query(query): Query<GetEventsQuery>,
) -> ApiResult<Json<Vec<PalmEventEnvelope>>> {
    let events = if let Some(deployment_id) = query.deployment_id {
        let dep_id = parse_deployment_id(&deployment_id)?;
        state
            .storage
            .get_events_for_deployment(&dep_id, query.limit)
            .await?
    } else if let Some(instance_id) = query.instance_id {
        let inst_id = parse_instance_id(&instance_id)?;
        state
            .storage
            .get_events_for_instance(&inst_id, query.limit)
            .await?
    } else {
        state.storage.get_recent_events(query.limit).await?
    };

    Ok(Json(events))
}

/// Stream events via SSE
pub async fn stream_events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.event_tx.subscribe();

    let stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(event) => {
                let json = serde_json::to_string(&event).unwrap_or_default();
                let sse_event = Event::default().data(json);
                Some((Ok(sse_event), rx))
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                // Client lagged behind, continue
                Some((Ok(Event::default().comment("lagged")), rx))
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => None,
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}

/// Helper to parse deployment ID from string (UUID-based)
fn parse_deployment_id(id: &str) -> ApiResult<DeploymentId> {
    let uuid_str = id.strip_prefix("deploy:").unwrap_or(id);
    let uuid = uuid::Uuid::parse_str(uuid_str)
        .map_err(|_| ApiError::BadRequest(format!("Invalid deployment ID: {}", id)))?;
    Ok(DeploymentId::from_uuid(uuid))
}

/// Helper to parse instance ID from string (UUID-based)
fn parse_instance_id(id: &str) -> ApiResult<InstanceId> {
    let uuid_str = id.strip_prefix("instance:").unwrap_or(id);
    let uuid = uuid::Uuid::parse_str(uuid_str)
        .map_err(|_| ApiError::BadRequest(format!("Invalid instance ID: {}", id)))?;
    Ok(InstanceId::from_uuid(uuid))
}
