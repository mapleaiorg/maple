//! SAL error types.
//!
//! Covers all failure modes: substrate execution, commitment checking,
//! provenance recording, migration, resource exhaustion, and configuration.

use thiserror::Error;

/// Errors that can occur in substrate abstraction layer operations.
#[derive(Debug, Error)]
pub enum SalError {
    /// Substrate execution failed.
    #[error("Substrate execution error: {0}")]
    ExecutionFailed(String),

    /// Commitment check failed.
    #[error("Commitment check failed: {0}")]
    CommitmentCheckFailed(String),

    /// Provenance recording failed.
    #[error("Provenance recording failed: {0}")]
    ProvenanceFailed(String),

    /// Migration failed.
    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    /// Resource limit exceeded (I.SAL-4).
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Substrate not available.
    #[error("Substrate unavailable: {0}")]
    SubstrateUnavailable(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Safety violation.
    #[error("Safety violation: {0}")]
    SafetyViolation(String),
}

/// Result type for SAL operations.
pub type SalResult<T> = Result<T, SalError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let e = SalError::ExecutionFailed("timeout".into());
        assert!(e.to_string().contains("timeout"));

        let e = SalError::ResourceExhausted("memory limit".into());
        assert!(e.to_string().contains("memory limit"));
    }

    #[test]
    fn all_error_variants_display() {
        let errors: Vec<SalError> = vec![
            SalError::ExecutionFailed("a".into()),
            SalError::CommitmentCheckFailed("b".into()),
            SalError::ProvenanceFailed("c".into()),
            SalError::MigrationFailed("d".into()),
            SalError::ResourceExhausted("e".into()),
            SalError::SubstrateUnavailable("f".into()),
            SalError::ConfigurationError("g".into()),
            SalError::SafetyViolation("h".into()),
        ];
        for error in &errors {
            assert!(!error.to_string().is_empty());
        }
        assert_eq!(errors.len(), 8);
    }

    #[test]
    fn error_is_std_error() {
        let e: Box<dyn std::error::Error> =
            Box::new(SalError::MigrationFailed("snapshot fail".into()));
        assert!(e.to_string().contains("snapshot fail"));
    }

    #[test]
    fn result_type_works() {
        let ok: SalResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: SalResult<i32> = Err(SalError::ConfigurationError("bad".into()));
        assert!(err.is_err());
    }

    #[test]
    fn error_debug_format() {
        let e = SalError::SafetyViolation("commitment gate bypassed".into());
        let debug = format!("{:?}", e);
        assert!(debug.contains("SafetyViolation"));
    }
}
