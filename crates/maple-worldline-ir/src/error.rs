//! WLIR error types.
//!
//! Covers all failure modes in the WLIR pipeline: verification,
//! type checking, commitment boundary violations, provenance,
//! safety fences, serialization, and module structure errors.

use thiserror::Error;

/// Errors that can occur in WLIR operations.
#[derive(Debug, Error)]
pub enum WlirError {
    /// Verification of a WLIR module failed.
    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    /// Type error in WLIR instructions.
    #[error("Type error: {0}")]
    TypeError(String),

    /// Commitment boundary was violated (unmatched enter/exit).
    #[error("Commitment boundary violation: {0}")]
    CommitmentBoundaryViolation(String),

    /// Provenance tracking is incomplete.
    #[error("Provenance incomplete: {0}")]
    ProvenanceIncomplete(String),

    /// Safety fence ordering is violated.
    #[error("Safety fence violation: {0}")]
    SafetyFenceViolation(String),

    /// Serialization failed.
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    /// Deserialization failed.
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    /// Invalid instruction encountered.
    #[error("Invalid instruction: {0}")]
    InvalidInstruction(String),

    /// Module structure is invalid.
    #[error("Module invalid: {0}")]
    ModuleInvalid(String),

    /// Control flow integrity violation.
    #[error("Control flow integrity: {0}")]
    ControlFlowIntegrity(String),
}

/// Result type for WLIR operations.
pub type WlirResult<T> = Result<T, WlirError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let e = WlirError::VerificationFailed("type mismatch".into());
        assert!(e.to_string().contains("type mismatch"));

        let e = WlirError::CommitmentBoundaryViolation("unmatched enter".into());
        assert!(e.to_string().contains("unmatched enter"));
    }

    #[test]
    fn all_error_variants_display() {
        let errors: Vec<WlirError> = vec![
            WlirError::VerificationFailed("a".into()),
            WlirError::TypeError("b".into()),
            WlirError::CommitmentBoundaryViolation("c".into()),
            WlirError::ProvenanceIncomplete("d".into()),
            WlirError::SafetyFenceViolation("e".into()),
            WlirError::SerializationFailed("f".into()),
            WlirError::DeserializationFailed("g".into()),
            WlirError::InvalidInstruction("h".into()),
            WlirError::ModuleInvalid("i".into()),
            WlirError::ControlFlowIntegrity("j".into()),
        ];
        for error in &errors {
            assert!(!error.to_string().is_empty());
        }
        assert_eq!(errors.len(), 10);
    }

    #[test]
    fn error_is_std_error() {
        let e: Box<dyn std::error::Error> =
            Box::new(WlirError::TypeError("wrong type".into()));
        assert!(e.to_string().contains("wrong type"));
    }

    #[test]
    fn result_type_works() {
        let ok: WlirResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: WlirResult<i32> = Err(WlirError::ModuleInvalid("empty".into()));
        assert!(err.is_err());
    }

    #[test]
    fn error_debug_format() {
        let e = WlirError::SafetyFenceViolation("missing fence".into());
        let debug = format!("{:?}", e);
        assert!(debug.contains("SafetyFenceViolation"));
    }
}
