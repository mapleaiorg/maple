use maple_mwl_types::{PolicyId, WorldlineId};
use thiserror::Error;

/// Errors from the Governance Engine (AAS).
#[derive(Error, Debug)]
pub enum AasError {
    #[error("identity not found: {0}")]
    IdentityNotFound(String),

    #[error("identity already exists: {0}")]
    IdentityAlreadyExists(WorldlineId),

    #[error("identity error: {0}")]
    IdentityError(#[from] maple_mwl_identity::IdentityError),

    #[error("capability error: {0}")]
    CapabilityError(String),

    #[error("capability not found: {0}")]
    CapabilityNotFound(String),

    #[error("duplicate capability grant: {0} already held by {1}")]
    DuplicateCapability(String, WorldlineId),

    #[error("policy error: {0}")]
    PolicyError(#[from] PolicyError),

    #[error("invariant violation: {invariant_id}: {message}")]
    InvariantViolation {
        invariant_id: String,
        message: String,
    },

    #[error("fabric error: {0}")]
    FabricError(#[from] maple_kernel_fabric::FabricError),

    #[error("gate error: {0}")]
    GateError(#[from] maple_kernel_gate::GateError),
}

/// Errors from the Policy Engine.
#[derive(Error, Debug)]
pub enum PolicyError {
    #[error("policy not found: {0}")]
    PolicyNotFound(PolicyId),

    #[error("cannot remove constitutional policy: {0}")]
    ConstitutionalPolicyRemoval(PolicyId),

    #[error("cannot weaken constitutional invariant: {0}")]
    ConstitutionalInvariantWeakening(String),

    #[error("duplicate policy: {0}")]
    DuplicatePolicy(PolicyId),

    #[error("invalid policy condition: {0}")]
    InvalidCondition(String),
}

/// A constitutional invariant violation detected by the enforcer.
#[derive(Clone, Debug)]
pub struct InvariantViolation {
    pub invariant_id: String,
    pub message: String,
    pub severity: ViolationSeverity,
}

/// Severity of an invariant violation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViolationSeverity {
    /// Constitutional violation — system MUST halt the operation
    Constitutional,
    /// Warning — system SHOULD log and alert but may continue
    Warning,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invariant_violation_display() {
        let err = AasError::InvariantViolation {
            invariant_id: "I.1".into(),
            message: "Resonance stage collapsed".into(),
        };
        assert!(err.to_string().contains("I.1"));
    }

    #[test]
    fn constitutional_policy_removal_error() {
        let err = PolicyError::ConstitutionalPolicyRemoval(PolicyId("POL-001".into()));
        assert!(err.to_string().contains("constitutional"));
    }
}
