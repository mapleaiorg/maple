//! Error types for the self-consequence engine.

use thiserror::Error;

/// Errors that can occur during self-consequence execution.
#[derive(Debug, Error)]
pub enum ConsequenceError {
    /// Self-modification execution failed.
    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    /// Rollback of a failed modification also failed.
    #[error("rollback failed: {0}")]
    RollbackFailed(String),

    /// Receipt generation or verification failed.
    #[error("receipt error: {0}")]
    ReceiptError(String),

    /// Attempted to execute a commitment that is not approved.
    #[error("commitment not approved: {0}")]
    CommitmentNotApproved(String),

    /// Observation feedback generation failed.
    #[error("feedback error: {0}")]
    FeedbackError(String),

    /// Invalid configuration.
    #[error("configuration error: {0}")]
    ConfigurationError(String),
}

/// Convenience result type for consequence operations.
pub type ConsequenceResult<T> = Result<T, ConsequenceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_execution_failed() {
        let err = ConsequenceError::ExecutionFailed("test compilation failed".into());
        assert_eq!(err.to_string(), "execution failed: test compilation failed");
    }

    #[test]
    fn error_display_rollback_failed() {
        let err = ConsequenceError::RollbackFailed("git revert conflict".into());
        assert_eq!(err.to_string(), "rollback failed: git revert conflict");
    }

    #[test]
    fn error_display_commitment_not_approved() {
        let err = ConsequenceError::CommitmentNotApproved("cmt-123".into());
        assert_eq!(err.to_string(), "commitment not approved: cmt-123");
    }

    #[test]
    fn consequence_result_ok() {
        let result: ConsequenceResult<u32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }
}
