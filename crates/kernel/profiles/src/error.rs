use thiserror::Error;

/// Errors from the Profile system.
#[derive(Error, Debug)]
pub enum ProfileError {
    #[error("unknown profile type: {0}")]
    UnknownProfile(String),

    #[error("profile violation: {violation}")]
    ProfileViolation { violation: ProfileViolation },

    #[error("coupling limit exceeded: strength {strength:.2} exceeds profile limit {limit:.2}")]
    CouplingLimitExceeded { strength: f64, limit: f64 },

    #[error("attention budget exceeded: requested {requested}, profile allows {allowed}")]
    AttentionBudgetExceeded { requested: u64, allowed: u64 },

    #[error("domain not permitted: {domain} not in profile's allowed domains")]
    DomainNotPermitted { domain: String },

    #[error("risk class too high: {risk_class} exceeds profile maximum {max_risk_class}")]
    RiskClassExceeded {
        risk_class: String,
        max_risk_class: String,
    },

    #[error("human approval required: {reason}")]
    HumanApprovalRequired { reason: String },

    #[error("irreversible action not permitted by profile: {reason}")]
    IrreversibleNotPermitted { reason: String },

    #[error("concurrent coupling limit: {current} active couplings exceeds limit of {max}")]
    ConcurrentCouplingLimit { current: u32, max: u32 },

    #[error("profile merge conflict: {message}")]
    MergeConflict { message: String },
}

/// A specific profile violation detected by the enforcer.
#[derive(Clone, Debug)]
pub struct ProfileViolation {
    /// Which profile was violated
    pub profile_type: String,
    /// What dimension was violated
    pub dimension: ViolationDimension,
    /// Human-readable description
    pub description: String,
    /// Severity
    pub severity: ViolationSeverity,
}

impl std::fmt::Display for ProfileViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:?}] {:?} violation in {}: {}",
            self.severity, self.dimension, self.profile_type, self.description
        )
    }
}

/// Dimension of a profile violation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViolationDimension {
    CouplingLimits,
    AttentionBudget,
    IntentResolution,
    CommitmentAuthority,
    ConsequenceScope,
    HumanInvolvement,
}

/// Severity of a profile violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationSeverity {
    Warning,
    Violation,
    Critical,
}

/// Result of a profile enforcement check.
#[derive(Clone, Debug)]
pub enum EnforcementResult {
    /// Operation is within profile bounds
    Permitted,
    /// Operation permitted with warnings
    PermittedWithWarnings(Vec<String>),
    /// Operation denied — profile violation
    Denied {
        reason: String,
        violations: Vec<ProfileViolation>,
    },
}

impl EnforcementResult {
    pub fn is_permitted(&self) -> bool {
        matches!(
            self,
            EnforcementResult::Permitted | EnforcementResult::PermittedWithWarnings(_)
        )
    }

    pub fn is_denied(&self) -> bool {
        matches!(self, EnforcementResult::Denied { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforcement_result_predicates() {
        assert!(EnforcementResult::Permitted.is_permitted());
        assert!(!EnforcementResult::Permitted.is_denied());

        let warnings = EnforcementResult::PermittedWithWarnings(vec!["test".into()]);
        assert!(warnings.is_permitted());

        let denied = EnforcementResult::Denied {
            reason: "test".into(),
            violations: vec![],
        };
        assert!(denied.is_denied());
        assert!(!denied.is_permitted());
    }

    #[test]
    fn error_display() {
        let err = ProfileError::CouplingLimitExceeded {
            strength: 0.95,
            limit: 0.8,
        };
        assert!(err.to_string().contains("0.95"));
        assert!(err.to_string().contains("0.80"));
    }

    #[test]
    fn violation_display() {
        let violation = ProfileViolation {
            profile_type: "Human".into(),
            dimension: ViolationDimension::CouplingLimits,
            description: "coupling too strong".into(),
            severity: ViolationSeverity::Critical,
        };
        let s = format!("{}", violation);
        assert!(s.contains("Critical"));
        assert!(s.contains("Human"));
    }
}
