//! Intent generators — transform converged meanings into regeneration intents.
//!
//! Each generator specializes in a particular category of self-meaning and
//! produces a fully-formed `SelfRegenerationIntent` with a concrete proposal,
//! impact assessment, and rollback plan.

use chrono::Utc;

use maple_worldline_meaning::types::{SelfMeaning, SelfMeaningCategory};

use crate::intent::{
    ImpactAssessment, ImprovementEstimate, IntentStatus, SelfRegenerationIntent,
};
use crate::proposal::{
    CodeChangeSpec, PerformanceGate, Comparison, RegenerationProposal, RollbackPlan,
    RollbackStrategy, SafetyCheck, TestSpec, TestType,
};
use crate::types::{
    ChangeType, CodeChangeType, IntentId, ProposalId, ReversibilityLevel, SubstrateTier,
};

// ── Intent Generator Trait ─────────────────────────────────────────────

/// Trait for generating intents from converged meanings.
///
/// Each implementation handles a specific category of meaning and
/// produces fully-formed intents with proposals and rollback plans.
pub trait IntentGenerator {
    /// Attempt to generate an intent from a converged meaning.
    ///
    /// Returns `None` if this generator doesn't handle the meaning's category.
    fn generate(&self, meaning: &SelfMeaning) -> Option<SelfRegenerationIntent>;

    /// Generator name for tracing/debugging.
    fn name(&self) -> &str;
}

// ── Performance Intent Generator ───────────────────────────────────────

/// Generates intents from performance-related meanings.
///
/// Handles `PerformanceBottleneck` and `OperatorOptimization` categories.
pub struct PerformanceIntentGenerator;

impl IntentGenerator for PerformanceIntentGenerator {
    fn generate(&self, meaning: &SelfMeaning) -> Option<SelfRegenerationIntent> {
        match &meaning.category {
            SelfMeaningCategory::PerformanceBottleneck {
                component,
                severity,
                root_causes,
            } => {
                let change_type = if *severity > 0.7 {
                    ChangeType::OperatorModification {
                        operator_id: component.clone(),
                        modification_scope: "performance optimization".into(),
                    }
                } else {
                    ChangeType::ConfigurationChange {
                        parameter: format!("{}_config", component),
                        current_value: "default".into(),
                        proposed_value: "optimized".into(),
                        rationale: format!(
                            "Performance bottleneck severity {:.2} in {}",
                            severity, component
                        ),
                    }
                };

                let tier = if *severity > 0.7 {
                    SubstrateTier::Tier1
                } else {
                    SubstrateTier::Tier0
                };

                let risk_score = severity * 0.3;
                let root_cause_desc = if root_causes.is_empty() {
                    "unknown root cause".to_string()
                } else {
                    root_causes
                        .iter()
                        .map(|r| r.description.clone())
                        .collect::<Vec<_>>()
                        .join("; ")
                };

                let proposal = RegenerationProposal {
                    id: ProposalId::new(),
                    summary: format!("Optimize {} performance", component),
                    rationale: format!(
                        "Bottleneck detected (severity={:.2}): {}",
                        severity, root_cause_desc
                    ),
                    affected_components: vec![component.clone()],
                    code_changes: vec![CodeChangeSpec {
                        file_path: format!("src/{}/mod.rs", component),
                        change_type: CodeChangeType::ModifyFunction {
                            function_name: "process".into(),
                        },
                        description: format!("Optimize hot path in {}", component),
                        affected_regions: vec!["process()".into()],
                        provenance: vec![meaning.id.clone()],
                    }],
                    required_tests: vec![
                        TestSpec {
                            name: format!("test_{}_correctness", component),
                            description: "Verify correctness after optimization".into(),
                            test_type: TestType::Unit,
                        },
                        TestSpec {
                            name: format!("test_{}_perf", component),
                            description: "Verify performance improvement".into(),
                            test_type: TestType::Performance,
                        },
                    ],
                    performance_gates: vec![PerformanceGate {
                        metric: format!("{}_latency_p99", component),
                        threshold: 10.0 * (1.0 - severity),
                        comparison: Comparison::LessThan,
                    }],
                    safety_checks: vec![SafetyCheck {
                        invariant: "output_equivalence".into(),
                        description: "Outputs must match pre-optimization behavior".into(),
                    }],
                    estimated_improvement: ImprovementEstimate {
                        metric: "latency".into(),
                        current_value: 100.0,
                        projected_value: 100.0 * (1.0 - severity * 0.5),
                        confidence: meaning.confidence,
                        unit: "ms".into(),
                    },
                    risk_score,
                    rollback_plan: RollbackPlan {
                        strategy: RollbackStrategy::GitRevert,
                        steps: vec![
                            "git revert optimization commit".into(),
                            "redeploy affected component".into(),
                        ],
                        estimated_duration_secs: 600,
                    },
                };

                Some(SelfRegenerationIntent {
                    id: IntentId::new(),
                    derived_from: vec![meaning.id.clone()],
                    change_type,
                    proposal: proposal.clone(),
                    confidence: meaning.confidence,
                    reversibility: ReversibilityLevel::FullyReversible,
                    impact: ImpactAssessment {
                        affected_components: vec![component.clone()],
                        risk_score,
                        risk_factors: root_causes
                            .iter()
                            .map(|r| r.description.clone())
                            .collect(),
                        blast_radius: format!("{} subsystem", component),
                    },
                    governance_tier: tier,
                    estimated_improvement: proposal.estimated_improvement.clone(),
                    stabilized_at: Utc::now(),
                    status: IntentStatus::Forming,
                })
            }
            SelfMeaningCategory::OperatorOptimization {
                operator_id,
                bottleneck_type,
                ..
            } => {
                let proposal = RegenerationProposal {
                    id: ProposalId::new(),
                    summary: format!("Optimize operator {}", operator_id),
                    rationale: format!(
                        "Operator bottleneck ({:?}) detected in {}",
                        bottleneck_type, operator_id
                    ),
                    affected_components: vec![operator_id.clone()],
                    code_changes: vec![CodeChangeSpec {
                        file_path: format!("src/operators/{}.rs", operator_id),
                        change_type: CodeChangeType::ModifyFunction {
                            function_name: "execute".into(),
                        },
                        description: format!(
                            "Optimize {:?} bottleneck in {}",
                            bottleneck_type, operator_id
                        ),
                        affected_regions: vec!["execute()".into()],
                        provenance: vec![meaning.id.clone()],
                    }],
                    required_tests: vec![TestSpec {
                        name: format!("test_{}_optimization", operator_id),
                        description: "Verify operator behavior after optimization".into(),
                        test_type: TestType::Integration,
                    }],
                    performance_gates: vec![],
                    safety_checks: vec![SafetyCheck {
                        invariant: "operator_contract".into(),
                        description: "Operator must satisfy its type contract".into(),
                    }],
                    estimated_improvement: ImprovementEstimate {
                        metric: "operator_throughput".into(),
                        current_value: 1000.0,
                        projected_value: 1500.0,
                        confidence: meaning.confidence,
                        unit: "ops/s".into(),
                    },
                    risk_score: 0.2,
                    rollback_plan: RollbackPlan {
                        strategy: RollbackStrategy::OperatorRollback,
                        steps: vec![format!("rollback {} to previous version", operator_id)],
                        estimated_duration_secs: 300,
                    },
                };

                Some(SelfRegenerationIntent {
                    id: IntentId::new(),
                    derived_from: vec![meaning.id.clone()],
                    change_type: ChangeType::OperatorModification {
                        operator_id: operator_id.clone(),
                        modification_scope: "optimization".into(),
                    },
                    confidence: meaning.confidence,
                    reversibility: ReversibilityLevel::FullyReversible,
                    impact: ImpactAssessment {
                        affected_components: vec![operator_id.clone()],
                        risk_score: 0.2,
                        risk_factors: vec![],
                        blast_radius: format!("operator:{}", operator_id),
                    },
                    governance_tier: SubstrateTier::Tier1,
                    estimated_improvement: proposal.estimated_improvement.clone(),
                    stabilized_at: Utc::now(),
                    status: IntentStatus::Forming,
                    proposal,
                })
            }
            _ => None,
        }
    }

    fn name(&self) -> &str {
        "performance"
    }
}

// ── Capacity Intent Generator ──────────────────────────────────────────

/// Generates intents from capacity-related meanings.
///
/// Handles `CapacityForecast` and `MemoryOptimization` categories.
pub struct CapacityIntentGenerator;

impl IntentGenerator for CapacityIntentGenerator {
    fn generate(&self, meaning: &SelfMeaning) -> Option<SelfRegenerationIntent> {
        match &meaning.category {
            SelfMeaningCategory::CapacityForecast {
                resource,
                current_utilization,
                projected_exhaustion_hours,
                ..
            } => {
                let urgency = if let Some(hours) = projected_exhaustion_hours {
                    if *hours < 4.0 {
                        0.9
                    } else if *hours < 24.0 {
                        0.6
                    } else {
                        0.3
                    }
                } else {
                    0.2
                };

                let risk_score = current_utilization * 0.4 + urgency * 0.3;
                let tier = if urgency > 0.7 {
                    SubstrateTier::Tier2
                } else {
                    SubstrateTier::Tier0
                };

                let proposal = RegenerationProposal {
                    id: ProposalId::new(),
                    summary: format!("Address {} capacity concern", resource),
                    rationale: format!(
                        "Resource {} at {:.0}% utilization, projected exhaustion in {:.1?}h",
                        resource,
                        current_utilization * 100.0,
                        projected_exhaustion_hours,
                    ),
                    affected_components: vec![resource.clone()],
                    code_changes: vec![CodeChangeSpec {
                        file_path: format!("config/{}.toml", resource),
                        change_type: CodeChangeType::CreateFile,
                        description: format!("Adjust {} resource limits", resource),
                        affected_regions: vec![],
                        provenance: vec![meaning.id.clone()],
                    }],
                    required_tests: vec![TestSpec {
                        name: format!("test_{}_capacity", resource),
                        description: "Verify capacity change is effective".into(),
                        test_type: TestType::Integration,
                    }],
                    performance_gates: vec![PerformanceGate {
                        metric: format!("{}_utilization", resource),
                        threshold: 0.8,
                        comparison: Comparison::LessThan,
                    }],
                    safety_checks: vec![SafetyCheck {
                        invariant: "service_availability".into(),
                        description: "Service must remain available during change".into(),
                    }],
                    estimated_improvement: ImprovementEstimate {
                        metric: format!("{}_utilization", resource),
                        current_value: *current_utilization,
                        projected_value: current_utilization * 0.6,
                        confidence: meaning.confidence,
                        unit: "ratio".into(),
                    },
                    risk_score,
                    rollback_plan: RollbackPlan {
                        strategy: RollbackStrategy::ConfigRestore,
                        steps: vec![format!("restore {} config from backup", resource)],
                        estimated_duration_secs: 120,
                    },
                };

                Some(SelfRegenerationIntent {
                    id: IntentId::new(),
                    derived_from: vec![meaning.id.clone()],
                    change_type: ChangeType::ConfigurationChange {
                        parameter: format!("{}_limit", resource),
                        current_value: format!("{:.2}", current_utilization),
                        proposed_value: format!("{:.2}", current_utilization * 0.6),
                        rationale: format!("Capacity forecast for {}", resource),
                    },
                    confidence: meaning.confidence,
                    reversibility: ReversibilityLevel::FullyReversible,
                    impact: ImpactAssessment {
                        affected_components: vec![resource.clone()],
                        risk_score,
                        risk_factors: vec![format!(
                            "Current utilization: {:.0}%",
                            current_utilization * 100.0
                        )],
                        blast_radius: format!("{} subsystem", resource),
                    },
                    governance_tier: tier,
                    estimated_improvement: proposal.estimated_improvement.clone(),
                    stabilized_at: Utc::now(),
                    status: IntentStatus::Forming,
                    proposal,
                })
            }
            SelfMeaningCategory::MemoryOptimization {
                component,
                optimization_type,
                ..
            } => {
                let proposal = RegenerationProposal {
                    id: ProposalId::new(),
                    summary: format!("Optimize {} memory usage", component),
                    rationale: format!(
                        "Memory optimization ({:?}) identified in {}",
                        optimization_type, component
                    ),
                    affected_components: vec![component.clone()],
                    code_changes: vec![CodeChangeSpec {
                        file_path: format!("src/{}/memory.rs", component),
                        change_type: CodeChangeType::ModifyFunction {
                            function_name: "allocate".into(),
                        },
                        description: format!("Apply {:?} optimization", optimization_type),
                        affected_regions: vec!["allocate()".into()],
                        provenance: vec![meaning.id.clone()],
                    }],
                    required_tests: vec![TestSpec {
                        name: format!("test_{}_memory", component),
                        description: "Verify memory usage reduction".into(),
                        test_type: TestType::Performance,
                    }],
                    performance_gates: vec![],
                    safety_checks: vec![],
                    estimated_improvement: ImprovementEstimate {
                        metric: "memory_usage".into(),
                        current_value: 1024.0,
                        projected_value: 768.0,
                        confidence: meaning.confidence,
                        unit: "MB".into(),
                    },
                    risk_score: 0.15,
                    rollback_plan: RollbackPlan {
                        strategy: RollbackStrategy::GitRevert,
                        steps: vec!["git revert memory optimization".into()],
                        estimated_duration_secs: 300,
                    },
                };

                Some(SelfRegenerationIntent {
                    id: IntentId::new(),
                    derived_from: vec![meaning.id.clone()],
                    change_type: ChangeType::KernelModification {
                        module: component.clone(),
                        modification_scope: "memory optimization".into(),
                    },
                    confidence: meaning.confidence,
                    reversibility: ReversibilityLevel::FullyReversible,
                    impact: ImpactAssessment {
                        affected_components: vec![component.clone()],
                        risk_score: 0.15,
                        risk_factors: vec![],
                        blast_radius: format!("{} memory subsystem", component),
                    },
                    governance_tier: SubstrateTier::Tier2,
                    estimated_improvement: proposal.estimated_improvement.clone(),
                    stabilized_at: Utc::now(),
                    status: IntentStatus::Forming,
                    proposal,
                })
            }
            _ => None,
        }
    }

    fn name(&self) -> &str {
        "capacity"
    }
}

// ── Code Quality Intent Generator ──────────────────────────────────────

/// Generates intents from code quality meanings.
///
/// Handles `CodeSimplification` category.
pub struct CodeQualityIntentGenerator;

impl IntentGenerator for CodeQualityIntentGenerator {
    fn generate(&self, meaning: &SelfMeaning) -> Option<SelfRegenerationIntent> {
        match &meaning.category {
            SelfMeaningCategory::CodeSimplification {
                component,
                redundancy_type,
                ..
            } => {
                let proposal = RegenerationProposal {
                    id: ProposalId::new(),
                    summary: format!("Simplify code in {}", component),
                    rationale: format!(
                        "Code redundancy ({:?}) detected in {}",
                        redundancy_type, component
                    ),
                    affected_components: vec![component.clone()],
                    code_changes: vec![CodeChangeSpec {
                        file_path: format!("src/{}/mod.rs", component),
                        change_type: CodeChangeType::RefactorModule {
                            module_name: component.clone(),
                        },
                        description: format!(
                            "Remove {:?} redundancy in {}",
                            redundancy_type, component
                        ),
                        affected_regions: vec![component.clone()],
                        provenance: vec![meaning.id.clone()],
                    }],
                    required_tests: vec![
                        TestSpec {
                            name: format!("test_{}_regression", component),
                            description: "Regression test after refactoring".into(),
                            test_type: TestType::Integration,
                        },
                    ],
                    performance_gates: vec![],
                    safety_checks: vec![SafetyCheck {
                        invariant: "behavior_preservation".into(),
                        description: "Refactoring must not change observable behavior".into(),
                    }],
                    estimated_improvement: ImprovementEstimate {
                        metric: "code_complexity".into(),
                        current_value: 100.0,
                        projected_value: 70.0,
                        confidence: meaning.confidence,
                        unit: "cyclomatic".into(),
                    },
                    risk_score: 0.1,
                    rollback_plan: RollbackPlan {
                        strategy: RollbackStrategy::GitRevert,
                        steps: vec!["git revert refactoring commit".into()],
                        estimated_duration_secs: 180,
                    },
                };

                Some(SelfRegenerationIntent {
                    id: IntentId::new(),
                    derived_from: vec![meaning.id.clone()],
                    change_type: ChangeType::KernelModification {
                        module: component.clone(),
                        modification_scope: "code simplification".into(),
                    },
                    confidence: meaning.confidence,
                    reversibility: ReversibilityLevel::FullyReversible,
                    impact: ImpactAssessment {
                        affected_components: vec![component.clone()],
                        risk_score: 0.1,
                        risk_factors: vec![],
                        blast_radius: format!("{} module", component),
                    },
                    governance_tier: SubstrateTier::Tier2,
                    estimated_improvement: proposal.estimated_improvement.clone(),
                    stabilized_at: Utc::now(),
                    status: IntentStatus::Forming,
                    proposal,
                })
            }
            _ => None,
        }
    }

    fn name(&self) -> &str {
        "code-quality"
    }
}

// ── Architecture Intent Generator ──────────────────────────────────────

/// Generates intents from architectural insights.
///
/// Handles `ArchitecturalInsight` category. These are the highest tier
/// changes requiring the most scrutiny.
pub struct ArchitectureIntentGenerator;

impl IntentGenerator for ArchitectureIntentGenerator {
    fn generate(&self, meaning: &SelfMeaning) -> Option<SelfRegenerationIntent> {
        match &meaning.category {
            SelfMeaningCategory::ArchitecturalInsight {
                insight_type,
                affected_components,
                structural_pressure,
            } => {
                let proposal = RegenerationProposal {
                    id: ProposalId::new(),
                    summary: format!("Architectural restructuring: {:?}", insight_type),
                    rationale: format!(
                        "Structural pressure '{}' affecting {} components",
                        structural_pressure,
                        affected_components.len(),
                    ),
                    affected_components: affected_components.clone(),
                    code_changes: affected_components
                        .iter()
                        .map(|c| CodeChangeSpec {
                            file_path: format!("src/{}/mod.rs", c),
                            change_type: CodeChangeType::RefactorModule {
                                module_name: c.clone(),
                            },
                            description: format!("Restructure {} for {:?}", c, insight_type),
                            affected_regions: vec![c.clone()],
                            provenance: vec![meaning.id.clone()],
                        })
                        .collect(),
                    required_tests: vec![
                        TestSpec {
                            name: "test_architecture_integration".into(),
                            description: "Full integration test post-restructuring".into(),
                            test_type: TestType::Integration,
                        },
                        TestSpec {
                            name: "test_architecture_safety".into(),
                            description: "Safety invariants after restructuring".into(),
                            test_type: TestType::Safety,
                        },
                    ],
                    performance_gates: vec![PerformanceGate {
                        metric: "system_latency_p99".into(),
                        threshold: 50.0,
                        comparison: Comparison::LessThan,
                    }],
                    safety_checks: vec![
                        SafetyCheck {
                            invariant: "data_integrity".into(),
                            description: "No data loss during restructuring".into(),
                        },
                        SafetyCheck {
                            invariant: "api_compatibility".into(),
                            description: "External API must remain compatible".into(),
                        },
                    ],
                    estimated_improvement: ImprovementEstimate {
                        metric: "coupling_score".into(),
                        current_value: 0.8,
                        projected_value: 0.4,
                        confidence: meaning.confidence * 0.8,
                        unit: "ratio".into(),
                    },
                    risk_score: 0.5,
                    rollback_plan: RollbackPlan {
                        strategy: RollbackStrategy::FullRedeploy,
                        steps: vec![
                            "git revert all restructuring commits".into(),
                            "full integration test suite".into(),
                            "staged redeployment".into(),
                        ],
                        estimated_duration_secs: 3600,
                    },
                };

                Some(SelfRegenerationIntent {
                    id: IntentId::new(),
                    derived_from: vec![meaning.id.clone()],
                    change_type: ChangeType::ArchitecturalChange {
                        description: format!("{:?}", insight_type),
                        affected_modules: affected_components.clone(),
                        migration_plan: format!(
                            "Staged migration addressing '{}'",
                            structural_pressure
                        ),
                    },
                    confidence: meaning.confidence,
                    reversibility: ReversibilityLevel::ConditionallyReversible {
                        conditions: vec![
                            "No external consumers affected".into(),
                            "Migration completed within window".into(),
                        ],
                    },
                    impact: ImpactAssessment {
                        affected_components: affected_components.clone(),
                        risk_score: 0.5,
                        risk_factors: vec![
                            format!("Affects {} components", affected_components.len()),
                            format!("Structural pressure: {}", structural_pressure),
                        ],
                        blast_radius: format!(
                            "{} components: {}",
                            affected_components.len(),
                            affected_components.join(", ")
                        ),
                    },
                    governance_tier: SubstrateTier::Tier3,
                    estimated_improvement: proposal.estimated_improvement.clone(),
                    stabilized_at: Utc::now(),
                    status: IntentStatus::Forming,
                    proposal,
                })
            }
            _ => None,
        }
    }

    fn name(&self) -> &str {
        "architecture"
    }
}

/// Create the default set of intent generators.
pub fn default_generators() -> Vec<Box<dyn IntentGenerator>> {
    vec![
        Box::new(PerformanceIntentGenerator),
        Box::new(CapacityIntentGenerator),
        Box::new(CodeQualityIntentGenerator),
        Box::new(ArchitectureIntentGenerator),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_meaning::types::{
        ArchitecturalInsightType, Evidence, EvidenceCategory, GrowthModel, MeaningId,
        MemoryOptimizationType, OperatorBottleneckType, RedundancyType,
    };

    fn make_evidence(n: usize) -> Vec<Evidence> {
        (0..n)
            .map(|i| Evidence {
                source: format!("source-{}", i),
                strength: 0.7,
                timestamp: Utc::now(),
                description: format!("evidence {}", i),
                category: EvidenceCategory::Anomaly,
            })
            .collect()
    }

    fn make_meaning(category: SelfMeaningCategory, confidence: f64) -> SelfMeaning {
        SelfMeaning {
            id: MeaningId::new(),
            category,
            evidence: make_evidence(15),
            confidence,
            ambiguity: 0.1,
            formed_at: Utc::now(),
            temporal_stability_secs: 7200.0,
            competing_with: vec![],
            converged: true,
        }
    }

    #[test]
    fn performance_generator_bottleneck() {
        let gen = PerformanceIntentGenerator;
        let meaning = make_meaning(
            SelfMeaningCategory::PerformanceBottleneck {
                component: "gate".into(),
                severity: 0.8,
                root_causes: vec![],
            },
            0.9,
        );
        let intent = gen.generate(&meaning).expect("should generate");
        assert!(matches!(
            intent.governance_tier,
            SubstrateTier::Tier1
        ));
        assert!(intent.confidence > 0.8);
    }

    #[test]
    fn performance_generator_operator_optimization() {
        let gen = PerformanceIntentGenerator;
        let meaning = make_meaning(
            SelfMeaningCategory::OperatorOptimization {
                operator_id: "scan".into(),
                bottleneck_type: OperatorBottleneckType::AlgorithmicComplexity,
                improvement_hypothesis: "Replace linear scan with indexed lookup".into(),
            },
            0.85,
        );
        let intent = gen.generate(&meaning).expect("should generate");
        assert_eq!(intent.governance_tier, SubstrateTier::Tier1);
        assert!(intent.proposal.has_rollback());
    }

    #[test]
    fn performance_generator_ignores_unrelated() {
        let gen = PerformanceIntentGenerator;
        let meaning = make_meaning(
            SelfMeaningCategory::ApiDesignInsight {
                pattern: "test".into(),
                improvement_direction: "test".into(),
            },
            0.9,
        );
        assert!(gen.generate(&meaning).is_none());
    }

    #[test]
    fn capacity_generator_forecast() {
        let gen = CapacityIntentGenerator;
        let meaning = make_meaning(
            SelfMeaningCategory::CapacityForecast {
                resource: "memory".into(),
                current_utilization: 0.9,
                projected_exhaustion_hours: Some(3.0),
                growth_model: GrowthModel::Exponential,
            },
            0.85,
        );
        let intent = gen.generate(&meaning).expect("should generate");
        // High urgency (< 4h) → Tier2
        assert_eq!(intent.governance_tier, SubstrateTier::Tier2);
    }

    #[test]
    fn capacity_generator_memory_optimization() {
        let gen = CapacityIntentGenerator;
        let meaning = make_meaning(
            SelfMeaningCategory::MemoryOptimization {
                component: "buffer_pool".into(),
                optimization_type: MemoryOptimizationType::CacheResizing,
                estimated_improvement: 0.25,
            },
            0.8,
        );
        let intent = gen.generate(&meaning).expect("should generate");
        assert_eq!(intent.governance_tier, SubstrateTier::Tier2);
    }

    #[test]
    fn code_quality_generator() {
        let gen = CodeQualityIntentGenerator;
        let meaning = make_meaning(
            SelfMeaningCategory::CodeSimplification {
                component: "parser".into(),
                dead_paths: vec!["unused_handler".into()],
                redundancy_type: RedundancyType::DuplicateLogic,
            },
            0.85,
        );
        let intent = gen.generate(&meaning).expect("should generate");
        assert_eq!(intent.governance_tier, SubstrateTier::Tier2);
        assert!(intent.proposal.has_safety_checks());
    }

    #[test]
    fn architecture_generator() {
        let gen = ArchitectureIntentGenerator;
        let meaning = make_meaning(
            SelfMeaningCategory::ArchitecturalInsight {
                insight_type: ArchitecturalInsightType::CouplingTooTight,
                affected_components: vec!["gate".into(), "fabric".into()],
                structural_pressure: "high coupling".into(),
            },
            0.9,
        );
        let intent = gen.generate(&meaning).expect("should generate");
        assert_eq!(intent.governance_tier, SubstrateTier::Tier3);
        assert_eq!(intent.proposal.code_changes.len(), 2); // one per component
        assert!(matches!(
            intent.reversibility,
            ReversibilityLevel::ConditionallyReversible { .. }
        ));
    }

    #[test]
    fn default_generators_all_four() {
        let gens = default_generators();
        assert_eq!(gens.len(), 4);
        let names: Vec<&str> = gens.iter().map(|g| g.name()).collect();
        assert!(names.contains(&"performance"));
        assert!(names.contains(&"capacity"));
        assert!(names.contains(&"code-quality"));
        assert!(names.contains(&"architecture"));
    }
}
