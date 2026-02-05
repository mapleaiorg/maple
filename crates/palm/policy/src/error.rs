//! Error types for policy evaluation

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Policy evaluation errors
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum PolicyError {
    /// Operation denied by policy
    #[error("Policy denied operation: {reason}")]
    Denied { reason: String },

    /// Missing required approval
    #[error("Missing required approval from: {approvers:?}")]
    MissingApproval { approvers: Vec<String> },

    /// Resource quota exceeded
    #[error("Resource quota exceeded: {resource}")]
    QuotaExceeded { resource: String },

    /// Platform constraint violated
    #[error("Platform constraint violated: {constraint}")]
    PlatformConstraint { constraint: String },

    /// Time restriction violated
    #[error("Time restriction violated: {restriction}")]
    TimeRestriction { restriction: String },

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {limit}")]
    RateLimitExceeded { limit: String },

    /// Policy evaluation failed
    #[error("Policy evaluation failed: {reason}")]
    EvaluationFailed { reason: String },

    /// Invalid policy configuration
    #[error("Invalid policy configuration: {reason}")]
    InvalidConfiguration { reason: String },

    /// Policy not found
    #[error("Policy not found: {policy_id}")]
    PolicyNotFound { policy_id: String },
}

/// Result type for policy operations
pub type Result<T> = std::result::Result<T, PolicyError>;
