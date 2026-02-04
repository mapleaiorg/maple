//! System lifecycle handlers.

use crate::api::rest::state::AppState;
use axum::{extract::State, Json};
use serde::Serialize;

/// Response body for system shutdown requests.
#[derive(Debug, Serialize)]
pub struct ShutdownResponse {
    pub status: String,
    pub message: String,
}

/// Request a graceful daemon shutdown.
pub async fn shutdown_daemon(State(state): State<AppState>) -> Json<ShutdownResponse> {
    if let Err(err) = state.shutdown_tx.send(true) {
        tracing::warn!("Failed to send shutdown signal: {}", err);
        return Json(ShutdownResponse {
            status: "error".to_string(),
            message: "Unable to signal shutdown".to_string(),
        });
    }

    Json(ShutdownResponse {
        status: "accepted".to_string(),
        message: "Shutdown signal sent".to_string(),
    })
}
