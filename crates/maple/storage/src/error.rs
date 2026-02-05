use thiserror::Error;

/// Result type for storage operations.
pub type StorageResult<T> = Result<T, StorageError>;

/// Storage-layer errors.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("record not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("invariant violation: {0}")]
    InvariantViolation(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("backend error: {0}")]
    Backend(String),
}
