//! # maple-worldline-codegen
//!
//! **Code Generation Engine** for the WorldLine Self-Producing Substrate.
//!
//! Takes approved `SelfModificationCommitment` from the self-modification gate,
//! generates code per `CodeChangeSpec`, validates in a sandbox (compilation,
//! tests, performance gates), and produces `CodegenArtifact` for deployment.
//!
//! ## Architecture
//!
//! ```text
//! SelfModificationCommitment (approved)
//!     │
//!     ▼
//! CodegenEngine
//!     │─── validate commitment (PolicyDecisionCard)
//!     │─── generate code (CodeGenerator trait)
//!     │─── sandbox compile (SandboxCompiler trait)
//!     │─── validate tests (TestValidator)
//!     │─── validate performance (PerformanceValidator)
//!     │─── validate safety (SafetyValidator)
//!     ▼
//! CodegenArtifact (for deployment pipeline)
//! ```
//!
//! ## Traits
//!
//! - [`CodeGenerator`] — abstracts LLM-backed code generation
//! - [`SandboxCompiler`] — abstracts compilation, test execution, perf evaluation
//!
//! Both have simulated implementations for testing.

#![deny(unsafe_code)]

pub mod artifact;
pub mod engine;
pub mod error;
pub mod generator;
pub mod sandbox;
pub mod types;
pub mod validator;

// Re-exports
pub use artifact::{ArtifactBuilder, CodegenArtifact};
pub use engine::CodegenEngine;
pub use error::{CodegenError, CodegenResult};
pub use generator::{CodeGenerator, GenerationContext, SimulatedGenerator};
pub use sandbox::{SandboxCompiler, SimulatedSandbox};
pub use types::{
    CodegenConfig, CodegenId, CodegenStatus, CodegenSummary, CompilationResult, GeneratedCode,
    GenerationRecord, PerformanceResult, TestResult,
};
pub use validator::{
    PerformanceValidationSummary, PerformanceValidator, SafetyValidator, TestValidationSummary,
    TestValidator,
};

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_intent::intent::ImprovementEstimate;
    use maple_worldline_intent::proposal::*;
    use maple_worldline_intent::types::{CodeChangeType, IntentId, MeaningId, ProposalId};
    use maple_worldline_self_mod_gate::commitment::IntentChain;
    use maple_worldline_self_mod_gate::types::{
        DeploymentStrategy, PolicyDecisionCard, SelfModTier,
    };
    use maple_worldline_self_mod_gate::SelfModificationCommitment;

    fn make_commitment(
        files: Vec<(&str, CodeChangeType)>,
        tier: SelfModTier,
        deployment: DeploymentStrategy,
    ) -> SelfModificationCommitment {
        let changes: Vec<CodeChangeSpec> = files
            .into_iter()
            .map(|(path, ct)| CodeChangeSpec {
                file_path: path.into(),
                change_type: ct,
                description: "test change".into(),
                affected_regions: vec![],
                provenance: vec![MeaningId::new()],
            })
            .collect();

        SelfModificationCommitment::new(
            RegenerationProposal {
                id: ProposalId::new(),
                summary: "Integration test".into(),
                rationale: "Testing full pipeline".into(),
                affected_components: vec!["module".into()],
                code_changes: changes,
                required_tests: vec![TestSpec {
                    name: "test_it".into(),
                    description: "Test".into(),
                    test_type: TestType::Unit,
                }],
                performance_gates: vec![PerformanceGate {
                    metric: "latency_p99".into(),
                    threshold: 10.0,
                    comparison: Comparison::LessThan,
                }],
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
    fn integration_full_cycle_tier0() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::all_pass()),
        );

        let commitment = make_commitment(
            vec![("src/config.rs", CodeChangeType::ModifyFunction {
                function_name: "load".into(),
            })],
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
        );
        let decision = PolicyDecisionCard::approved();

        let artifact = engine.generate(&commitment, &decision).unwrap();
        assert!(artifact.is_deployable());
        assert_eq!(artifact.tier, SelfModTier::Tier0Configuration);
        assert_eq!(artifact.total_files, 1);
        assert_eq!(artifact.tests_passed, 1);
        assert_eq!(artifact.perf_gates_passed, 1);
    }

    #[test]
    fn integration_multi_change_proposal() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::all_pass()),
        );

        let commitment = make_commitment(
            vec![
                ("src/config.rs", CodeChangeType::ModifyFunction { function_name: "load".into() }),
                ("src/handler.rs", CodeChangeType::ModifyStruct { struct_name: "Handler".into() }),
                ("src/traits.rs", CodeChangeType::ModifyTrait { trait_name: "Execute".into() }),
                ("src/impl.rs", CodeChangeType::AddImplementation { trait_name: "Execute".into(), struct_name: "Handler".into() }),
                ("src/new_module.rs", CodeChangeType::CreateFile),
            ],
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
        );

        let artifact = engine.generate(&commitment, &PolicyDecisionCard::approved()).unwrap();
        assert_eq!(artifact.total_files, 5);
        assert!(artifact.is_deployable());
    }

    #[test]
    fn integration_provenance_flows_through() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::all_pass()),
        );

        let commitment = make_commitment(
            vec![("src/config.rs", CodeChangeType::ModifyFunction { function_name: "load".into() })],
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
        );

        let artifact = engine.generate(&commitment, &PolicyDecisionCard::approved()).unwrap();
        assert!(artifact.intent_chain.has_full_provenance());
        assert_eq!(artifact.intent_chain.observation_ids, vec!["obs-1"]);
    }

    #[test]
    fn integration_generation_failure_tracked() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(false)),
            Box::new(SimulatedSandbox::all_pass()),
        );

        let commitment = make_commitment(
            vec![("src/config.rs", CodeChangeType::ModifyFunction { function_name: "load".into() })],
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
        );

        let result = engine.generate(&commitment, &PolicyDecisionCard::approved());
        assert!(result.is_err());

        // Should be tracked in records
        let summary = engine.summary();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn integration_performance_gate_blocks() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::performance_fails()),
        );

        let commitment = make_commitment(
            vec![("src/config.rs", CodeChangeType::ModifyFunction { function_name: "load".into() })],
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
        );

        let result = engine.generate(&commitment, &PolicyDecisionCard::approved());
        assert!(matches!(result, Err(CodegenError::PerformanceGateFailed(_))));
    }

    #[test]
    fn integration_tier1_with_canary() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::all_pass()),
        );

        let commitment = make_commitment(
            vec![("src/operator.rs", CodeChangeType::ModifyFunction { function_name: "handle".into() })],
            SelfModTier::Tier1OperatorInternal,
            DeploymentStrategy::Canary { traffic_fraction: 0.05 },
        );

        let artifact = engine.generate(&commitment, &PolicyDecisionCard::approved()).unwrap();
        assert!(artifact.is_deployable());
        assert_eq!(artifact.tier, SelfModTier::Tier1OperatorInternal);
        assert!(matches!(artifact.deployment_strategy, DeploymentStrategy::Canary { .. }));
    }
}
