//! Governance for hardware generation (Tier 4-5).
//!
//! Hardware generation is a Tier 4 (SubstrateChange) or Tier 5
//! (ArchitecturalChange) operation that ALWAYS requires human review.
//! This module enforces that requirement.

use serde::{Deserialize, Serialize};

use crate::epu::EpuSpec;
use crate::error::{HardwareError, HardwareResult};

// ── Governance Tier ─────────────────────────────────────────────────

/// Governance tier for hardware operations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareGovernanceTier {
    /// Tier 4: Substrate-level changes (FPGA reconfiguration).
    Tier4SubstrateChange,
    /// Tier 5: Architectural changes (new EPU design).
    Tier5ArchitecturalChange,
}

impl std::fmt::Display for HardwareGovernanceTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tier4SubstrateChange => write!(f, "Tier4:SubstrateChange"),
            Self::Tier5ArchitecturalChange => write!(f, "Tier5:ArchitecturalChange"),
        }
    }
}

// ── Governance Decision ─────────────────────────────────────────────

/// Decision on a hardware governance request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernanceDecision {
    Approved,
    PendingReview,
    Denied(String),
}

impl std::fmt::Display for GovernanceDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Approved => write!(f, "approved"),
            Self::PendingReview => write!(f, "pending-review"),
            Self::Denied(reason) => write!(f, "denied: {}", reason),
        }
    }
}

// ── Governance Request ──────────────────────────────────────────────

/// A request for governance approval for hardware generation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GovernanceRequest {
    pub tier: HardwareGovernanceTier,
    pub epu_name: String,
    pub description: String,
    pub resource_impact: String,
    pub rollback_plan: String,
}

/// Record of a governance review.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GovernanceRecord {
    pub request: GovernanceRequest,
    pub decision: GovernanceDecision,
    pub reviewer: String,
    pub reviewed_at: chrono::DateTime<chrono::Utc>,
}

// ── Hardware Governance Trait ────────────────────────────────────────

/// Trait for hardware governance checks.
pub trait HardwareGovernance: Send + Sync {
    /// Determine the governance tier for an EPU design.
    fn classify(&self, spec: &EpuSpec) -> HardwareGovernanceTier;

    /// Submit a governance request and get a decision.
    fn review(&self, request: &GovernanceRequest) -> HardwareResult<GovernanceRecord>;

    /// Name of this governance implementation.
    fn name(&self) -> &str;
}

/// Simulated hardware governance for deterministic testing.
///
/// New EPU designs are always Tier 5; modifications are Tier 4.
/// In simulation, all requests are approved (in production, human review required).
pub struct SimulatedHardwareGovernance {
    auto_approve: bool,
}

impl SimulatedHardwareGovernance {
    /// Create with auto-approval (for testing).
    pub fn new() -> Self {
        Self { auto_approve: true }
    }

    /// Create requiring manual approval (simulated pending).
    pub fn strict() -> Self {
        Self {
            auto_approve: false,
        }
    }
}

impl Default for SimulatedHardwareGovernance {
    fn default() -> Self {
        Self::new()
    }
}

impl HardwareGovernance for SimulatedHardwareGovernance {
    fn classify(&self, spec: &EpuSpec) -> HardwareGovernanceTier {
        // New EPU designs (version 1.x) are Tier 5
        if spec.version.starts_with("1.") {
            HardwareGovernanceTier::Tier5ArchitecturalChange
        } else {
            HardwareGovernanceTier::Tier4SubstrateChange
        }
    }

    fn review(&self, request: &GovernanceRequest) -> HardwareResult<GovernanceRecord> {
        let decision = if self.auto_approve {
            GovernanceDecision::Approved
        } else {
            GovernanceDecision::PendingReview
        };

        Ok(GovernanceRecord {
            request: request.clone(),
            decision,
            reviewer: "simulated-reviewer".into(),
            reviewed_at: chrono::Utc::now(),
        })
    }

    fn name(&self) -> &str {
        "simulated-hardware-governance"
    }
}

/// Check governance before hardware generation, returning an error if denied.
pub fn enforce_governance(
    governance: &dyn HardwareGovernance,
    spec: &EpuSpec,
) -> HardwareResult<GovernanceRecord> {
    let tier = governance.classify(spec);

    let request = GovernanceRequest {
        tier: tier.clone(),
        epu_name: spec.name.clone(),
        description: format!("Hardware generation for EPU '{}'", spec.name),
        resource_impact: format!("{}", spec.total_resources),
        rollback_plan: "Revert to previous bitstream".into(),
    };

    let record = governance.review(&request)?;

    match &record.decision {
        GovernanceDecision::Denied(reason) => Err(HardwareError::GovernanceRequired(format!(
            "{} governance denied: {}",
            tier, reason
        ))),
        GovernanceDecision::PendingReview => Err(HardwareError::GovernanceRequired(format!(
            "{} governance pending human review for EPU '{}'",
            tier, spec.name
        ))),
        GovernanceDecision::Approved => Ok(record),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epu::{EpuDesigner, SimulatedEpuDesigner};

    fn sample_spec() -> EpuSpec {
        SimulatedEpuDesigner::new().design("gov-test", 100).unwrap()
    }

    #[test]
    fn governance_tier_display() {
        assert_eq!(
            HardwareGovernanceTier::Tier4SubstrateChange.to_string(),
            "Tier4:SubstrateChange"
        );
        assert_eq!(
            HardwareGovernanceTier::Tier5ArchitecturalChange.to_string(),
            "Tier5:ArchitecturalChange"
        );
    }

    #[test]
    fn governance_decision_display() {
        assert_eq!(GovernanceDecision::Approved.to_string(), "approved");
        assert_eq!(GovernanceDecision::PendingReview.to_string(), "pending-review");
        assert!(GovernanceDecision::Denied("too risky".into())
            .to_string()
            .contains("too risky"));
    }

    #[test]
    fn classify_new_design_tier5() {
        let gov = SimulatedHardwareGovernance::new();
        let spec = sample_spec(); // version 1.0.0
        let tier = gov.classify(&spec);
        assert_eq!(tier, HardwareGovernanceTier::Tier5ArchitecturalChange);
    }

    #[test]
    fn auto_approve_succeeds() {
        let gov = SimulatedHardwareGovernance::new();
        let spec = sample_spec();
        let record = enforce_governance(&gov, &spec).unwrap();
        assert_eq!(record.decision, GovernanceDecision::Approved);
    }

    #[test]
    fn strict_mode_returns_pending() {
        let gov = SimulatedHardwareGovernance::strict();
        let spec = sample_spec();
        let result = enforce_governance(&gov, &spec);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("pending human review"));
    }

    #[test]
    fn governance_request_has_rollback() {
        let gov = SimulatedHardwareGovernance::new();
        let spec = sample_spec();
        let record = enforce_governance(&gov, &spec).unwrap();
        assert!(!record.request.rollback_plan.is_empty());
    }

    #[test]
    fn governance_name() {
        let gov = SimulatedHardwareGovernance::new();
        assert_eq!(gov.name(), "simulated-hardware-governance");
    }
}
