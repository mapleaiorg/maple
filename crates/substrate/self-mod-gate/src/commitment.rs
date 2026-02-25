//! Self-modification commitment — the unit of authorized self-change.
//!
//! A `SelfModificationCommitment` bundles a regeneration proposal with
//! governance metadata, a mandatory rollback plan, deployment strategy,
//! and a full provenance chain back to the original observations.
//!
//! **Structural enforcement**: you cannot create a commitment without
//! a rollback plan — the type system requires it via the constructor.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use maple_worldline_intent::proposal::{RegenerationProposal, RollbackPlan};
use maple_worldline_intent::types::IntentId;
use maple_worldline_intent::types::MeaningId;

use crate::error::{SelfModGateError, SelfModGateResult};
use crate::types::{DeploymentStrategy, SelfModTier};

// ── Intent Chain ────────────────────────────────────────────────────────

/// Full provenance chain from observations to this commitment.
///
/// Every self-modification must be traceable back to the original
/// observations that motivated it: I.REGEN-4 (Observable Change).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentChain {
    /// Observation IDs that started the chain.
    pub observation_ids: Vec<String>,
    /// Meaning IDs derived from observations.
    pub meaning_ids: Vec<MeaningId>,
    /// The intent that was stabilized into this commitment.
    pub intent_id: IntentId,
}

impl IntentChain {
    /// Whether the chain has full provenance (observations + meanings + intent).
    pub fn has_full_provenance(&self) -> bool {
        !self.observation_ids.is_empty() && !self.meaning_ids.is_empty()
    }

    /// Total length of the provenance chain.
    pub fn chain_length(&self) -> usize {
        self.observation_ids.len() + self.meaning_ids.len() + 1 // +1 for intent
    }
}

// ── Validation Criterion ────────────────────────────────────────────────

/// A criterion that must be satisfied before a commitment is fulfilled.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationCriterion {
    /// Criterion name.
    pub name: String,
    /// What this criterion validates.
    pub description: String,
    /// Whether failure of this criterion blocks deployment.
    pub mandatory: bool,
}

// ── Self-Modification Commitment ────────────────────────────────────────

/// A self-modification commitment — an authorized, scoped modification.
///
/// This is the central type of the self-modification gate. It bundles
/// all metadata needed to evaluate, approve, execute, and audit a
/// self-modification.
///
/// **Invariant**: A commitment CANNOT be created without a rollback plan.
/// This is enforced by the `new()` constructor which requires `RollbackPlan`.
#[derive(Clone, Debug)]
pub struct SelfModificationCommitment {
    /// Unique commitment identifier.
    pub id: String,
    /// The regeneration proposal specifying what changes to make.
    pub proposal: RegenerationProposal,
    /// Self-modification tier (determines approval requirements).
    pub tier: SelfModTier,
    /// Deployment strategy.
    pub deployment: DeploymentStrategy,
    /// Rollback plan (MANDATORY — structurally enforced by constructor).
    pub rollback_plan: RollbackPlan,
    /// Validation criteria that must pass before fulfillment.
    pub validation_criteria: Vec<ValidationCriterion>,
    /// Maximum deployment duration (seconds) — auto-rollback if exceeded.
    pub max_deployment_duration_secs: u64,
    /// Full provenance chain from observations to this commitment.
    pub intent_chain: IntentChain,
    /// When this commitment was created.
    pub created_at: DateTime<Utc>,
}

impl SelfModificationCommitment {
    /// Create a new self-modification commitment.
    ///
    /// Validates:
    /// - Rollback plan has at least one step
    /// - Deployment strategy is compatible with the tier
    /// - Creates default validation criteria for the tier
    pub fn new(
        proposal: RegenerationProposal,
        tier: SelfModTier,
        deployment: DeploymentStrategy,
        rollback_plan: RollbackPlan,
        intent_chain: IntentChain,
    ) -> SelfModGateResult<Self> {
        // Validate rollback plan has steps
        if rollback_plan.steps.is_empty() {
            return Err(SelfModGateError::RollbackPlanInvalid(
                "Rollback plan must have at least one step".into(),
            ));
        }

        // Validate deployment strategy matches tier
        deployment.validate_for_tier(&tier)?;

        // Create default validation criteria
        let validation_criteria = Self::default_criteria(&tier);
        let max_deployment_duration_secs = tier.default_max_deployment_secs();

        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            proposal,
            tier,
            deployment,
            rollback_plan,
            validation_criteria,
            max_deployment_duration_secs,
            intent_chain,
            created_at: Utc::now(),
        })
    }

    /// Default validation criteria for a given tier.
    fn default_criteria(tier: &SelfModTier) -> Vec<ValidationCriterion> {
        let mut criteria = vec![
            ValidationCriterion {
                name: "compilation".into(),
                description: "Modified code compiles successfully".into(),
                mandatory: true,
            },
            ValidationCriterion {
                name: "existing_tests".into(),
                description: "All existing tests pass".into(),
                mandatory: true,
            },
        ];

        if tier.requires_governance_review() {
            criteria.push(ValidationCriterion {
                name: "governance_approval".into(),
                description: "Governance review approved".into(),
                mandatory: true,
            });
        }

        if tier.requires_human_review() {
            criteria.push(ValidationCriterion {
                name: "human_approval".into(),
                description: "Human reviewer approved".into(),
                mandatory: true,
            });
        }

        criteria.push(ValidationCriterion {
            name: "performance_benchmark".into(),
            description: "No performance regression beyond threshold".into(),
            mandatory: !tier.is_auto_approve(),
        });

        criteria
    }

    /// Affected file paths from the proposal.
    pub fn affected_files(&self) -> Vec<String> {
        self.proposal
            .code_changes
            .iter()
            .map(|c| c.file_path.clone())
            .collect()
    }

    /// Affected components from the proposal.
    pub fn affected_components(&self) -> &[String] {
        &self.proposal.affected_components
    }
}

impl std::fmt::Display for SelfModificationCommitment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} (tier={}, deployment={}, {} changes)",
            &self.id[..8],
            self.proposal.summary,
            self.tier,
            self.deployment,
            self.proposal.code_changes.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_intent::intent::ImprovementEstimate;
    use maple_worldline_intent::proposal::*;
    use maple_worldline_intent::types::{CodeChangeType, ProposalId};

    fn make_intent_chain() -> IntentChain {
        IntentChain {
            observation_ids: vec!["obs-1".into(), "obs-2".into()],
            meaning_ids: vec![MeaningId::new()],
            intent_id: IntentId::new(),
        }
    }

    fn make_proposal() -> RegenerationProposal {
        RegenerationProposal {
            id: ProposalId::new(),
            summary: "Optimize config loading".into(),
            rationale: "Reduce startup time".into(),
            affected_components: vec!["config".into()],
            code_changes: vec![CodeChangeSpec {
                file_path: "src/config.rs".into(),
                change_type: CodeChangeType::ModifyFunction {
                    function_name: "load".into(),
                },
                description: "Cache config".into(),
                affected_regions: vec!["load()".into()],
                provenance: vec![MeaningId::new()],
            }],
            required_tests: vec![TestSpec {
                name: "test_load".into(),
                description: "Verify loading".into(),
                test_type: TestType::Unit,
            }],
            performance_gates: vec![],
            safety_checks: vec![],
            estimated_improvement: ImprovementEstimate {
                metric: "startup_time".into(),
                current_value: 500.0,
                projected_value: 200.0,
                confidence: 0.9,
                unit: "ms".into(),
            },
            risk_score: 0.1,
            rollback_plan: RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec!["git revert HEAD".into()],
                estimated_duration_secs: 60,
            },
        }
    }

    #[test]
    fn commitment_creation_success() {
        let commitment = SelfModificationCommitment::new(
            make_proposal(),
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec!["git revert HEAD".into()],
                estimated_duration_secs: 60,
            },
            make_intent_chain(),
        );
        assert!(commitment.is_ok());
        let c = commitment.unwrap();
        assert!(!c.id.is_empty());
        assert_eq!(c.tier, SelfModTier::Tier0Configuration);
    }

    #[test]
    fn commitment_rejects_empty_rollback() {
        let result = SelfModificationCommitment::new(
            make_proposal(),
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec![], // Empty!
                estimated_duration_secs: 0,
            },
            make_intent_chain(),
        );
        assert!(matches!(
            result,
            Err(SelfModGateError::RollbackPlanInvalid(_))
        ));
    }

    #[test]
    fn commitment_rejects_tier_mismatch() {
        let result = SelfModificationCommitment::new(
            make_proposal(),
            SelfModTier::Tier3KernelChange, // High tier
            DeploymentStrategy::Immediate,  // But immediate deployment
            RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec!["git revert HEAD".into()],
                estimated_duration_secs: 60,
            },
            make_intent_chain(),
        );
        assert!(matches!(result, Err(SelfModGateError::TierMismatch(_))));
    }

    #[test]
    fn intent_chain_provenance() {
        let chain = make_intent_chain();
        assert!(chain.has_full_provenance());
        assert_eq!(chain.chain_length(), 4); // 2 obs + 1 meaning + 1 intent

        let empty_chain = IntentChain {
            observation_ids: vec![],
            meaning_ids: vec![],
            intent_id: IntentId::new(),
        };
        assert!(!empty_chain.has_full_provenance());
    }

    #[test]
    fn default_criteria_tier0() {
        let c = SelfModificationCommitment::new(
            make_proposal(),
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec!["revert".into()],
                estimated_duration_secs: 60,
            },
            make_intent_chain(),
        )
        .unwrap();
        // Tier0: compilation + existing_tests + performance (advisory)
        assert!(c.validation_criteria.len() >= 2);
    }

    #[test]
    fn default_criteria_tier3_includes_human_review() {
        let c = SelfModificationCommitment::new(
            make_proposal(),
            SelfModTier::Tier3KernelChange,
            DeploymentStrategy::Canary {
                traffic_fraction: 0.05,
            },
            RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec!["revert".into()],
                estimated_duration_secs: 60,
            },
            make_intent_chain(),
        )
        .unwrap();
        let names: Vec<&str> = c
            .validation_criteria
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert!(names.contains(&"human_approval"));
        assert!(names.contains(&"governance_approval"));
    }
}
