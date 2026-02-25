//! Error types for the self-modification commitment gate.

use thiserror::Error;

/// Errors that can occur during self-modification gate operations.
#[derive(Debug, Error)]
pub enum SelfModGateError {
    /// Commitment construction is invalid.
    #[error("commitment invalid: {0}")]
    CommitmentInvalid(String),

    /// Rollback plan validation failed.
    #[error("rollback plan invalid: {0}")]
    RollbackPlanInvalid(String),

    /// Rate limit exceeded for self-modification.
    #[error("rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Adjudication pipeline failed.
    #[error("adjudication failed: {0}")]
    AdjudicationFailed(String),

    /// Safety invariant violated.
    #[error("safety violation: {0}")]
    SafetyViolation(String),

    /// Deployment strategy does not match tier requirements.
    #[error("tier mismatch: {0}")]
    TierMismatch(String),

    /// A mandatory self-modification check failed.
    #[error("check failed: {0}")]
    CheckFailed(String),

    /// Ledger recording error.
    #[error("ledger error: {0}")]
    LedgerError(String),
}

/// Convenience result type for self-modification gate operations.
pub type SelfModGateResult<T> = Result<T, SelfModGateError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_commitment_invalid() {
        let err = SelfModGateError::CommitmentInvalid("missing rollback".into());
        assert_eq!(err.to_string(), "commitment invalid: missing rollback");
    }

    #[test]
    fn error_display_rate_limit() {
        let err = SelfModGateError::RateLimitExceeded("Tier0: 2/hr exceeded".into());
        assert_eq!(err.to_string(), "rate limit exceeded: Tier0: 2/hr exceeded");
    }

    #[test]
    fn error_display_safety_violation() {
        let err = SelfModGateError::SafetyViolation("I.REGEN-3: gate integrity".into());
        assert_eq!(
            err.to_string(),
            "safety violation: I.REGEN-3: gate integrity"
        );
    }

    #[test]
    fn result_ok() {
        let r: SelfModGateResult<u32> = Ok(42);
        assert_eq!(r.unwrap(), 42);
    }
}
