//! Error types for the WLIR crate.

use thiserror::Error;

/// Errors that can occur during WLIR operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WlirError {
    /// Failed to parse an S-expression or module definition.
    #[error("parse error: {0}")]
    ParseError(String),

    /// A validation check failed.
    #[error("validation failed: {0}")]
    ValidationFailed(String),

    /// Serialization or deserialization failed.
    #[error("serialization failed: {0}")]
    SerializationFailed(String),

    /// A required provenance header is missing.
    #[error("provenance header is missing")]
    ProvenanceMissing,

    /// An operation that violates axiomatic constraints was attempted.
    #[error("unsafe operation: {0}")]
    UnsafeOperation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let err = WlirError::ParseError("unexpected token".into());
        assert_eq!(err.to_string(), "parse error: unexpected token");

        let err = WlirError::ValidationFailed("name is empty".into());
        assert_eq!(err.to_string(), "validation failed: name is empty");

        let err = WlirError::SerializationFailed("invalid utf-8".into());
        assert_eq!(err.to_string(), "serialization failed: invalid utf-8");

        let err = WlirError::ProvenanceMissing;
        assert_eq!(err.to_string(), "provenance header is missing");

        let err = WlirError::UnsafeOperation("network access".into());
        assert_eq!(err.to_string(), "unsafe operation: network access");
    }

    #[test]
    fn error_clone_and_eq() {
        let err1 = WlirError::ProvenanceMissing;
        let err2 = err1.clone();
        assert_eq!(err1, err2);

        let err3 = WlirError::ParseError("a".into());
        let err4 = WlirError::ParseError("b".into());
        assert_ne!(err3, err4);
    }

    #[test]
    fn error_is_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(WlirError::UnsafeOperation("test".into()));
        assert!(err.to_string().contains("unsafe operation"));
    }
}
