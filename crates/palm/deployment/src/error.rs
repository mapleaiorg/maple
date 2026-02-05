//! Deployment error types

use palm_types::{DeploymentId, InstanceId, PolicyError};
use thiserror::Error;

/// Deployment errors
#[derive(Debug, Error)]
pub enum DeploymentError {
    #[error("Deployment not found: {0}")]
    NotFound(DeploymentId),

    #[error("Agent spec not found: {0}")]
    SpecNotFound(String),

    #[error("Invalid deployment state: {current}, expected one of: {expected:?}")]
    InvalidState {
        current: String,
        expected: Vec<String>,
    },

    #[error("Health threshold not met: required {required:.2}, actual {actual:.2}")]
    HealthThresholdNotMet { required: f64, actual: f64 },

    #[error("Canary evaluation failed: {reason}")]
    CanaryFailed { reason: String },

    #[error("Validation failed during deployment")]
    ValidationFailed,

    #[error("Resonator creation failed: {0}")]
    ResonatorCreationFailed(String),

    #[error("Termination failed for instance {instance_id}: {reason}")]
    TerminationFailed {
        instance_id: InstanceId,
        reason: String,
    },

    #[error("Timeout waiting for {operation}")]
    Timeout { operation: String },

    #[error("Policy denied: {0}")]
    PolicyDenied(#[from] PolicyError),

    #[error("Registry error: {0}")]
    Registry(#[from] palm_registry::RegistryError),

    #[error("Scheduler error: {0}")]
    Scheduler(String),

    #[error("State store error: {0}")]
    StateStore(String),

    #[error("Resource reservation failed: {0}")]
    ResourceReservation(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for deployment operations
pub type Result<T> = std::result::Result<T, DeploymentError>;
