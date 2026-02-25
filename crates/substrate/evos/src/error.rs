//! Error types for the EVOS integration layer.

use thiserror::Error;

/// Errors that can occur during EVOS operations.
#[derive(Debug, Error)]
pub enum EvosError {
    /// A cycle step failed.
    #[error("cycle error: {0}")]
    CycleError(String),

    /// A subsystem reported an error.
    #[error("subsystem error: {0}")]
    SubsystemError(String),

    /// Health check detected a failure.
    #[error("health check failed: {0}")]
    HealthCheckFailed(String),

    /// A safety invariant was violated.
    #[error("invariant violation: {0}")]
    InvariantViolation(String),

    /// Substrate is not ready for the requested operation.
    #[error("substrate not ready: {0}")]
    SubstrateNotReady(String),

    /// Invalid configuration.
    #[error("configuration error: {0}")]
    ConfigurationError(String),

    /// Cross-subsystem integration error.
    #[error("integration error: {0}")]
    IntegrationError(String),
}

/// Result type for EVOS operations.
pub type EvosResult<T> = Result<T, EvosError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_cycle() {
        let e = EvosError::CycleError("step 3 failed".into());
        assert!(e.to_string().contains("step 3 failed"));
    }

    #[test]
    fn error_display_subsystem() {
        let e = EvosError::SubsystemError("meaning engine offline".into());
        assert!(e.to_string().contains("meaning engine offline"));
    }

    #[test]
    fn error_display_health() {
        let e = EvosError::HealthCheckFailed("3 subsystems degraded".into());
        assert!(e.to_string().contains("3 subsystems degraded"));
    }

    #[test]
    fn error_display_invariant() {
        let e = EvosError::InvariantViolation("cycle skipped phase".into());
        assert!(e.to_string().contains("cycle skipped phase"));
    }

    #[test]
    fn error_display_not_ready() {
        let e = EvosError::SubstrateNotReady("bootstrap not at phase 1".into());
        assert!(e.to_string().contains("bootstrap not at phase 1"));
    }
}
