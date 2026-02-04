use thiserror::Error;

/// iBank runtime errors.
#[derive(Debug, Error)]
pub enum IBankError {
    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    #[error("Accountability verification failed: {0}")]
    Accountability(String),

    #[error("Risk policy denied request: {0}")]
    RiskDenied(String),

    #[error("Hybrid review required: {0}")]
    HybridRequired(String),

    #[error("Connector not found for rail '{0}'")]
    ConnectorNotFound(String),

    #[error("Connector '{connector}' failed: {message}")]
    ConnectorFailure { connector: String, message: String },

    #[error("MAPLE runtime error: {0}")]
    MapleRuntime(String),

    #[error("Ledger error: {0}")]
    Ledger(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl IBankError {
    pub fn stage_violation(expected: &str, actual: &str) -> Self {
        Self::InvariantViolation(format!(
            "stage order violation: expected '{}', got '{}'",
            expected, actual
        ))
    }
}
