//! Registry error types

use palm_types::{AgentSpecId, DeploymentId, InstanceId};
use thiserror::Error;

/// Registry errors
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Agent spec not found: {0}")]
    SpecNotFound(AgentSpecId),

    #[error("Deployment not found: {0}")]
    DeploymentNotFound(DeploymentId),

    #[error("Instance not found: {0}")]
    InstanceNotFound(InstanceId),

    #[error("Spec already exists: {0}")]
    SpecAlreadyExists(AgentSpecId),

    #[error("Instance already exists: {0}")]
    InstanceAlreadyExists(InstanceId),

    #[error("Version conflict: current {current}, expected {expected}")]
    VersionConflict {
        current: String,
        expected: String,
    },

    #[error("Invalid spec: {0}")]
    InvalidSpec(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Lock error")]
    Lock,
}

/// Result type for registry operations
pub type Result<T> = std::result::Result<T, RegistryError>;
