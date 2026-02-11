use thiserror::Error;

/// Errors from memory engine operations.
#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("memory entry missing provenance (EventId reference required)")]
    MissingProvenance,

    #[error("memory entry not found: {0}")]
    NotFound(String),

    #[error("wrong memory class: expected {expected}, got {actual}")]
    WrongClass { expected: String, actual: String },

    #[error("working plane capacity exceeded (max {max})")]
    CapacityExceeded { max: usize },

    #[error("fabric error during rebuild: {0}")]
    FabricError(String),

    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<maple_kernel_fabric::FabricError> for MemoryError {
    fn from(e: maple_kernel_fabric::FabricError) -> Self {
        MemoryError::FabricError(e.to_string())
    }
}
