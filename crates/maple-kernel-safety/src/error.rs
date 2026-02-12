use maple_mwl_types::WorldlineId;
use thiserror::Error;

/// Errors from the Safety Suite.
#[derive(Error, Debug)]
pub enum SafetyError {
    #[error("boundary violation: {boundary_type} — {message}")]
    BoundaryViolation {
        boundary_type: String,
        message: String,
    },

    #[error("attention budget exhausted for worldline {0}")]
    AttentionExhausted(WorldlineId),

    #[error("attention allocation exceeds budget: requested {requested}, available {available}")]
    InsufficientAttention { requested: u64, available: u64 },

    #[error("coercion detected: {indicator}")]
    CoercionDetected { indicator: String },

    #[error("consent violation: {0}")]
    ConsentViolation(String),

    #[error("ethical override triggered: {reason}")]
    EthicalOverride { reason: String },

    #[error("coupling exceeds boundary limit: strength {strength}, limit {limit}")]
    CouplingBeyondBoundary { strength: f64, limit: f64 },

    #[error("emergency decouple: {reason}")]
    EmergencyDecouple { reason: String },

    #[error("invariant violation: {invariant_id} — {message}")]
    InvariantViolation {
        invariant_id: String,
        message: String,
    },
}

/// Result of a safety check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SafetyCheckResult {
    /// Safe to proceed
    Safe,
    /// Warning — proceed with caution
    Warning(String),
    /// Blocked — operation must not proceed
    Blocked(String),
    /// Override — safety override triggered
    Override(String),
}

impl SafetyCheckResult {
    pub fn is_safe(&self) -> bool {
        matches!(self, SafetyCheckResult::Safe)
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, SafetyCheckResult::Blocked(_) | SafetyCheckResult::Override(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safety_check_result_predicates() {
        assert!(SafetyCheckResult::Safe.is_safe());
        assert!(!SafetyCheckResult::Safe.is_blocked());

        let blocked = SafetyCheckResult::Blocked("test".into());
        assert!(!blocked.is_safe());
        assert!(blocked.is_blocked());

        let override_result = SafetyCheckResult::Override("safety".into());
        assert!(override_result.is_blocked());
    }

    #[test]
    fn error_display() {
        let err = SafetyError::ConsentViolation("silence treated as consent".into());
        assert!(err.to_string().contains("consent"));
    }
}
