//! Declaration mapper — translates `SelfRegenerationIntent` into
//! `CommitmentDeclaration` compatible with the commitment gate.
//!
//! The mapper handles the type-level translation between the intent world
//! (governed by substrate tiers, meaning provenance) and the gate world
//! (governed by effect domains, capability refs, confidence profiles).

use maple_kernel_gate::CommitmentDeclaration;
use maple_mwl_types::{
    CapabilityId, CommitmentScope, ConfidenceProfile, EffectDomain, EventId,
    Reversibility, WorldlineId,
};
use maple_worldline_intent::intent::SelfRegenerationIntent;
use maple_worldline_intent::types::{ChangeType, ReversibilityLevel, SubstrateTier};

use crate::error::CommitmentResult;

// ── Declaration Mapper ──────────────────────────────────────────────────

/// Maps `SelfRegenerationIntent` into `CommitmentDeclaration` for the gate.
pub struct DeclarationMapper {
    /// The worldline identity declaring commitments.
    pub worldline_id: WorldlineId,
    /// Default capability grants for this worldline.
    pub default_capabilities: Vec<CapabilityId>,
}

impl DeclarationMapper {
    /// Create a new mapper with the given worldline identity.
    pub fn new(worldline_id: WorldlineId) -> Self {
        Self {
            worldline_id,
            default_capabilities: vec![
                CapabilityId("CAP-SELF-MODIFY".into()),
                CapabilityId("CAP-CONFIG".into()),
            ],
        }
    }

    /// Map a stabilized intent into a commitment declaration.
    pub fn map_intent(
        &self,
        intent: &SelfRegenerationIntent,
    ) -> CommitmentResult<CommitmentDeclaration> {
        let effect_domain = map_effect_domain(&intent.change_type);
        let scope = CommitmentScope {
            effect_domain,
            targets: vec![self.worldline_id.clone()], // self-modification targets self
            constraints: self.build_constraints(intent),
        };

        let confidence = map_confidence(intent.confidence, &intent.governance_tier);
        let reversibility = map_reversibility(&intent.reversibility);
        let evidence = self.build_evidence(intent);

        let capabilities = self.capabilities_for_change(&intent.change_type);

        let decl = CommitmentDeclaration::builder(self.worldline_id.clone(), scope)
            .derived_from_intent(EventId::new()) // link to intent's event
            .confidence(confidence)
            .reversibility(reversibility)
            .capabilities(capabilities)
            .build();

        // Attach evidence via a fresh builder (evidence added one at a time)
        let mut builder = CommitmentDeclaration::builder(
            decl.declaring_identity.clone(),
            decl.scope.clone(),
        )
        .derived_from_intent(decl.derived_from_intent.unwrap_or_else(EventId::new))
        .confidence(decl.confidence.clone())
        .reversibility(decl.reversibility.clone())
        .capabilities(decl.capability_refs.clone());

        for e in &evidence {
            builder = builder.evidence(e.as_str());
        }

        Ok(builder.build())
    }

    /// Build constraints from the intent's governance tier and proposal.
    fn build_constraints(&self, intent: &SelfRegenerationIntent) -> Vec<String> {
        let mut constraints = vec![
            format!("governance_tier:{}", intent.governance_tier),
            format!(
                "min_observation_secs:{}",
                intent.governance_tier.min_observation_secs()
            ),
        ];

        if intent.proposal.has_rollback() {
            constraints.push("rollback_plan:present".into());
        }

        if intent.proposal.has_safety_checks() {
            constraints.push("safety_checks:present".into());
        }

        constraints
    }

    /// Build evidence trail from intent provenance.
    fn build_evidence(&self, intent: &SelfRegenerationIntent) -> Vec<String> {
        let mut evidence = vec![
            format!("intent_id:{}", intent.id),
            format!("confidence:{:.2}", intent.confidence),
            format!("risk_score:{:.2}", intent.impact.risk_score),
            format!("change_type:{}", intent.change_type),
            format!("blast_radius:{}", intent.impact.blast_radius),
        ];

        for factor in &intent.impact.risk_factors {
            evidence.push(format!("risk_factor:{}", factor));
        }

        // Include meaning provenance
        for mid in &intent.derived_from {
            evidence.push(format!("derived_from_meaning:{}", mid));
        }

        evidence.push(format!("proposal_rationale:{}", intent.proposal.rationale));
        evidence
    }

    /// Determine required capabilities based on change type.
    fn capabilities_for_change(&self, change_type: &ChangeType) -> Vec<CapabilityId> {
        let mut caps = self.default_capabilities.clone();

        match change_type {
            ChangeType::ConfigurationChange { .. } => {
                caps.push(CapabilityId("CAP-CONFIG-WRITE".into()));
            }
            ChangeType::OperatorModification { .. } | ChangeType::NewOperator { .. } => {
                caps.push(CapabilityId("CAP-OPERATOR-MODIFY".into()));
            }
            ChangeType::KernelModification { .. } => {
                caps.push(CapabilityId("CAP-KERNEL-MODIFY".into()));
            }
            ChangeType::ArchitecturalChange { .. } => {
                caps.push(CapabilityId("CAP-ARCHITECTURE-MODIFY".into()));
            }
            ChangeType::ApiModification { .. } => {
                caps.push(CapabilityId("CAP-API-MODIFY".into()));
            }
            ChangeType::DslGeneration { .. } => {
                caps.push(CapabilityId("CAP-DSL-GENERATE".into()));
            }
            ChangeType::CompilationStrategy { .. } => {
                caps.push(CapabilityId("CAP-COMPILATION-MODIFY".into()));
            }
        }

        caps
    }
}

// ── Mapping Helpers ─────────────────────────────────────────────────────

/// Map a `ChangeType` to an `EffectDomain`.
pub fn map_effect_domain(change_type: &ChangeType) -> EffectDomain {
    match change_type {
        ChangeType::ConfigurationChange { .. } => EffectDomain::Infrastructure,
        ChangeType::OperatorModification { .. } => EffectDomain::Infrastructure,
        ChangeType::NewOperator { .. } => EffectDomain::Infrastructure,
        ChangeType::KernelModification { .. } => EffectDomain::DataMutation,
        ChangeType::ApiModification { .. } => EffectDomain::Communication,
        ChangeType::ArchitecturalChange { .. } => EffectDomain::Infrastructure,
        ChangeType::DslGeneration { .. } => EffectDomain::Custom("dsl-generation".into()),
        ChangeType::CompilationStrategy { .. } => EffectDomain::Custom("compilation".into()),
    }
}

/// Map a `ReversibilityLevel` to a gate-compatible `Reversibility`.
pub fn map_reversibility(level: &ReversibilityLevel) -> Reversibility {
    match level {
        ReversibilityLevel::FullyReversible => Reversibility::FullyReversible,
        ReversibilityLevel::ConditionallyReversible { conditions } => {
            Reversibility::Conditional {
                conditions: conditions.clone(),
            }
        }
        ReversibilityLevel::TimeWindowReversible { window_secs } => {
            Reversibility::TimeWindow {
                window_ms: window_secs * 1000,
            }
        }
        ReversibilityLevel::Irreversible => Reversibility::Irreversible,
    }
}

/// Map intent confidence + governance tier to a `ConfidenceProfile`.
pub fn map_confidence(confidence: f64, tier: &SubstrateTier) -> ConfidenceProfile {
    // Higher tiers require more stability, so we scale stability proportionally
    let tier_factor = match tier {
        SubstrateTier::Tier0 => 0.8,
        SubstrateTier::Tier1 => 0.85,
        SubstrateTier::Tier2 => 0.9,
        SubstrateTier::Tier3 => 0.95,
    };

    ConfidenceProfile::new(
        confidence,             // overall confidence from intent
        confidence * tier_factor, // stability scales with tier
        confidence * 0.9,       // signal consistency
        confidence * 0.85,      // historical alignment
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use maple_mwl_types::IdentityMaterial;
    use maple_worldline_intent::intent::{ImpactAssessment, ImprovementEstimate, IntentStatus};
    use maple_worldline_intent::proposal::{
        RegenerationProposal, RollbackPlan, RollbackStrategy,
    };
    use maple_worldline_intent::types::{IntentId, ProposalId};
    use maple_worldline_intent::types::MeaningId;

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn make_intent(change_type: ChangeType) -> SelfRegenerationIntent {
        SelfRegenerationIntent {
            id: IntentId::new(),
            derived_from: vec![MeaningId::new()],
            change_type,
            proposal: RegenerationProposal {
                id: ProposalId::new(),
                summary: "test proposal".into(),
                rationale: "test rationale".into(),
                affected_components: vec!["test".into()],
                code_changes: vec![],
                required_tests: vec![],
                performance_gates: vec![],
                safety_checks: vec![],
                estimated_improvement: ImprovementEstimate {
                    metric: "latency".into(),
                    current_value: 100.0,
                    projected_value: 80.0,
                    confidence: 0.9,
                    unit: "ms".into(),
                },
                risk_score: 0.2,
                rollback_plan: RollbackPlan {
                    strategy: RollbackStrategy::ConfigRestore,
                    steps: vec!["restore config".into()],
                    estimated_duration_secs: 60,
                },
            },
            confidence: 0.9,
            reversibility: ReversibilityLevel::FullyReversible,
            impact: ImpactAssessment {
                affected_components: vec!["test".into()],
                risk_score: 0.2,
                risk_factors: vec!["minor risk".into()],
                blast_radius: "test only".into(),
            },
            governance_tier: SubstrateTier::Tier0,
            estimated_improvement: ImprovementEstimate {
                metric: "latency".into(),
                current_value: 100.0,
                projected_value: 80.0,
                confidence: 0.9,
                unit: "ms".into(),
            },
            stabilized_at: Utc::now(),
            status: IntentStatus::Validated,
        }
    }

    #[test]
    fn map_config_change_intent() {
        let mapper = DeclarationMapper::new(test_worldline());
        let intent = make_intent(ChangeType::ConfigurationChange {
            parameter: "batch_size".into(),
            current_value: "32".into(),
            proposed_value: "64".into(),
            rationale: "improve throughput".into(),
        });

        let decl = mapper.map_intent(&intent).unwrap();
        assert!(decl.derived_from_intent.is_some());
        assert_eq!(decl.scope.effect_domain, EffectDomain::Infrastructure);
        assert!(!decl.evidence.is_empty());
    }

    #[test]
    fn map_kernel_modification() {
        let mapper = DeclarationMapper::new(test_worldline());
        let intent = make_intent(ChangeType::KernelModification {
            module: "scheduler".into(),
            modification_scope: "optimization".into(),
        });

        let decl = mapper.map_intent(&intent).unwrap();
        assert_eq!(decl.scope.effect_domain, EffectDomain::DataMutation);
    }

    #[test]
    fn map_effect_domain_all_variants() {
        assert_eq!(
            map_effect_domain(&ChangeType::ConfigurationChange {
                parameter: "a".into(),
                current_value: "1".into(),
                proposed_value: "2".into(),
                rationale: "c".into(),
            }),
            EffectDomain::Infrastructure
        );
        assert_eq!(
            map_effect_domain(&ChangeType::KernelModification {
                module: "a".into(),
                modification_scope: "b".into(),
            }),
            EffectDomain::DataMutation
        );
        assert_eq!(
            map_effect_domain(&ChangeType::ApiModification {
                api_component: "a".into(),
                breaking: false,
                migration_plan: None,
            }),
            EffectDomain::Communication
        );
    }

    #[test]
    fn map_reversibility_all_variants() {
        assert!(matches!(
            map_reversibility(&ReversibilityLevel::FullyReversible),
            Reversibility::FullyReversible
        ));
        assert!(matches!(
            map_reversibility(&ReversibilityLevel::ConditionallyReversible {
                conditions: vec![]
            }),
            Reversibility::Conditional { .. }
        ));
        assert!(matches!(
            map_reversibility(&ReversibilityLevel::TimeWindowReversible { window_secs: 3600 }),
            Reversibility::TimeWindow { window_ms: 3600000 }
        ));
        assert!(matches!(
            map_reversibility(&ReversibilityLevel::Irreversible),
            Reversibility::Irreversible
        ));
    }

    #[test]
    fn map_confidence_scales_with_tier() {
        let t0 = map_confidence(0.9, &SubstrateTier::Tier0);
        let t3 = map_confidence(0.9, &SubstrateTier::Tier3);

        // Higher tier → higher stability requirement
        assert!(t3.stability > t0.stability);
        // Both have same overall confidence
        assert!((t0.overall - 0.9).abs() < f64::EPSILON);
        assert!((t3.overall - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn capabilities_include_change_specific() {
        let mapper = DeclarationMapper::new(test_worldline());

        let caps = mapper.capabilities_for_change(&ChangeType::ArchitecturalChange {
            description: "a".into(),
            affected_modules: vec![],
            migration_plan: "b".into(),
        });
        assert!(caps.iter().any(|c| c.0 == "CAP-ARCHITECTURE-MODIFY"));
        assert!(caps.iter().any(|c| c.0 == "CAP-SELF-MODIFY"));
    }
}
