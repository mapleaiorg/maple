//! API Router configuration

use super::handlers;
use super::state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Create the main API router
pub fn create_router(state: AppState) -> Router {
    let api_routes = Router::new()
        // Health and status
        .route("/health", get(handlers::health_check))
        .route("/status", get(handlers::daemon_status))
        // Specs
        .route("/specs", get(handlers::list_specs))
        .route("/specs", post(handlers::create_spec))
        .route("/specs/:id", get(handlers::get_spec))
        .route("/specs/:id", put(handlers::update_spec))
        .route("/specs/:id", delete(handlers::delete_spec))
        // Deployments
        .route("/deployments", get(handlers::list_deployments))
        .route("/deployments", post(handlers::create_deployment))
        .route("/deployments/:id", get(handlers::get_deployment))
        .route("/deployments/:id", put(handlers::update_deployment))
        .route("/deployments/:id", delete(handlers::delete_deployment))
        .route("/deployments/:id/scale", post(handlers::scale_deployment))
        .route("/deployments/:id/pause", post(handlers::pause_deployment))
        .route("/deployments/:id/resume", post(handlers::resume_deployment))
        // Instances
        .route("/instances", get(handlers::list_instances))
        .route("/instances/:id", get(handlers::get_instance))
        .route("/instances/:id", delete(handlers::delete_instance))
        .route("/instances/:id/health", get(handlers::get_instance_health))
        .route("/instances/:id/checkpoint", post(handlers::create_checkpoint))
        .route("/instances/:id/snapshots", get(handlers::list_snapshots))
        .route("/instances/:id/restore", post(handlers::restore_snapshot))
        // Health
        .route("/health/unhealthy", get(handlers::list_unhealthy))
        .route("/health/summary", get(handlers::health_summary))
        // Events
        .route("/events", get(handlers::get_events))
        .route("/events/stream", get(handlers::stream_events));

    // Build router with middleware
    Router::new()
        .nest("/api/v1", api_routes)
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state)
}
