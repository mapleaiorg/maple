//! Error types for the self-commitment engine.

use thiserror::Error;

/// Errors that can occur during self-commitment processing.
#[derive(Debug, Error)]
pub enum CommitmentError {
    /// Failed to map intent to commitment declaration.
    #[error("mapping failed: {0}")]
    MappingFailed(String),

    /// Observation period has not completed.
    #[error("observation incomplete: {0}")]
    ObservationIncomplete(String),

    /// Lifecycle state transition error.
    #[error("lifecycle error: {0}")]
    LifecycleError(String),

    /// Gate submission failed.
    #[error("gate submission failed: {0}")]
    GateSubmissionFailed(String),

    /// Intent is not ready for commitment.
    #[error("intent not ready: {0}")]
    IntentNotReady(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    ConfigurationError(String),
}

/// Result type for self-commitment operations.
pub type CommitmentResult<T> = Result<T, CommitmentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let err = CommitmentError::MappingFailed("bad field".into());
        assert_eq!(err.to_string(), "mapping failed: bad field");

        let err = CommitmentError::ObservationIncomplete("30min remaining".into());
        assert_eq!(err.to_string(), "observation incomplete: 30min remaining");
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CommitmentError>();
    }

    #[test]
    fn result_type_works() {
        let ok: CommitmentResult<u32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: CommitmentResult<u32> =
            Err(CommitmentError::ConfigurationError("bad config".into()));
        assert!(err.is_err());
    }

    #[test]
    fn all_variants_display() {
        let variants: Vec<CommitmentError> = vec![
            CommitmentError::MappingFailed("a".into()),
            CommitmentError::ObservationIncomplete("b".into()),
            CommitmentError::LifecycleError("c".into()),
            CommitmentError::GateSubmissionFailed("d".into()),
            CommitmentError::IntentNotReady("e".into()),
            CommitmentError::ConfigurationError("f".into()),
        ];
        for v in &variants {
            assert!(!v.to_string().is_empty());
        }
        assert_eq!(variants.len(), 6);
    }
}
