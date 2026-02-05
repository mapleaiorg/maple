//! Error types for palm-health crate.
//!
//! Defines health monitoring and resilience-specific errors.

use palm_types::InstanceId;
use thiserror::Error;

/// Errors that can occur during health monitoring and resilience operations.
#[derive(Debug, Error)]
pub enum HealthError {
    /// Instance not found in monitoring system.
    #[error("instance not found: {0}")]
    InstanceNotFound(InstanceId),

    /// Probe execution failed.
    #[error("probe failed for instance {instance_id}: {reason}")]
    ProbeFailed {
        instance_id: InstanceId,
        reason: String,
    },

    /// Probe timed out waiting for response.
    #[error("probe timed out for instance {instance_id} after {timeout_ms}ms")]
    ProbeTimeout {
        instance_id: InstanceId,
        timeout_ms: u64,
    },

    /// Health assessment computation failed.
    #[error("health assessment failed: {0}")]
    AssessmentFailed(String),

    /// Circuit breaker is open, rejecting requests.
    #[error("circuit breaker open for instance {0}")]
    CircuitBreakerOpen(InstanceId),

    /// Recovery action failed to execute.
    #[error("recovery action failed for instance {instance_id}: {reason}")]
    RecoveryFailed {
        instance_id: InstanceId,
        reason: String,
    },

    /// Policy evaluation error.
    #[error("policy error: {0}")]
    PolicyError(String),

    /// Monitor already running for this instance.
    #[error("monitor already running for instance {0}")]
    MonitorAlreadyRunning(InstanceId),

    /// Monitor not found for instance.
    #[error("monitor not found for instance {0}")]
    MonitorNotFound(InstanceId),

    /// Configuration error.
    #[error("configuration error: {0}")]
    ConfigurationError(String),

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Result type for health operations.
pub type HealthResult<T> = Result<T, HealthError>;
