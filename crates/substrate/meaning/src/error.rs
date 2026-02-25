//! Error types for the meaning formation engine.

use thiserror::Error;

/// Errors that can occur during meaning formation.
#[derive(Debug, Error)]
pub enum MeaningError {
    /// Hypothesis generation failed.
    #[error("hypothesis generation failed: {0}")]
    HypothesisGenerationFailed(String),

    /// Evidence evaluation failed.
    #[error("evidence evaluation failed: {0}")]
    EvidenceEvaluationFailed(String),

    /// Ambiguity resolution failed.
    #[error("ambiguity resolution failed: {0}")]
    AmbiguityResolutionFailed(String),

    /// Convergence tracking error.
    #[error("convergence error: {0}")]
    ConvergenceError(String),

    /// Bridge error (meaning-to-intent).
    #[error("bridge error: {0}")]
    BridgeError(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    ConfigurationError(String),
}

/// Result type for meaning formation operations.
pub type MeaningResult<T> = Result<T, MeaningError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let err = MeaningError::HypothesisGenerationFailed("no anomalies".into());
        assert_eq!(
            err.to_string(),
            "hypothesis generation failed: no anomalies"
        );

        let err = MeaningError::EvidenceEvaluationFailed("invalid prior".into());
        assert_eq!(err.to_string(), "evidence evaluation failed: invalid prior");

        let err = MeaningError::AmbiguityResolutionFailed("timeout".into());
        assert_eq!(err.to_string(), "ambiguity resolution failed: timeout");

        let err = MeaningError::ConvergenceError("insufficient data".into());
        assert_eq!(err.to_string(), "convergence error: insufficient data");
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MeaningError>();
    }

    #[test]
    fn result_type_works() {
        let ok: MeaningResult<u32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: MeaningResult<u32> = Err(MeaningError::ConfigurationError("bad config".into()));
        assert!(err.is_err());
    }

    #[test]
    fn all_variants_display() {
        let variants: Vec<MeaningError> = vec![
            MeaningError::HypothesisGenerationFailed("a".into()),
            MeaningError::EvidenceEvaluationFailed("b".into()),
            MeaningError::AmbiguityResolutionFailed("c".into()),
            MeaningError::ConvergenceError("d".into()),
            MeaningError::BridgeError("e".into()),
            MeaningError::ConfigurationError("f".into()),
        ];
        for v in &variants {
            assert!(!v.to_string().is_empty());
        }
        assert_eq!(variants.len(), 6);
    }
}
