//! # maple-worldline-self-mod-gate
//!
//! **The Commitment Boundary** — the self-modification gate that authorizes
//! WorldLine to modify its own code.
//!
//! Nothing changes without:
//! - Explicit declaration (commitment)
//! - Governance approval per tier
//! - Mandatory rollback plan
//! - Full provenance chain
//! - Bounded scope
//! - Rate limiting
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                  Self-Modification Commitment Gate                   │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  RegenerationProposal ──▶ SelfModificationCommitment               │
//! │                                   │                                 │
//! │                           ┌───────▼───────┐                         │
//! │                           │ Rate Limiter   │ ◄── I.REGEN-6          │
//! │                           └───────┬───────┘                         │
//! │                                   │                                 │
//! │                           ┌───────▼───────┐                         │
//! │                           │  6 Checks     │ ◄── Safety Invariants   │
//! │                           └───────┬───────┘     (I.REGEN-1..7)      │
//! │                                   │                                 │
//! │                           ┌───────▼───────┐                         │
//! │                           │ Adjudication  │                         │
//! │                           │ (Tier-Based)  │                         │
//! │                           └───────┬───────┘                         │
//! │                                   │                                 │
//! │                           PolicyDecisionCard                        │
//! │                         (Approved/Denied/Pending)                    │
//! │                                   │                                 │
//! │                           ┌───────▼───────┐                         │
//! │                           │   Ledger      │ ◄── Audit trail         │
//! │                           └───────────────┘                         │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Tier System
//!
//! | Tier | Name                    | Approval            |
//! |------|-------------------------|---------------------|
//! | 0    | Configuration           | Auto + notify       |
//! | 1    | Operator Internal       | Auto + canary       |
//! | 2    | API Change              | Governance review   |
//! | 3    | Kernel Change           | Multi-party + human |
//! | 4    | Substrate Change        | Board + quorum      |
//! | 5    | Architectural Change    | Board + quorum + review |

#![deny(unsafe_code)]

pub mod adjudication;
pub mod commitment;
pub mod error;
pub mod gate;
pub mod ledger;
pub mod rate_limiter;
pub mod safety;
pub mod types;

// Re-exports
pub use commitment::{IntentChain, SelfModificationCommitment, ValidationCriterion};
pub use error::{SelfModGateError, SelfModGateResult};
pub use gate::{SelfModCheck, SelfModCheckResult, SelfModificationGate};
pub use ledger::{
    DeploymentStatus, PerformanceDelta, SelfModificationLedger, SelfModificationLedgerEntry,
};
pub use rate_limiter::{RegenerationRateLimiter, TierRateLimit};
pub use safety::{SafetyResult, SelfModificationSafetyInvariants};
pub use types::{
    ApprovalRequirements, Condition, Decision, DeploymentStrategy, PolicyDecisionCard,
    ReviewRequirement, SelfModTier,
};

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_intent::intent::ImprovementEstimate;
    use maple_worldline_intent::proposal::*;
    use maple_worldline_intent::types::{CodeChangeType, IntentId, MeaningId, ProposalId};

    fn make_commitment_for_tier(
        tier: SelfModTier,
        deployment: DeploymentStrategy,
        files: Vec<&str>,
    ) -> SelfModificationCommitment {
        let changes: Vec<CodeChangeSpec> = files
            .iter()
            .map(|f| CodeChangeSpec {
                file_path: f.to_string(),
                change_type: CodeChangeType::ModifyFunction {
                    function_name: "test".into(),
                },
                description: "test".into(),
                affected_regions: vec![],
                provenance: vec![MeaningId::new()],
            })
            .collect();

        SelfModificationCommitment::new(
            RegenerationProposal {
                id: ProposalId::new(),
                summary: "Test proposal".into(),
                rationale: "Testing".into(),
                affected_components: vec!["module".into()],
                code_changes: changes,
                required_tests: vec![TestSpec {
                    name: "t".into(),
                    description: "t".into(),
                    test_type: TestType::Unit,
                }],
                performance_gates: vec![],
                safety_checks: vec![],
                estimated_improvement: ImprovementEstimate {
                    metric: "speed".into(),
                    current_value: 10.0,
                    projected_value: 8.0,
                    confidence: 0.9,
                    unit: "ms".into(),
                },
                risk_score: 0.1,
                rollback_plan: RollbackPlan {
                    strategy: RollbackStrategy::GitRevert,
                    steps: vec!["revert".into()],
                    estimated_duration_secs: 60,
                },
            },
            tier,
            deployment,
            RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec!["git revert HEAD".into()],
                estimated_duration_secs: 60,
            },
            IntentChain {
                observation_ids: vec!["obs-1".into()],
                meaning_ids: vec![MeaningId::new()],
                intent_id: IntentId::new(),
            },
        )
        .unwrap()
    }

    #[test]
    fn full_pipeline_tier0_auto_approve() {
        // Create gate → commitment → adjudicate → expect approved with conditions
        let gate = SelfModificationGate::new();
        let commitment = make_commitment_for_tier(
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            vec!["src/config.rs"],
        );

        let decision = gate.adjudicate(&commitment);
        assert!(decision.is_approved());
        assert!(!decision.conditions.is_empty());

        // Record in ledger
        let mut ledger = SelfModificationLedger::new(100);
        let mut entry = SelfModificationLedgerEntry::new(
            commitment.id.clone(),
            commitment.tier.clone(),
            commitment.affected_files(),
            commitment.intent_chain.clone(),
            decision,
            vec![],
        );
        entry.update_status(DeploymentStatus::Succeeded);
        ledger.record(entry);

        assert_eq!(ledger.len(), 1);
        assert!((ledger.success_rate() - 1.0).abs() < 0.01);
    }

    #[test]
    fn safety_violation_denied() {
        // Attempt to modify gate code → should be denied
        let gate = SelfModificationGate::new();
        let commitment = make_commitment_for_tier(
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            vec!["src/gate.rs"], // Gate-critical!
        );

        let decision = gate.adjudicate(&commitment);
        assert!(decision.is_denied());

        // Also verify safety invariants directly
        let safety_results = SelfModificationSafetyInvariants::check_all(&commitment);
        let failures: Vec<_> = safety_results.iter().filter(|r| !r.passed).collect();
        assert!(!failures.is_empty());
    }

    #[test]
    fn tier3_requires_human_review() {
        let gate = SelfModificationGate::new();
        let commitment = make_commitment_for_tier(
            SelfModTier::Tier3KernelChange,
            DeploymentStrategy::BlueGreen,
            vec!["src/kernel.rs"],
        );

        let decision = gate.adjudicate(&commitment);
        assert!(decision.is_pending());
        assert!(decision
            .review_requirements
            .iter()
            .any(|r| matches!(r, ReviewRequirement::HumanReview)));
        assert!(decision
            .review_requirements
            .iter()
            .any(|r| matches!(
                r,
                ReviewRequirement::MultiPartyGovernance { min_approvers: 2 }
            )));
    }
}
