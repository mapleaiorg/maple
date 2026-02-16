//! Codegen artifact — bundled output for deployment.
//!
//! A `CodegenArtifact` is the complete output of the codegen engine:
//! generated files, compilation results, test results, performance
//! results, and metadata. This is what the deployment pipeline consumes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use maple_worldline_intent::proposal::RollbackPlan;
use maple_worldline_self_mod_gate::commitment::{IntentChain, SelfModificationCommitment};
use maple_worldline_self_mod_gate::types::{DeploymentStrategy, SelfModTier};

use crate::error::{CodegenError, CodegenResult};
use crate::types::{CodegenId, CompilationResult, GeneratedCode, PerformanceResult, TestResult};

// ── Codegen Artifact ───────────────────────────────────────────────────

/// A complete codegen artifact ready for deployment.
///
/// This is the output of the codegen engine and the input to the
/// deployment pipeline (consequence engine / Prompt 17).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodegenArtifact {
    /// Unique codegen session ID.
    pub codegen_id: CodegenId,
    /// Commitment ID this artifact fulfills.
    pub commitment_id: String,
    /// Self-modification tier.
    pub tier: SelfModTier,
    /// All generated code files.
    pub generated_files: Vec<GeneratedCode>,
    /// Compilation results for each file.
    pub compilation_results: Vec<CompilationResult>,
    /// Test results.
    pub test_results: Vec<TestResult>,
    /// Performance gate results.
    pub performance_results: Vec<PerformanceResult>,
    /// Whether all validation passed.
    pub fully_validated: bool,
    /// Total files generated.
    pub total_files: usize,
    /// Total tests run.
    pub total_tests: usize,
    /// Tests passed.
    pub tests_passed: usize,
    /// Total performance gates checked.
    pub total_perf_gates: usize,
    /// Performance gates passed.
    pub perf_gates_passed: usize,
    /// When the artifact was assembled.
    pub assembled_at: DateTime<Utc>,
    /// Total generation + validation duration (ms).
    pub total_duration_ms: i64,
    /// Deployment strategy from the commitment.
    pub deployment_strategy: DeploymentStrategy,
    /// Rollback plan from the commitment.
    pub rollback_plan: RollbackPlan,
    /// Provenance: the intent chain from the commitment.
    pub intent_chain: IntentChain,
}

impl CodegenArtifact {
    /// Whether this artifact is safe to deploy.
    pub fn is_deployable(&self) -> bool {
        self.fully_validated && !self.generated_files.is_empty()
    }

    /// Affected file paths.
    pub fn affected_files(&self) -> Vec<&str> {
        self.generated_files
            .iter()
            .map(|g| g.file_path.as_str())
            .collect()
    }

    /// Summary line for logging.
    pub fn summary_line(&self) -> String {
        format!(
            "[{}] {} files, {}/{} tests, {}/{} perf gates, {}",
            self.codegen_id,
            self.total_files,
            self.tests_passed,
            self.total_tests,
            self.perf_gates_passed,
            self.total_perf_gates,
            if self.fully_validated {
                "VALIDATED"
            } else {
                "NOT VALIDATED"
            },
        )
    }
}

impl std::fmt::Display for CodegenArtifact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary_line())
    }
}

// ── Artifact Builder ───────────────────────────────────────────────────

/// Builder for assembling a CodegenArtifact from engine state.
pub struct ArtifactBuilder {
    codegen_id: CodegenId,
    commitment_id: String,
    tier: SelfModTier,
    deployment_strategy: DeploymentStrategy,
    rollback_plan: RollbackPlan,
    intent_chain: IntentChain,
    generated_files: Vec<GeneratedCode>,
    compilation_results: Vec<CompilationResult>,
    test_results: Vec<TestResult>,
    performance_results: Vec<PerformanceResult>,
    start_time: DateTime<Utc>,
}

impl ArtifactBuilder {
    /// Create a new builder from a codegen ID and commitment.
    pub fn new(codegen_id: CodegenId, commitment: &SelfModificationCommitment) -> Self {
        Self {
            codegen_id,
            commitment_id: commitment.id.clone(),
            tier: commitment.tier.clone(),
            deployment_strategy: commitment.deployment.clone(),
            rollback_plan: commitment.rollback_plan.clone(),
            intent_chain: commitment.intent_chain.clone(),
            generated_files: vec![],
            compilation_results: vec![],
            test_results: vec![],
            performance_results: vec![],
            start_time: Utc::now(),
        }
    }

    /// Set generated files.
    pub fn with_generated_files(mut self, files: Vec<GeneratedCode>) -> Self {
        self.generated_files = files;
        self
    }

    /// Set compilation results.
    pub fn with_compilation_results(mut self, results: Vec<CompilationResult>) -> Self {
        self.compilation_results = results;
        self
    }

    /// Set test results.
    pub fn with_test_results(mut self, results: Vec<TestResult>) -> Self {
        self.test_results = results;
        self
    }

    /// Set performance results.
    pub fn with_performance_results(mut self, results: Vec<PerformanceResult>) -> Self {
        self.performance_results = results;
        self
    }

    /// Build the artifact. Validates that generated files are present.
    pub fn build(self) -> CodegenResult<CodegenArtifact> {
        if self.generated_files.is_empty() {
            return Err(CodegenError::ArtifactAssemblyFailed(
                "No generated files in artifact".into(),
            ));
        }

        let total_files = self.generated_files.len();
        let total_tests = self.test_results.len();
        let tests_passed = self.test_results.iter().filter(|r| r.passed).count();
        let total_perf_gates = self.performance_results.len();
        let perf_gates_passed = self.performance_results.iter().filter(|r| r.passed).count();

        let all_compiled = self.compilation_results.iter().all(|r| r.success);
        let all_tests = total_tests == 0 || tests_passed == total_tests;
        let all_perf = total_perf_gates == 0 || perf_gates_passed == total_perf_gates;
        let fully_validated = all_compiled && all_tests && all_perf;

        let now = Utc::now();
        let total_duration_ms = (now - self.start_time).num_milliseconds();

        Ok(CodegenArtifact {
            codegen_id: self.codegen_id,
            commitment_id: self.commitment_id,
            tier: self.tier,
            generated_files: self.generated_files,
            compilation_results: self.compilation_results,
            test_results: self.test_results,
            performance_results: self.performance_results,
            fully_validated,
            total_files,
            total_tests,
            tests_passed,
            total_perf_gates,
            perf_gates_passed,
            assembled_at: now,
            total_duration_ms,
            deployment_strategy: self.deployment_strategy,
            rollback_plan: self.rollback_plan,
            intent_chain: self.intent_chain,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CodegenId;
    use maple_worldline_intent::intent::ImprovementEstimate;
    use maple_worldline_intent::proposal::*;
    use maple_worldline_intent::types::{CodeChangeType, IntentId, MeaningId, ProposalId};
    use maple_worldline_self_mod_gate::commitment::IntentChain;

    fn make_commitment() -> SelfModificationCommitment {
        SelfModificationCommitment::new(
            RegenerationProposal {
                id: ProposalId::new(),
                summary: "Test".into(),
                rationale: "Testing".into(),
                affected_components: vec!["module".into()],
                code_changes: vec![CodeChangeSpec {
                    file_path: "src/config.rs".into(),
                    change_type: CodeChangeType::ModifyFunction {
                        function_name: "load".into(),
                    },
                    description: "test".into(),
                    affected_regions: vec![],
                    provenance: vec![MeaningId::new()],
                }],
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
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
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

    fn make_generated_code() -> GeneratedCode {
        GeneratedCode {
            change_spec_index: 0,
            file_path: "src/config.rs".into(),
            content: "fn load() {}".into(),
            description: "test".into(),
            content_hash: "abc123".into(),
            generated_at: Utc::now(),
        }
    }

    #[test]
    fn artifact_deployable_when_validated() {
        let commitment = make_commitment();
        let artifact = ArtifactBuilder::new(CodegenId::new(), &commitment)
            .with_generated_files(vec![make_generated_code()])
            .with_compilation_results(vec![CompilationResult {
                file_path: "src/config.rs".into(),
                success: true,
                diagnostics: vec![],
                duration_ms: 50,
            }])
            .with_test_results(vec![TestResult {
                test_name: "test_load".into(),
                passed: true,
                output: "ok".into(),
                duration_ms: 10,
            }])
            .build()
            .unwrap();

        assert!(artifact.is_deployable());
        assert!(artifact.fully_validated);
    }

    #[test]
    fn artifact_not_deployable_when_empty() {
        let commitment = make_commitment();
        let result = ArtifactBuilder::new(CodegenId::new(), &commitment).build();
        assert!(result.is_err());
    }

    #[test]
    fn artifact_not_deployable_when_not_validated() {
        let commitment = make_commitment();
        let artifact = ArtifactBuilder::new(CodegenId::new(), &commitment)
            .with_generated_files(vec![make_generated_code()])
            .with_compilation_results(vec![CompilationResult {
                file_path: "src/config.rs".into(),
                success: false, // Compilation failed!
                diagnostics: vec!["error".into()],
                duration_ms: 50,
            }])
            .build()
            .unwrap();

        assert!(!artifact.is_deployable());
        assert!(!artifact.fully_validated);
    }

    #[test]
    fn artifact_affected_files() {
        let commitment = make_commitment();
        let artifact = ArtifactBuilder::new(CodegenId::new(), &commitment)
            .with_generated_files(vec![make_generated_code()])
            .with_compilation_results(vec![CompilationResult {
                file_path: "src/config.rs".into(),
                success: true,
                diagnostics: vec![],
                duration_ms: 50,
            }])
            .build()
            .unwrap();

        let files = artifact.affected_files();
        assert_eq!(files, vec!["src/config.rs"]);
    }

    #[test]
    fn artifact_summary_line() {
        let commitment = make_commitment();
        let artifact = ArtifactBuilder::new(CodegenId::new(), &commitment)
            .with_generated_files(vec![make_generated_code()])
            .with_compilation_results(vec![CompilationResult {
                file_path: "src/config.rs".into(),
                success: true,
                diagnostics: vec![],
                duration_ms: 50,
            }])
            .build()
            .unwrap();

        let summary = artifact.summary_line();
        assert!(summary.contains("1 files"));
        assert!(summary.contains("VALIDATED"));
    }

    #[test]
    fn artifact_provenance_preserved() {
        let commitment = make_commitment();
        let artifact = ArtifactBuilder::new(CodegenId::new(), &commitment)
            .with_generated_files(vec![make_generated_code()])
            .with_compilation_results(vec![CompilationResult {
                file_path: "src/config.rs".into(),
                success: true,
                diagnostics: vec![],
                duration_ms: 50,
            }])
            .build()
            .unwrap();

        assert!(artifact.intent_chain.has_full_provenance());
        assert!(!artifact.intent_chain.observation_ids.is_empty());
    }
}
