//! Core types for the self-modification commitment gate.
//!
//! Defines the 6-tier self-modification classification, policy decision cards,
//! conditions, review requirements, and deployment strategies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use maple_worldline_intent::types::SubstrateTier;

use crate::error::{SelfModGateError, SelfModGateResult};

// ── Self-Modification Tier ──────────────────────────────────────────────

/// Self-modification tier classification.
///
/// Extends the 4-tier `SubstrateTier` to 6 tiers with finer-grained
/// governance requirements for higher-impact changes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelfModTier {
    /// Configuration changes, parameter tuning.
    /// Approval: Automated, with notification.
    Tier0Configuration,
    /// Operator implementation changes (within existing API).
    /// Approval: Automated with canary validation.
    Tier1OperatorInternal,
    /// API surface changes (new capabilities, modified interfaces).
    /// Approval: Requires governance review.
    Tier2ApiChange,
    /// Kernel module changes (event fabric, commitment gate, memory engine).
    /// Approval: Multi-party governance review.
    Tier3KernelChange,
    /// Language/compiler changes.
    /// Approval: Human review + multi-party governance.
    Tier4SubstrateChange,
    /// Architecture restructuring.
    /// Approval: Full governance board + human quorum.
    Tier5ArchitecturalChange,
}

impl SelfModTier {
    /// Minimum confidence required for this tier.
    pub fn min_confidence(&self) -> f64 {
        match self {
            Self::Tier0Configuration => 0.7,
            Self::Tier1OperatorInternal => 0.8,
            Self::Tier2ApiChange => 0.85,
            Self::Tier3KernelChange => 0.9,
            Self::Tier4SubstrateChange => 0.95,
            Self::Tier5ArchitecturalChange => 0.98,
        }
    }

    /// Minimum observation period (seconds) for this tier.
    pub fn min_observation_secs(&self) -> u64 {
        match self {
            Self::Tier0Configuration => 1800,     // 30 minutes
            Self::Tier1OperatorInternal => 3600,   // 1 hour
            Self::Tier2ApiChange => 86400,         // 24 hours
            Self::Tier3KernelChange => 259200,     // 72 hours
            Self::Tier4SubstrateChange => 604800,  // 1 week
            Self::Tier5ArchitecturalChange => 1209600, // 2 weeks
        }
    }

    /// Whether this tier requires human review.
    pub fn requires_human_review(&self) -> bool {
        matches!(
            self,
            Self::Tier3KernelChange
                | Self::Tier4SubstrateChange
                | Self::Tier5ArchitecturalChange
        )
    }

    /// Whether this tier requires governance review.
    pub fn requires_governance_review(&self) -> bool {
        matches!(
            self,
            Self::Tier2ApiChange
                | Self::Tier3KernelChange
                | Self::Tier4SubstrateChange
                | Self::Tier5ArchitecturalChange
        )
    }

    /// Whether this tier auto-approves (with conditions).
    pub fn is_auto_approve(&self) -> bool {
        matches!(
            self,
            Self::Tier0Configuration | Self::Tier1OperatorInternal
        )
    }

    /// Default maximum deployment duration (seconds) for this tier.
    pub fn default_max_deployment_secs(&self) -> u64 {
        match self {
            Self::Tier0Configuration => 300,       // 5 minutes
            Self::Tier1OperatorInternal => 3600,   // 1 hour
            Self::Tier2ApiChange => 7200,          // 2 hours
            Self::Tier3KernelChange => 14400,      // 4 hours
            Self::Tier4SubstrateChange => 28800,   // 8 hours
            Self::Tier5ArchitecturalChange => 86400, // 24 hours
        }
    }
}

impl From<SubstrateTier> for SelfModTier {
    fn from(tier: SubstrateTier) -> Self {
        match tier {
            SubstrateTier::Tier0 => Self::Tier0Configuration,
            SubstrateTier::Tier1 => Self::Tier1OperatorInternal,
            SubstrateTier::Tier2 => Self::Tier2ApiChange,
            SubstrateTier::Tier3 => Self::Tier3KernelChange,
        }
    }
}

impl std::fmt::Display for SelfModTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tier0Configuration => write!(f, "tier-0-configuration"),
            Self::Tier1OperatorInternal => write!(f, "tier-1-operator-internal"),
            Self::Tier2ApiChange => write!(f, "tier-2-api-change"),
            Self::Tier3KernelChange => write!(f, "tier-3-kernel-change"),
            Self::Tier4SubstrateChange => write!(f, "tier-4-substrate-change"),
            Self::Tier5ArchitecturalChange => write!(f, "tier-5-architectural-change"),
        }
    }
}

// ── Decision ────────────────────────────────────────────────────────────

/// Adjudication decision for a self-modification commitment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    /// Approved for execution (possibly with conditions).
    Approved,
    /// Denied — modification must not proceed.
    Denied,
    /// Pending human or governance review.
    PendingReview,
}

// ── Conditions ──────────────────────────────────────────────────────────

/// Conditions attached to an approved self-modification.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    /// Governance must be notified of the modification.
    NotifyGovernance,
    /// Auto-rollback on any performance regression.
    AutoRollbackOnRegression,
    /// Canary deployment required before full rollout.
    CanaryRequired {
        /// Fraction of traffic to canary (0.0–1.0).
        traffic_fraction: f64,
        /// Canary observation duration (seconds).
        duration_secs: u64,
    },
    /// Staged rollout required.
    StagedRollout,
    /// Manual approval required before proceeding.
    ManualApproval,
}

// ── Review Requirements ─────────────────────────────────────────────────

/// Review requirements for pending-review decisions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewRequirement {
    /// Single governance review.
    GovernanceReview,
    /// Multi-party governance (min N approvers).
    MultiPartyGovernance { min_approvers: u32 },
    /// Human review required.
    HumanReview,
    /// Full governance board review.
    GovernanceBoard,
    /// Human quorum (min N humans).
    HumanQuorum { min_approvers: u32 },
}

// ── Policy Decision Card ────────────────────────────────────────────────

/// The result of adjudicating a self-modification commitment.
///
/// Follows the RCF `PolicyDecisionCard` pattern: a structured decision
/// with conditions, review requirements, and rationale.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyDecisionCard {
    /// The decision.
    pub decision: Decision,
    /// Conditions that must be met (for approved decisions).
    pub conditions: Vec<Condition>,
    /// Review requirements (for pending-review decisions).
    pub review_requirements: Vec<ReviewRequirement>,
    /// Rationale for the decision.
    pub reason: Option<String>,
    /// When this decision was made.
    pub decided_at: DateTime<Utc>,
}

impl PolicyDecisionCard {
    /// Create an approved decision with no conditions.
    pub fn approved() -> Self {
        Self {
            decision: Decision::Approved,
            conditions: vec![],
            review_requirements: vec![],
            reason: None,
            decided_at: Utc::now(),
        }
    }

    /// Create an approved decision with conditions.
    pub fn approved_with_conditions(conditions: Vec<Condition>) -> Self {
        Self {
            decision: Decision::Approved,
            conditions,
            review_requirements: vec![],
            reason: None,
            decided_at: Utc::now(),
        }
    }

    /// Create a denied decision with reason.
    pub fn denied(reason: impl Into<String>) -> Self {
        Self {
            decision: Decision::Denied,
            conditions: vec![],
            review_requirements: vec![],
            reason: Some(reason.into()),
            decided_at: Utc::now(),
        }
    }

    /// Create a pending-review decision.
    pub fn pending_review(requirements: Vec<ReviewRequirement>) -> Self {
        Self {
            decision: Decision::PendingReview,
            conditions: vec![],
            review_requirements: requirements,
            reason: None,
            decided_at: Utc::now(),
        }
    }

    /// Whether the decision is approved.
    pub fn is_approved(&self) -> bool {
        matches!(self.decision, Decision::Approved)
    }

    /// Whether the decision is denied.
    pub fn is_denied(&self) -> bool {
        matches!(self.decision, Decision::Denied)
    }

    /// Whether the decision is pending review.
    pub fn is_pending(&self) -> bool {
        matches!(self.decision, Decision::PendingReview)
    }
}

// ── Approval Requirements ───────────────────────────────────────────────

/// Approval requirements for a specific tier.
#[derive(Clone, Debug)]
pub struct ApprovalRequirements {
    /// Conditions required for approval.
    pub conditions: Vec<Condition>,
    /// Review requirements (empty if auto-approve).
    pub review_requirements: Vec<ReviewRequirement>,
}

// ── Deployment Strategy ─────────────────────────────────────────────────

/// Strategy for deploying a self-modification.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DeploymentStrategy {
    /// Immediate deployment (Tier 0 only).
    Immediate,
    /// Canary deployment with traffic fraction.
    Canary { traffic_fraction: f64 },
    /// Staged rollout through multiple phases.
    Staged,
    /// Blue-green deployment.
    BlueGreen,
}

impl DeploymentStrategy {
    /// Validate that this strategy is appropriate for the given tier.
    pub fn validate_for_tier(&self, tier: &SelfModTier) -> SelfModGateResult<()> {
        match (self, tier) {
            // Immediate is only valid for Tier 0
            (DeploymentStrategy::Immediate, SelfModTier::Tier0Configuration) => Ok(()),
            (DeploymentStrategy::Immediate, other) => Err(SelfModGateError::TierMismatch(
                format!("Immediate deployment not allowed for {}", other),
            )),
            // Canary required for Tier 1+
            (DeploymentStrategy::Canary { traffic_fraction }, _) => {
                if *traffic_fraction <= 0.0 || *traffic_fraction > 1.0 {
                    return Err(SelfModGateError::CommitmentInvalid(
                        "Canary traffic fraction must be in (0.0, 1.0]".into(),
                    ));
                }
                Ok(())
            }
            // Staged and BlueGreen are always valid
            (DeploymentStrategy::Staged, _) => Ok(()),
            (DeploymentStrategy::BlueGreen, _) => Ok(()),
        }
    }
}

impl std::fmt::Display for DeploymentStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Immediate => write!(f, "immediate"),
            Self::Canary { traffic_fraction } => write!(f, "canary({:.0}%)", traffic_fraction * 100.0),
            Self::Staged => write!(f, "staged"),
            Self::BlueGreen => write!(f, "blue-green"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_from_substrate_tier() {
        assert_eq!(SelfModTier::from(SubstrateTier::Tier0), SelfModTier::Tier0Configuration);
        assert_eq!(SelfModTier::from(SubstrateTier::Tier3), SelfModTier::Tier3KernelChange);
    }

    #[test]
    fn tier_display() {
        assert_eq!(SelfModTier::Tier0Configuration.to_string(), "tier-0-configuration");
        assert_eq!(SelfModTier::Tier5ArchitecturalChange.to_string(), "tier-5-architectural-change");
    }

    #[test]
    fn tier_confidence_increases() {
        let tiers = [
            SelfModTier::Tier0Configuration,
            SelfModTier::Tier1OperatorInternal,
            SelfModTier::Tier2ApiChange,
            SelfModTier::Tier3KernelChange,
            SelfModTier::Tier4SubstrateChange,
            SelfModTier::Tier5ArchitecturalChange,
        ];
        for pair in tiers.windows(2) {
            assert!(pair[0].min_confidence() < pair[1].min_confidence());
        }
    }

    #[test]
    fn tier_observation_increases() {
        let tiers = [
            SelfModTier::Tier0Configuration,
            SelfModTier::Tier1OperatorInternal,
            SelfModTier::Tier2ApiChange,
            SelfModTier::Tier3KernelChange,
            SelfModTier::Tier4SubstrateChange,
            SelfModTier::Tier5ArchitecturalChange,
        ];
        for pair in tiers.windows(2) {
            assert!(pair[0].min_observation_secs() < pair[1].min_observation_secs());
        }
    }

    #[test]
    fn tier_human_review_requirements() {
        assert!(!SelfModTier::Tier0Configuration.requires_human_review());
        assert!(!SelfModTier::Tier1OperatorInternal.requires_human_review());
        assert!(!SelfModTier::Tier2ApiChange.requires_human_review());
        assert!(SelfModTier::Tier3KernelChange.requires_human_review());
        assert!(SelfModTier::Tier4SubstrateChange.requires_human_review());
        assert!(SelfModTier::Tier5ArchitecturalChange.requires_human_review());
    }

    #[test]
    fn tier_auto_approve() {
        assert!(SelfModTier::Tier0Configuration.is_auto_approve());
        assert!(SelfModTier::Tier1OperatorInternal.is_auto_approve());
        assert!(!SelfModTier::Tier2ApiChange.is_auto_approve());
    }

    #[test]
    fn policy_card_approved() {
        let card = PolicyDecisionCard::approved();
        assert!(card.is_approved());
        assert!(!card.is_denied());
        assert!(!card.is_pending());
    }

    #[test]
    fn policy_card_denied() {
        let card = PolicyDecisionCard::denied("risk too high");
        assert!(card.is_denied());
        assert_eq!(card.reason.unwrap(), "risk too high");
    }

    #[test]
    fn policy_card_pending() {
        let card = PolicyDecisionCard::pending_review(vec![
            ReviewRequirement::HumanReview,
            ReviewRequirement::GovernanceReview,
        ]);
        assert!(card.is_pending());
        assert_eq!(card.review_requirements.len(), 2);
    }

    #[test]
    fn deployment_strategy_validate() {
        // Immediate valid for Tier0
        assert!(DeploymentStrategy::Immediate.validate_for_tier(&SelfModTier::Tier0Configuration).is_ok());
        // Immediate invalid for Tier1+
        assert!(DeploymentStrategy::Immediate.validate_for_tier(&SelfModTier::Tier1OperatorInternal).is_err());
        // Canary valid for any tier
        let canary = DeploymentStrategy::Canary { traffic_fraction: 0.05 };
        assert!(canary.validate_for_tier(&SelfModTier::Tier3KernelChange).is_ok());
        // Invalid canary fraction
        let bad_canary = DeploymentStrategy::Canary { traffic_fraction: 0.0 };
        assert!(bad_canary.validate_for_tier(&SelfModTier::Tier0Configuration).is_err());
    }
}
