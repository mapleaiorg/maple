//! Error types for palm-state crate.
//!
//! Defines state management and continuity-specific errors.

use palm_types::InstanceId;
use thiserror::Error;

/// Errors that can occur during state management operations.
#[derive(Debug, Error)]
pub enum StateError {
    /// Instance not found.
    #[error("instance not found: {0}")]
    InstanceNotFound(InstanceId),

    /// Snapshot not found.
    #[error("snapshot not found: {0}")]
    SnapshotNotFound(StateSnapshotId),

    /// Checkpoint creation failed.
    #[error("checkpoint failed for {instance_id}: {reason}")]
    CheckpointFailed {
        instance_id: InstanceId,
        reason: String,
    },

    /// Restore operation failed.
    #[error("restore failed for {instance_id}: {reason}")]
    RestoreFailed {
        instance_id: InstanceId,
        reason: String,
    },

    /// Migration failed.
    #[error("migration failed: {0}")]
    MigrationFailed(String),

    /// Continuity verification failed.
    #[error("continuity verification failed: {0}")]
    ContinuityVerificationFailed(String),

    /// Storage error.
    #[error("storage error: {0}")]
    Storage(String),

    /// Runtime error.
    #[error("runtime error: {0}")]
    Runtime(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Integrity check failed.
    #[error("integrity check failed: expected {expected}, got {actual}")]
    IntegrityCheckFailed { expected: String, actual: String },

    /// Coupling restoration failed.
    #[error("coupling restoration failed: {0}")]
    CouplingRestorationFailed(String),

    /// Commitment reconciliation failed.
    #[error("commitment reconciliation failed: {0}")]
    CommitmentReconciliationFailed(String),

    /// Registry error.
    #[error("registry error: {0}")]
    Registry(#[from] palm_registry::RegistryError),
}

/// State snapshot identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct StateSnapshotId(uuid::Uuid);

impl StateSnapshotId {
    /// Generate a new snapshot ID.
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Create from a UUID.
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }
}

impl std::fmt::Display for StateSnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "snapshot-{}", self.0)
    }
}

/// Result type for state operations.
pub type Result<T> = std::result::Result<T, StateError>;
