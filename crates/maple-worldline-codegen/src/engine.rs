//! Codegen engine — orchestrates the full code generation pipeline.
//!
//! Pipeline:
//! 1. Validate commitment (must be approved, has valid provenance)
//! 2. Generate code for each CodeChangeSpec
//! 3. Sandbox compile each generated file
//! 4. Run tests in sandbox
//! 5. Evaluate performance gates
//! 6. Validate safety checks
//! 7. Assemble deployment artifact

use std::collections::VecDeque;

use maple_worldline_self_mod_gate::commitment::SelfModificationCommitment;
use maple_worldline_self_mod_gate::types::PolicyDecisionCard;

use crate::artifact::ArtifactBuilder;
use crate::error::{CodegenError, CodegenResult};
use crate::generator::{CodeGenerator, GenerationContext};
use crate::sandbox::SandboxCompiler;
use crate::types::{CodegenConfig, CodegenId, CodegenSummary, CodegenStatus, GenerationRecord};
use crate::validator::{PerformanceValidator, SafetyValidator, TestValidator};

/// The code generation engine.
///
/// Orchestrates the full codegen pipeline: validate → generate →
/// compile → test → performance → safety → artifact.
pub struct CodegenEngine {
    /// Code generator implementation.
    generator: Box<dyn CodeGenerator>,
    /// Sandbox compiler implementation.
    sandbox: Box<dyn SandboxCompiler>,
    /// Engine configuration.
    config: CodegenConfig,
    /// Bounded FIFO queue of generation records.
    records: VecDeque<GenerationRecord>,
}

impl CodegenEngine {
    /// Create a new codegen engine.
    pub fn new(
        generator: Box<dyn CodeGenerator>,
        sandbox: Box<dyn SandboxCompiler>,
    ) -> Self {
        Self {
            generator,
            sandbox,
            config: CodegenConfig::default(),
            records: VecDeque::new(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(mut self, config: CodegenConfig) -> Self {
        self.config = config;
        self
    }

    /// Generate code for an approved commitment.
    ///
    /// Full pipeline:
    /// 1. Validate commitment approval
    /// 2. Validate provenance chain
    /// 3. Generate code per CodeChangeSpec
    /// 4. Sandbox compile each file
    /// 5. Run tests
    /// 6. Evaluate performance gates
    /// 7. Safety validation
    /// 8. Assemble artifact
    pub fn generate(
        &mut self,
        commitment: &SelfModificationCommitment,
        decision: &PolicyDecisionCard,
    ) -> CodegenResult<crate::artifact::CodegenArtifact> {
        // 1. Validate commitment approval
        if !decision.is_approved() {
            return Err(CodegenError::CommitmentNotApproved(format!(
                "Decision is {:?}, expected Approved",
                decision.decision,
            )));
        }

        // 2. Validate provenance chain
        if !commitment.intent_chain.has_full_provenance() {
            return Err(CodegenError::CommitmentValidationFailed(
                "Incomplete provenance chain — observations and meanings required".into(),
            ));
        }

        // 3. Create generation record
        let mut record = GenerationRecord::new(
            commitment.id.clone(),
            commitment.tier.clone(),
        );
        let codegen_id = record.id.clone();

        // 4. Generate code per CodeChangeSpec
        let proposal = &commitment.proposal;
        let mut generated_files = Vec::new();
        for (i, change_spec) in proposal.code_changes.iter().enumerate() {
            let context = GenerationContext {
                proposal_summary: proposal.summary.clone(),
                rationale: proposal.rationale.clone(),
                tier: commitment.tier.clone(),
                change_index: i,
                total_changes: proposal.code_changes.len(),
            };
            match self.generator.generate(change_spec, &context) {
                Ok(code) => generated_files.push(code),
                Err(e) => {
                    record.mark_failed(format!("Generation failed at index {}: {}", i, e));
                    self.store_record(record);
                    return Err(e);
                }
            }
        }
        record.generated_code = generated_files.clone();
        record.mark_generated();

        // 5. Sandbox compile each file
        let mut compilation_results = Vec::new();
        for code in &generated_files {
            match self.sandbox.compile(code) {
                Ok(result) => {
                    if !result.success {
                        let reason = format!(
                            "Compilation failed for '{}': {}",
                            code.file_path,
                            result.diagnostics.join("; "),
                        );
                        record.compilation_results = compilation_results;
                        record.mark_failed(reason.clone());
                        self.store_record(record);
                        return Err(CodegenError::CompilationFailed(reason));
                    }
                    compilation_results.push(result);
                }
                Err(e) => {
                    record.mark_failed(e.to_string());
                    self.store_record(record);
                    return Err(e);
                }
            }
        }
        record.compilation_results = compilation_results.clone();
        record.mark_compiled();

        // 6. Run tests
        let test_results = self
            .sandbox
            .run_tests(&generated_files, &proposal.required_tests)?;
        let _test_summary = TestValidator::validate(
            &test_results,
            &proposal.required_tests,
            self.config.require_all_tests_pass,
        ).map_err(|e| {
            record.test_results = test_results.clone();
            record.mark_failed(e.to_string());
            self.store_record(record.clone());
            e
        })?;
        record.test_results = test_results.clone();

        // 7. Evaluate performance gates
        let perf_results = self
            .sandbox
            .evaluate_performance(&generated_files, &proposal.performance_gates)?;
        let _perf_summary = PerformanceValidator::validate(
            &perf_results,
            &proposal.performance_gates,
            self.config.require_performance_gates,
        ).map_err(|e| {
            record.performance_results = perf_results.clone();
            record.mark_failed(e.to_string());
            self.store_record(record.clone());
            e
        })?;
        record.performance_results = perf_results.clone();

        // 8. Safety validation
        if self.config.enforce_safety_checks {
            SafetyValidator::validate(
                &generated_files,
                &proposal.safety_checks,
                &self.config,
            ).map_err(|e| {
                record.mark_failed(e.to_string());
                self.store_record(record.clone());
                e
            })?;
        }

        // 9. Mark validated
        record.mark_validated();

        // 10. Assemble artifact
        let artifact = ArtifactBuilder::new(codegen_id, commitment)
            .with_generated_files(generated_files)
            .with_compilation_results(compilation_results)
            .with_test_results(test_results)
            .with_performance_results(perf_results)
            .build()?;

        self.store_record(record);
        Ok(artifact)
    }

    /// Find a generation record by ID.
    pub fn find(&self, id: &CodegenId) -> Option<&GenerationRecord> {
        self.records.iter().find(|r| r.id == *id)
    }

    /// All tracked generation records.
    pub fn all_records(&self) -> &VecDeque<GenerationRecord> {
        &self.records
    }

    /// Summary statistics.
    pub fn summary(&self) -> CodegenSummary {
        let mut summary = CodegenSummary::default();
        summary.total = self.records.len();
        for record in &self.records {
            match &record.status {
                CodegenStatus::Validated => {
                    summary.succeeded += 1;
                    summary.total_files_generated += record.generated_code.len();
                    summary.total_tests_run += record.test_results.len();
                    summary.total_tests_passed +=
                        record.test_results.iter().filter(|r| r.passed).count();
                }
                CodegenStatus::Failed(_) => {
                    summary.failed += 1;
                }
                _ => {
                    summary.pending += 1;
                }
            }
        }
        summary
    }

    /// Store record with FIFO eviction.
    fn store_record(&mut self, record: GenerationRecord) {
        if self.records.len() >= self.config.max_tracked_records {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::SimulatedGenerator;
    use crate::sandbox::SimulatedSandbox;
    use maple_worldline_intent::intent::ImprovementEstimate;
    use maple_worldline_intent::proposal::*;
    use maple_worldline_intent::types::{CodeChangeType, IntentId, MeaningId, ProposalId};
    use maple_worldline_self_mod_gate::commitment::IntentChain;
    use maple_worldline_self_mod_gate::types::{DeploymentStrategy, SelfModTier};

    fn make_commitment_with(
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
                summary: "Test proposal".into(),
                rationale: "Testing".into(),
                affected_components: vec!["module".into()],
                code_changes: changes,
                required_tests: vec![TestSpec {
                    name: "test_it".into(),
                    description: "Test".into(),
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

    fn make_simple_commitment() -> SelfModificationCommitment {
        make_commitment_with(
            vec![("src/config.rs", CodeChangeType::ModifyFunction {
                function_name: "load".into(),
            })],
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
        )
    }

    fn approved_decision() -> PolicyDecisionCard {
        PolicyDecisionCard::approved()
    }

    fn denied_decision() -> PolicyDecisionCard {
        PolicyDecisionCard::denied("risk too high")
    }

    #[test]
    fn engine_full_pipeline_success() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::all_pass()),
        );

        let commitment = make_simple_commitment();
        let artifact = engine.generate(&commitment, &approved_decision()).unwrap();

        assert!(artifact.is_deployable());
        assert!(artifact.fully_validated);
        assert_eq!(artifact.total_files, 1);
        assert_eq!(artifact.commitment_id, commitment.id);
    }

    #[test]
    fn engine_rejects_non_approved_commitment() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::all_pass()),
        );

        let commitment = make_simple_commitment();
        let result = engine.generate(&commitment, &denied_decision());
        assert!(result.is_err());
        match result {
            Err(CodegenError::CommitmentNotApproved(_)) => {}
            _ => panic!("Expected CommitmentNotApproved"),
        }
    }

    #[test]
    fn engine_rejects_missing_provenance() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::all_pass()),
        );

        // Create commitment with empty provenance
        let changes = vec![CodeChangeSpec {
            file_path: "src/config.rs".into(),
            change_type: CodeChangeType::ModifyFunction { function_name: "load".into() },
            description: "test".into(),
            affected_regions: vec![],
            provenance: vec![MeaningId::new()],
        }];

        let commitment = SelfModificationCommitment::new(
            RegenerationProposal {
                id: ProposalId::new(),
                summary: "Test".into(),
                rationale: "Testing".into(),
                affected_components: vec!["module".into()],
                code_changes: changes,
                required_tests: vec![TestSpec { name: "t".into(), description: "t".into(), test_type: TestType::Unit }],
                performance_gates: vec![],
                safety_checks: vec![],
                estimated_improvement: ImprovementEstimate { metric: "speed".into(), current_value: 10.0, projected_value: 8.0, confidence: 0.9, unit: "ms".into() },
                risk_score: 0.1,
                rollback_plan: RollbackPlan { strategy: RollbackStrategy::GitRevert, steps: vec!["revert".into()], estimated_duration_secs: 60 },
            },
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            RollbackPlan { strategy: RollbackStrategy::GitRevert, steps: vec!["git revert HEAD".into()], estimated_duration_secs: 60 },
            IntentChain { observation_ids: vec![], meaning_ids: vec![], intent_id: IntentId::new() },
        ).unwrap();

        let result = engine.generate(&commitment, &approved_decision());
        assert!(matches!(result, Err(CodegenError::CommitmentValidationFailed(_))));
    }

    #[test]
    fn engine_generation_failure() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(false)),
            Box::new(SimulatedSandbox::all_pass()),
        );

        let commitment = make_simple_commitment();
        let result = engine.generate(&commitment, &approved_decision());
        assert!(result.is_err());

        // Record should be stored as failed
        assert_eq!(engine.all_records().len(), 1);
        assert!(matches!(
            engine.all_records()[0].status,
            CodegenStatus::Failed(_)
        ));
    }

    #[test]
    fn engine_compilation_failure() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::compilation_fails()),
        );

        let commitment = make_simple_commitment();
        let result = engine.generate(&commitment, &approved_decision());
        assert!(matches!(result, Err(CodegenError::CompilationFailed(_))));
    }

    #[test]
    fn engine_test_failure() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::tests_fail()),
        );

        let commitment = make_simple_commitment();
        let result = engine.generate(&commitment, &approved_decision());
        assert!(matches!(result, Err(CodegenError::TestValidationFailed(_))));
    }

    #[test]
    fn engine_multiple_code_changes() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::all_pass()),
        );

        let commitment = make_commitment_with(
            vec![
                ("src/config.rs", CodeChangeType::ModifyFunction { function_name: "load".into() }),
                ("src/handler.rs", CodeChangeType::ModifyStruct { struct_name: "Handler".into() }),
                ("src/new_mod.rs", CodeChangeType::CreateFile),
            ],
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
        );

        let artifact = engine.generate(&commitment, &approved_decision()).unwrap();
        assert_eq!(artifact.total_files, 3);
        assert!(artifact.is_deployable());
    }

    #[test]
    fn engine_fifo_eviction() {
        let config = CodegenConfig {
            max_tracked_records: 2,
            ..CodegenConfig::default()
        };
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::all_pass()),
        )
        .with_config(config);

        // Generate 3 times — first should be evicted
        for _ in 0..3 {
            let commitment = make_simple_commitment();
            engine.generate(&commitment, &approved_decision()).unwrap();
        }

        assert_eq!(engine.all_records().len(), 2);
    }

    #[test]
    fn engine_summary_statistics() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::all_pass()),
        );

        // Two successes
        for _ in 0..2 {
            let commitment = make_simple_commitment();
            engine.generate(&commitment, &approved_decision()).unwrap();
        }

        let summary = engine.summary();
        assert_eq!(summary.total, 2);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.total_files_generated, 2);
    }

    #[test]
    fn engine_artifact_has_provenance() {
        let mut engine = CodegenEngine::new(
            Box::new(SimulatedGenerator::new(true)),
            Box::new(SimulatedSandbox::all_pass()),
        );

        let commitment = make_simple_commitment();
        let artifact = engine.generate(&commitment, &approved_decision()).unwrap();
        assert!(artifact.intent_chain.has_full_provenance());
    }
}
