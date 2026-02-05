//! Error types for the control plane

use palm_types::{AgentSpecId, DeploymentId, InstanceId};
use thiserror::Error;

/// Control plane error type
#[derive(Debug, Error)]
pub enum ControlPlaneError {
    /// Policy denied the operation
    #[error("Policy denied: {0}")]
    PolicyDenied(String),

    /// Deployment subsystem error
    #[error("Deployment error: {0}")]
    Deployment(#[from] palm_deployment::DeploymentError),

    /// Registry subsystem error
    #[error("Registry error: {0}")]
    Registry(#[from] palm_registry::RegistryError),

    /// Health subsystem error
    #[error("Health error: {0}")]
    Health(#[from] palm_health::HealthError),

    /// State subsystem error
    #[error("State error: {0}")]
    State(#[from] palm_state::StateError),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Control plane not initialized
    #[error("Not initialized")]
    NotInitialized,

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for control plane operations
pub type Result<T> = std::result::Result<T, ControlPlaneError>;

impl ControlPlaneError {
    /// Create a not found error for a spec
    pub fn spec_not_found(spec_id: &AgentSpecId) -> Self {
        Self::NotFound(format!("Agent spec {}", spec_id))
    }

    /// Create a not found error for a deployment
    pub fn deployment_not_found(deployment_id: &DeploymentId) -> Self {
        Self::NotFound(format!("Deployment {}", deployment_id))
    }

    /// Create a not found error for an instance
    pub fn instance_not_found(instance_id: &InstanceId) -> Self {
        Self::NotFound(format!("Instance {}", instance_id))
    }
}
