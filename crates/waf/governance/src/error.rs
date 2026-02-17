//! Governance error types.

use thiserror::Error;

/// Errors that can occur during governance operations.
#[derive(Debug, Error)]
pub enum GovernanceError {
    /// The requested operation was denied by governance policy.
    #[error("governance denied: {0}")]
    Denied(String),

    /// The governance tier of the caller is insufficient for the operation.
    #[error("insufficient governance tier: required {required}, actual {actual}")]
    InsufficientTier {
        required: String,
        actual: String,
    },

    /// A quorum of approvers was not reached.
    #[error("quorum not reached: {0}")]
    QuorumNotReached(String),

    /// An escalation to a higher governance tier failed.
    #[error("escalation failed: {0}")]
    EscalationFailed(String),

    /// The approval request timed out after the given number of milliseconds.
    #[error("approval timed out after {0}ms")]
    ApprovalTimeout(u64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denied_error_display() {
        let err = GovernanceError::Denied("policy violation".into());
        assert_eq!(err.to_string(), "governance denied: policy violation");
    }

    #[test]
    fn insufficient_tier_error_display() {
        let err = GovernanceError::InsufficientTier {
            required: "Tier3".into(),
            actual: "Tier1".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("required Tier3"));
        assert!(msg.contains("actual Tier1"));
    }

    #[test]
    fn timeout_error_display() {
        let err = GovernanceError::ApprovalTimeout(30000);
        assert_eq!(err.to_string(), "approval timed out after 30000ms");
    }
}
