//! Error types for WorldLine conformance testing.

use std::fmt;

/// Errors that can occur during conformance testing.
#[derive(Debug, Clone)]
pub enum ConformanceError {
    /// An invariant check failed with the given reason.
    InvariantViolation {
        invariant_id: String,
        reason: String,
    },
    /// A subsystem was unreachable or failed to initialize.
    SubsystemUnavailable { subsystem: String, reason: String },
    /// Configuration error (e.g., invalid category filter).
    InvalidConfiguration(String),
    /// A check timed out.
    Timeout {
        invariant_id: String,
        elapsed_ms: u64,
    },
    /// Internal error in the conformance framework itself.
    Internal(String),
}

impl fmt::Display for ConformanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvariantViolation {
                invariant_id,
                reason,
            } => {
                write!(f, "invariant {} violated: {}", invariant_id, reason)
            }
            Self::SubsystemUnavailable { subsystem, reason } => {
                write!(f, "subsystem {} unavailable: {}", subsystem, reason)
            }
            Self::InvalidConfiguration(msg) => {
                write!(f, "invalid configuration: {}", msg)
            }
            Self::Timeout {
                invariant_id,
                elapsed_ms,
            } => {
                write!(
                    f,
                    "invariant {} timed out after {}ms",
                    invariant_id, elapsed_ms
                )
            }
            Self::Internal(msg) => {
                write!(f, "internal error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ConformanceError {}

/// Convenience result type for conformance operations.
pub type ConformanceResult<T> = Result<T, ConformanceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invariant_violation_display() {
        let err = ConformanceError::InvariantViolation {
            invariant_id: "I.OBS-1".into(),
            reason: "overhead exceeded 1%".into(),
        };
        assert!(err.to_string().contains("I.OBS-1"));
        assert!(err.to_string().contains("overhead exceeded 1%"));
    }

    #[test]
    fn test_subsystem_unavailable_display() {
        let err = ConformanceError::SubsystemUnavailable {
            subsystem: "observation".into(),
            reason: "not initialized".into(),
        };
        assert!(err.to_string().contains("observation"));
    }

    #[test]
    fn test_invalid_configuration_display() {
        let err = ConformanceError::InvalidConfiguration("bad filter".into());
        assert!(err.to_string().contains("bad filter"));
    }

    #[test]
    fn test_timeout_display() {
        let err = ConformanceError::Timeout {
            invariant_id: "I.REGEN-1".into(),
            elapsed_ms: 5000,
        };
        assert!(err.to_string().contains("5000ms"));
    }

    #[test]
    fn test_internal_display() {
        let err = ConformanceError::Internal("unexpected state".into());
        assert!(err.to_string().contains("unexpected state"));
    }
}
