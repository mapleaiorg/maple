//! Playground API handlers

use crate::api::rest::state::AppState;
use crate::error::ApiResult;
use crate::storage::{ActivityStorage, InstanceStorage, ResonatorStorage};
use axum::{
    extract::{Query, State},
    response::{Html, sse::{Event, KeepAlive, Sse}},
    Json,
};
use futures_util::stream::{self, Stream};
use palm_shared_state::{
    Activity, PlaygroundConfigPublic, PlaygroundConfigUpdate, SystemState, SystemStats,
};
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;

const PLAYGROUND_HTML: &str = include_str!("../../../../assets/playground.html");

/// Serve the playground dashboard HTML
pub async fn playground_index() -> Html<&'static str> {
    Html(PLAYGROUND_HTML)
}

#[derive(Debug, Deserialize)]
pub struct ActivitiesQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub after_sequence: Option<u64>,
}

fn default_limit() -> usize {
    200
}

/// Aggregated playground state
pub async fn playground_state(
    State(state): State<AppState>,
) -> ApiResult<Json<SystemState>> {
    let config = state.playground.config_public().await;
    let backends = state.playground.backend_catalog().await;
    let agents = state.storage.list_instances().await?;
    let resonators = state.storage.list_resonators().await?;
    let activities = state.storage.list_activities(200, None).await?;

    let agents_total = agents.len();
    let agents_healthy = agents.iter().filter(|a| a.health.is_healthy()).count();
    let resonators_total = resonators.len();
    let active_couplings = resonators.iter().map(|r| r.couplings.len()).sum();
    let last_activity_at = activities.iter().map(|a| a.timestamp).max();

    let stats = SystemStats {
        agents_total,
        agents_healthy,
        resonators_total,
        activities_total: activities.len(),
        active_couplings,
        last_activity_at,
    };

    Ok(Json(SystemState {
        generated_at: chrono::Utc::now(),
        playground: config,
        backends,
        stats,
        agents,
        resonators,
        activities,
    }))
}

/// Get playground configuration (public)
pub async fn get_playground_config(
    State(state): State<AppState>,
) -> ApiResult<Json<PlaygroundConfigPublic>> {
    let config = state.playground.config_public().await;
    Ok(Json(config))
}

/// Update playground configuration
pub async fn update_playground_config(
    State(state): State<AppState>,
    Json(update): Json<PlaygroundConfigUpdate>,
) -> ApiResult<Json<PlaygroundConfigPublic>> {
    let config = state.playground.update_config(update).await?;
    Ok(Json(config))
}

/// List available AI backends
pub async fn list_playground_backends(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<palm_shared_state::AiBackendPublic>>> {
    let list = state.playground.backend_catalog().await;
    Ok(Json(list))
}

/// List resonators
pub async fn list_playground_resonators(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<palm_shared_state::ResonatorStatus>>> {
    let resonators = state.storage.list_resonators().await?;
    Ok(Json(resonators))
}

/// List agents (instances)
pub async fn list_playground_agents(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<palm_types::instance::AgentInstance>>> {
    let agents = state.storage.list_instances().await?;
    Ok(Json(agents))
}

/// List activities
pub async fn list_playground_activities(
    State(state): State<AppState>,
    Query(query): Query<ActivitiesQuery>,
) -> ApiResult<Json<Vec<Activity>>> {
    let activities = state
        .storage
        .list_activities(query.limit, query.after_sequence)
        .await?;
    Ok(Json(activities))
}

/// Stream activities via SSE
pub async fn stream_playground_activities(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.activity_tx.subscribe();

    let stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(activity) => {
                let json = serde_json::to_string(&activity).unwrap_or_default();
                let sse_event = Event::default().data(json);
                Some((Ok(sse_event), rx))
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                Some((Ok(Event::default().comment("lagged")), rx))
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => None,
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("ping"),
    )
}
