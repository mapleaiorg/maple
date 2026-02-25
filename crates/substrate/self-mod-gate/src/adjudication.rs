//! Adjudication pipeline — evaluates and decides on self-modification commitments.
//!
//! The pipeline runs in order:
//! 1. Rate limit check (fast fail)
//! 2. All self-modification checks (parallel-ready)
//! 3. Mandatory check failure → denied
//! 4. Tier-specific approval decision

use crate::commitment::SelfModificationCommitment;
use crate::gate::{SelfModCheckResult, SelfModificationGate};
use crate::types::{Condition, PolicyDecisionCard, ReviewRequirement, SelfModTier};

// ── Check Result ────────────────────────────────────────────────────────

/// Result of a single check during adjudication.
#[derive(Clone, Debug)]
pub struct CheckResult {
    /// Name of the check that produced this result.
    pub check_name: String,
    /// The check result (pass/fail + details).
    pub result: SelfModCheckResult,
    /// Whether this check was mandatory.
    pub mandatory: bool,
}

impl CheckResult {
    /// Whether this is a mandatory failure.
    pub fn is_mandatory_failure(&self) -> bool {
        self.mandatory && !self.result.passed
    }
}

// ── Adjudication Pipeline ───────────────────────────────────────────────

impl SelfModificationGate {
    /// Adjudicate a self-modification commitment.
    ///
    /// Pipeline:
    /// 1. Rate limit check (fast fail)
    /// 2. Run all self-modification checks
    /// 3. If any mandatory check fails → denied
    /// 4. Tier-specific approval decision
    pub fn adjudicate(&self, commitment: &SelfModificationCommitment) -> PolicyDecisionCard {
        // 1. Rate limit check
        if !self.rate_limiter.allow(
            &commitment.tier,
            &commitment
                .affected_components()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        ) {
            return PolicyDecisionCard::denied(format!(
                "Rate limit exceeded for tier {}",
                commitment.tier,
            ));
        }

        // 2. Run all checks
        let check_results: Vec<CheckResult> = self
            .run_checks(commitment)
            .into_iter()
            .map(|(result, mandatory)| CheckResult {
                check_name: result.check_name.clone(),
                result,
                mandatory,
            })
            .collect();

        // 3. Check for mandatory failures
        for cr in &check_results {
            if cr.is_mandatory_failure() {
                return PolicyDecisionCard::denied(format!(
                    "Mandatory check '{}' failed: {}",
                    cr.check_name, cr.result.details,
                ));
            }
        }

        // 4. Tier-specific approval
        Self::tier_approval(&commitment.tier)
    }

    /// Tier-specific approval logic.
    ///
    /// - Tier 0-1: Auto-approve with conditions
    /// - Tier 2+: Pending review with requirements
    fn tier_approval(tier: &SelfModTier) -> PolicyDecisionCard {
        match tier {
            SelfModTier::Tier0Configuration => PolicyDecisionCard::approved_with_conditions(vec![
                Condition::NotifyGovernance,
                Condition::AutoRollbackOnRegression,
            ]),
            SelfModTier::Tier1OperatorInternal => {
                PolicyDecisionCard::approved_with_conditions(vec![
                    Condition::CanaryRequired {
                        traffic_fraction: 0.05,
                        duration_secs: 3600,
                    },
                    Condition::AutoRollbackOnRegression,
                ])
            }
            SelfModTier::Tier2ApiChange => {
                PolicyDecisionCard::pending_review(vec![ReviewRequirement::GovernanceReview])
            }
            SelfModTier::Tier3KernelChange => PolicyDecisionCard::pending_review(vec![
                ReviewRequirement::MultiPartyGovernance { min_approvers: 2 },
                ReviewRequirement::HumanReview,
            ]),
            SelfModTier::Tier4SubstrateChange => PolicyDecisionCard::pending_review(vec![
                ReviewRequirement::GovernanceBoard,
                ReviewRequirement::HumanQuorum { min_approvers: 3 },
            ]),
            SelfModTier::Tier5ArchitecturalChange => PolicyDecisionCard::pending_review(vec![
                ReviewRequirement::GovernanceBoard,
                ReviewRequirement::HumanQuorum { min_approvers: 3 },
                ReviewRequirement::GovernanceReview,
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commitment::IntentChain;
    use crate::types::DeploymentStrategy;
    use maple_worldline_intent::intent::ImprovementEstimate;
    use maple_worldline_intent::proposal::*;
    use maple_worldline_intent::types::MeaningId;
    use maple_worldline_intent::types::{CodeChangeType, IntentId, ProposalId};

    fn make_commitment(
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
                summary: "Test".into(),
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
    fn tier0_auto_approves_with_conditions() {
        let gate = SelfModificationGate::new();
        let commitment = make_commitment(
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            vec!["src/config.rs"],
        );

        let decision = gate.adjudicate(&commitment);
        assert!(decision.is_approved());
        assert!(!decision.conditions.is_empty());
        assert!(decision
            .conditions
            .iter()
            .any(|c| matches!(c, Condition::NotifyGovernance)));
    }

    #[test]
    fn tier1_auto_approves_with_canary() {
        let gate = SelfModificationGate::new();
        let commitment = make_commitment(
            SelfModTier::Tier1OperatorInternal,
            DeploymentStrategy::Canary {
                traffic_fraction: 0.05,
            },
            vec!["src/operator.rs"],
        );

        let decision = gate.adjudicate(&commitment);
        assert!(decision.is_approved());
        assert!(decision
            .conditions
            .iter()
            .any(|c| matches!(c, Condition::CanaryRequired { .. })));
    }

    #[test]
    fn tier2_pending_review() {
        let gate = SelfModificationGate::new();
        let commitment = make_commitment(
            SelfModTier::Tier2ApiChange,
            DeploymentStrategy::Staged,
            vec!["src/api.rs"],
        );

        let decision = gate.adjudicate(&commitment);
        assert!(decision.is_pending());
        assert!(decision
            .review_requirements
            .iter()
            .any(|r| matches!(r, ReviewRequirement::GovernanceReview)));
    }

    #[test]
    fn tier3_requires_multi_party_and_human() {
        let gate = SelfModificationGate::new();
        let commitment = make_commitment(
            SelfModTier::Tier3KernelChange,
            DeploymentStrategy::BlueGreen,
            vec!["src/kernel.rs"],
        );

        let decision = gate.adjudicate(&commitment);
        assert!(decision.is_pending());
        assert!(decision.review_requirements.iter().any(|r| matches!(
            r,
            ReviewRequirement::MultiPartyGovernance { min_approvers: 2 }
        )));
        assert!(decision
            .review_requirements
            .iter()
            .any(|r| matches!(r, ReviewRequirement::HumanReview)));
    }

    #[test]
    fn safety_file_denied() {
        let gate = SelfModificationGate::new();
        let commitment = make_commitment(
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            vec!["src/safety/handler.rs"], // Safety-critical!
        );

        let decision = gate.adjudicate(&commitment);
        assert!(decision.is_denied());
        assert!(decision.reason.unwrap().contains("safety"));
    }

    #[test]
    fn gate_file_denied() {
        let gate = SelfModificationGate::new();
        let commitment = make_commitment(
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            vec!["src/adjudication.rs"], // Gate-critical!
        );

        let decision = gate.adjudicate(&commitment);
        assert!(decision.is_denied());
    }
}
