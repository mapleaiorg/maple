//! Error types for palm-daemon

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// Daemon-level errors
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum DaemonError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Server startup error
    #[error("Server error: {0}")]
    Server(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// Control plane error
    #[error("Control plane error: {0}")]
    ControlPlane(String),

    /// Scheduler error
    #[error("Scheduler error: {0}")]
    Scheduler(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Storage-specific errors
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum StorageError {
    /// Item not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Conflict (e.g., already exists)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Invalid data
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Query error
    #[error("Query error: {0}")]
    Query(String),
}

/// API-specific errors
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum ApiError {
    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Bad request
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Conflict
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// Policy denied
    #[error("Policy denied: {0}")]
    PolicyDenied(String),
}

/// Error response body
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            ApiError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            ApiError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR"),
            ApiError::PolicyDenied(_) => (StatusCode::FORBIDDEN, "POLICY_DENIED"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
            ApiError::Storage(StorageError::NotFound(_)) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            ApiError::Storage(_) => (StatusCode::INTERNAL_SERVER_ERROR, "STORAGE_ERROR"),
        };

        let body = ErrorResponse {
            error: self.to_string(),
            code: code.to_string(),
            details: None,
        };

        (status, Json(body)).into_response()
    }
}

/// Result type alias for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// Result type alias for daemon operations
pub type DaemonResult<T> = Result<T, DaemonError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_status_codes() {
        assert!(matches!(
            ApiError::NotFound("test".to_string())
                .into_response()
                .status(),
            StatusCode::NOT_FOUND
        ));

        assert!(matches!(
            ApiError::BadRequest("test".to_string())
                .into_response()
                .status(),
            StatusCode::BAD_REQUEST
        ));

        assert!(matches!(
            ApiError::Conflict("test".to_string())
                .into_response()
                .status(),
            StatusCode::CONFLICT
        ));
    }
}
