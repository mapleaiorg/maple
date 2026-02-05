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
        .route("/system/shutdown", post(handlers::shutdown_daemon))
        // Agent kernel
        .route("/agent-kernel/status", get(handlers::agent_kernel_status))
        .route("/agent-kernel/handle", post(handlers::agent_kernel_handle))
        .route("/agent-kernel/audit", get(handlers::agent_kernel_audit))
        .route(
            "/agent-kernel/commitments",
            get(handlers::agent_kernel_commitments),
        )
        .route(
            "/agent-kernel/commitments/:id",
            get(handlers::agent_kernel_commitment),
        )
        .route(
            "/agent-kernel/commitments/:id/receipts",
            get(handlers::agent_kernel_commitment_receipts),
        )
        // Agent (simple aliases)
        .route("/agent/status", get(handlers::agent_kernel_status))
        .route("/agent/handle", post(handlers::agent_kernel_handle))
        .route("/agent/audit", get(handlers::agent_kernel_audit))
        .route(
            "/agent/commitments",
            get(handlers::agent_kernel_commitments),
        )
        .route(
            "/agent/commitments/:id",
            get(handlers::agent_kernel_commitment),
        )
        .route(
            "/agent/commitments/:id/receipts",
            get(handlers::agent_kernel_commitment_receipts),
        )
        // Playground
        .route("/playground/state", get(handlers::playground_state))
        .route("/playground/config", get(handlers::get_playground_config))
        .route(
            "/playground/config",
            put(handlers::update_playground_config),
        )
        .route(
            "/playground/backends",
            get(handlers::list_playground_backends),
        )
        .route(
            "/playground/infer",
            post(handlers::infer_playground_backend),
        )
        .route(
            "/playground/resonators",
            get(handlers::list_playground_resonators),
        )
        .route("/playground/agents", get(handlers::list_playground_agents))
        .route(
            "/playground/activities",
            get(handlers::list_playground_activities),
        )
        .route(
            "/playground/activities/stream",
            get(handlers::stream_playground_activities),
        )
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
        .route(
            "/instances/:id/checkpoint",
            post(handlers::create_checkpoint),
        )
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
        .route("/playground", get(handlers::playground_index))
        .route("/", get(handlers::playground_index))
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
