//! Error types for palm-observability

use thiserror::Error;

/// Errors that can occur in observability operations
#[derive(Debug, Error)]
pub enum ObservabilityError {
    /// Metrics-related error
    #[error("Metrics error: {0}")]
    Metrics(String),

    /// Tracing-related error
    #[error("Tracing error: {0}")]
    Tracing(String),

    /// Audit-related error
    #[error("Audit error: {0}")]
    Audit(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Integrity verification failed
    #[error("Integrity verification failed: {0}")]
    IntegrityFailed(String),
}

/// Result type alias for observability operations
pub type Result<T> = std::result::Result<T, ObservabilityError>;
