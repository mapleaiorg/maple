//! Error Types for RCF
//!
//! Defines all error types used across the RCF system.

use crate::ResonanceType;
use thiserror::Error;

/// RCF Error type
#[derive(Error, Debug)]
pub enum RcfError {
    /// Invalid transition between resonance types
    #[error("Invalid transition from {from} to {to}: {reason}")]
    InvalidTransition {
        from: ResonanceType,
        to: ResonanceType,
        reason: String,
    },

    /// Attempted to execute non-executable type
    #[error("Cannot execute {resonance_type}: only Commitment is executable")]
    NotExecutable { resonance_type: ResonanceType },

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Identity error
    #[error("Identity error: {0}")]
    IdentityError(String),

    /// Continuity chain error
    #[error("Continuity chain error: {0}")]
    ContinuityError(String),

    /// Capability error
    #[error("Capability error: {0}")]
    CapabilityError(String),

    /// Temporal error
    #[error("Temporal error: {0}")]
    TemporalError(String),

    /// Scope violation
    #[error("Scope violation: {0}")]
    ScopeViolation(String),

    /// Resource limit exceeded
    #[error("Resource limit exceeded: {resource} (limit: {limit}, requested: {requested})")]
    ResourceLimitExceeded {
        resource: String,
        limit: u64,
        requested: u64,
    },

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid field value
    #[error("Invalid value for field {field}: {reason}")]
    InvalidFieldValue { field: String, reason: String },

    /// Hash mismatch (integrity violation)
    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    /// Signature verification failed
    #[error("Signature verification failed: {0}")]
    SignatureError(String),

    /// Expired
    #[error("{item} has expired at {expired_at}")]
    Expired {
        item: String,
        expired_at: chrono::DateTime<chrono::Utc>,
    },

    /// Not yet effective
    #[error("{item} is not yet effective until {effective_at}")]
    NotYetEffective {
        item: String,
        effective_at: chrono::DateTime<chrono::Utc>,
    },

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl RcfError {
    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        RcfError::ValidationError(msg.into())
    }

    /// Create an identity error
    pub fn identity(msg: impl Into<String>) -> Self {
        RcfError::IdentityError(msg.into())
    }

    /// Create a capability error
    pub fn capability(msg: impl Into<String>) -> Self {
        RcfError::CapabilityError(msg.into())
    }

    /// Create a missing field error
    pub fn missing_field(field: impl Into<String>) -> Self {
        RcfError::MissingField(field.into())
    }

    /// Create an invalid field value error
    pub fn invalid_field(field: impl Into<String>, reason: impl Into<String>) -> Self {
        RcfError::InvalidFieldValue {
            field: field.into(),
            reason: reason.into(),
        }
    }

    /// Check if this is a validation error
    pub fn is_validation_error(&self) -> bool {
        matches!(self, RcfError::ValidationError(_))
    }

    /// Check if this is a temporal error (expired or not yet effective)
    pub fn is_temporal_error(&self) -> bool {
        matches!(self, RcfError::Expired { .. } | RcfError::NotYetEffective { .. })
    }

    /// Check if this is a security-related error
    pub fn is_security_error(&self) -> bool {
        matches!(
            self,
            RcfError::HashMismatch { .. }
                | RcfError::SignatureError(_)
                | RcfError::CapabilityError(_)
                | RcfError::ScopeViolation(_)
        )
    }
}

/// Result type for RCF operations
pub type RcfResult<T> = Result<T, RcfError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = RcfError::validation("test error");
        assert!(err.is_validation_error());

        let err = RcfError::missing_field("commitment_id");
        assert!(matches!(err, RcfError::MissingField(_)));
    }

    #[test]
    fn test_error_display() {
        let err = RcfError::InvalidTransition {
            from: ResonanceType::Intent,
            to: ResonanceType::Meaning,
            reason: "backward transition".to_string(),
        };
        assert!(err.to_string().contains("Invalid transition"));
    }
}
