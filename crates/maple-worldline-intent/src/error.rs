//! Error types for the intent stabilization engine.

use thiserror::Error;

/// Errors that can occur during intent stabilization.
#[derive(Debug, Error)]
pub enum IntentError {
    /// Intent generation failed.
    #[error("intent generation failed: {0}")]
    GenerationFailed(String),

    /// Intent validation failed.
    #[error("intent validation failed: {0}")]
    ValidationFailed(String),

    /// Intent prioritization failed.
    #[error("prioritization failed: {0}")]
    PrioritizationFailed(String),

    /// Deferral evaluation error.
    #[error("deferral error: {0}")]
    DeferralError(String),

    /// Proposal construction error.
    #[error("proposal error: {0}")]
    ProposalError(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    ConfigurationError(String),
}

/// Result type for intent stabilization operations.
pub type IntentResult<T> = Result<T, IntentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let err = IntentError::GenerationFailed("no meanings".into());
        assert_eq!(err.to_string(), "intent generation failed: no meanings");

        let err = IntentError::ValidationFailed("low confidence".into());
        assert_eq!(err.to_string(), "intent validation failed: low confidence");
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<IntentError>();
    }

    #[test]
    fn result_type_works() {
        let ok: IntentResult<u32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: IntentResult<u32> =
            Err(IntentError::ConfigurationError("bad config".into()));
        assert!(err.is_err());
    }

    #[test]
    fn all_variants_display() {
        let variants: Vec<IntentError> = vec![
            IntentError::GenerationFailed("a".into()),
            IntentError::ValidationFailed("b".into()),
            IntentError::PrioritizationFailed("c".into()),
            IntentError::DeferralError("d".into()),
            IntentError::ProposalError("e".into()),
            IntentError::ConfigurationError("f".into()),
        ];
        for v in &variants {
            assert!(!v.to_string().is_empty());
        }
        assert_eq!(variants.len(), 6);
    }
}
