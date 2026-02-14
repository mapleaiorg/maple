//! Deployment error types.
//!
//! Covers all failure modes in the deployment pipeline: artifact validation,
//! strategy execution, phase failures, rollback, GitHub operations, health
//! checks, timeouts, feedback generation, and configuration errors.

use thiserror::Error;

/// Errors that can occur during deployment.
#[derive(Debug, Error)]
pub enum DeploymentError {
    /// The codegen artifact is not deployable (not validated or empty).
    #[error("Artifact not deployable: {0}")]
    ArtifactNotDeployable(String),

    /// Strategy execution failed at a high level.
    #[error("Strategy execution failed: {0}")]
    StrategyExecutionFailed(String),

    /// A specific deployment phase failed.
    #[error("Phase failed: {0}")]
    PhaseFailed(String),

    /// Rollback execution failed.
    #[error("Rollback failed: {0}")]
    RollbackFailed(String),

    /// GitHub integration error.
    #[error("GitHub error: {0}")]
    GitHubError(String),

    /// Health check detected regression or failure.
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    /// Deployment timed out.
    #[error("Deployment timeout: {0}")]
    Timeout(String),

    /// Failed to generate observation feedback.
    #[error("Feedback error: {0}")]
    FeedbackError(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Result type for deployment operations.
pub type DeploymentResult<T> = Result<T, DeploymentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let e = DeploymentError::ArtifactNotDeployable("not validated".into());
        assert!(e.to_string().contains("not validated"));

        let e = DeploymentError::StrategyExecutionFailed("canary regression".into());
        assert!(e.to_string().contains("canary regression"));

        let e = DeploymentError::RollbackFailed("timeout".into());
        assert!(e.to_string().contains("timeout"));
    }

    #[test]
    fn all_error_variants_display() {
        let errors: Vec<DeploymentError> = vec![
            DeploymentError::ArtifactNotDeployable("a".into()),
            DeploymentError::StrategyExecutionFailed("b".into()),
            DeploymentError::PhaseFailed("c".into()),
            DeploymentError::RollbackFailed("d".into()),
            DeploymentError::GitHubError("e".into()),
            DeploymentError::HealthCheckFailed("f".into()),
            DeploymentError::Timeout("g".into()),
            DeploymentError::FeedbackError("h".into()),
            DeploymentError::ConfigurationError("i".into()),
        ];
        for error in &errors {
            let msg = error.to_string();
            assert!(!msg.is_empty());
        }
        assert_eq!(errors.len(), 9);
    }

    #[test]
    fn error_is_std_error() {
        let e: Box<dyn std::error::Error> =
            Box::new(DeploymentError::Timeout("5 minutes".into()));
        assert!(e.to_string().contains("5 minutes"));
    }

    #[test]
    fn result_type_works() {
        let ok: DeploymentResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: DeploymentResult<i32> =
            Err(DeploymentError::PhaseFailed("deploy".into()));
        assert!(err.is_err());
    }

    #[test]
    fn error_debug_format() {
        let e = DeploymentError::GitHubError("auth failed".into());
        let debug = format!("{:?}", e);
        assert!(debug.contains("GitHubError"));
        assert!(debug.contains("auth failed"));
    }
}
