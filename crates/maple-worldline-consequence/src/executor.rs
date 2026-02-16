//! Consequence executor — applies self-modifications from regeneration proposals.
//!
//! The executor is a trait so that the engine can be tested with a simulated
//! executor (no real code changes) while production uses a real executor.

use maple_worldline_intent::proposal::RegenerationProposal;

use crate::error::{ConsequenceError, ConsequenceResult};

// ── Execution Result ────────────────────────────────────────────────────

/// Result of executing a regeneration proposal.
#[derive(Clone, Debug)]
pub struct ExecutionResult {
    /// Whether execution was successful overall.
    pub success: bool,
    /// Number of tests that passed.
    pub tests_passed: usize,
    /// Number of tests that failed.
    pub tests_failed: usize,
    /// Human-readable output/log of the execution.
    pub output: String,
    /// Execution duration in milliseconds.
    pub duration_ms: i64,
}

// ── Executor Trait ──────────────────────────────────────────────────────

/// Trait for executing regeneration proposals.
///
/// Implementations apply code changes, run tests, and verify performance
/// gates as specified by the proposal.
pub trait ConsequenceExecutor {
    /// Execute a regeneration proposal and return the result.
    fn execute(&self, proposal: &RegenerationProposal) -> ConsequenceResult<ExecutionResult>;

    /// Name of this executor for logging.
    fn name(&self) -> &str;
}

// ── Simulated Executor ─────────────────────────────────────────────────

/// A simulated executor for testing that does not apply real changes.
///
/// Configurable to succeed or fail, with deterministic test counts
/// derived from the proposal.
pub struct SimulatedExecutor {
    should_succeed: bool,
}

impl SimulatedExecutor {
    /// Create a simulated executor.
    ///
    /// If `should_succeed` is true, all executions succeed. Otherwise, they fail.
    pub fn new(should_succeed: bool) -> Self {
        Self { should_succeed }
    }
}

impl ConsequenceExecutor for SimulatedExecutor {
    fn execute(&self, proposal: &RegenerationProposal) -> ConsequenceResult<ExecutionResult> {
        let total_tests = proposal.required_tests.len();

        if self.should_succeed {
            Ok(ExecutionResult {
                success: true,
                tests_passed: total_tests,
                tests_failed: 0,
                output: format!(
                    "Simulated execution of '{}': {} changes applied, {} tests passed",
                    proposal.summary,
                    proposal.code_changes.len(),
                    total_tests,
                ),
                duration_ms: 100, // Simulated duration
            })
        } else {
            // Simulate partial test failure: first test fails, rest pass
            let tests_failed = if total_tests > 0 { 1 } else { 0 };
            let _tests_passed = total_tests.saturating_sub(tests_failed);
            Err(ConsequenceError::ExecutionFailed(format!(
                "Simulated failure of '{}': {}/{} tests failed",
                proposal.summary, tests_failed, total_tests,
            )))
        }
    }

    fn name(&self) -> &str {
        "simulated-executor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_intent::intent::ImprovementEstimate;
    use maple_worldline_intent::proposal::*;
    use maple_worldline_intent::types::MeaningId;
    use maple_worldline_intent::types::{CodeChangeType, ProposalId};

    fn make_test_proposal() -> RegenerationProposal {
        RegenerationProposal {
            id: ProposalId::new(),
            summary: "Optimize gate scheduling".into(),
            rationale: "Reduce latency by 20%".into(),
            affected_components: vec!["gate".into()],
            code_changes: vec![CodeChangeSpec {
                file_path: "src/gate.rs".into(),
                change_type: CodeChangeType::ModifyFunction {
                    function_name: "schedule".into(),
                },
                description: "Replace linear scan".into(),
                affected_regions: vec!["schedule()".into()],
                provenance: vec![MeaningId::new()],
            }],
            required_tests: vec![
                TestSpec {
                    name: "test_ordering".into(),
                    description: "Verify order".into(),
                    test_type: TestType::Unit,
                },
                TestSpec {
                    name: "test_perf".into(),
                    description: "Verify speed".into(),
                    test_type: TestType::Performance,
                },
            ],
            performance_gates: vec![PerformanceGate {
                metric: "latency_p99".into(),
                threshold: 5.0,
                comparison: Comparison::LessThan,
            }],
            safety_checks: vec![SafetyCheck {
                invariant: "no_task_loss".into(),
                description: "All tasks scheduled".into(),
            }],
            estimated_improvement: ImprovementEstimate {
                metric: "latency".into(),
                current_value: 10.0,
                projected_value: 8.0,
                confidence: 0.85,
                unit: "ms".into(),
            },
            risk_score: 0.2,
            rollback_plan: RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec!["git revert HEAD".into()],
                estimated_duration_secs: 60,
            },
        }
    }

    #[test]
    fn simulated_executor_success() {
        let executor = SimulatedExecutor::new(true);
        let proposal = make_test_proposal();
        let result = executor.execute(&proposal).unwrap();

        assert!(result.success);
        assert_eq!(result.tests_passed, 2);
        assert_eq!(result.tests_failed, 0);
        assert!(result.output.contains("Optimize gate scheduling"));
    }

    #[test]
    fn simulated_executor_failure() {
        let executor = SimulatedExecutor::new(false);
        let proposal = make_test_proposal();
        let result = executor.execute(&proposal);

        assert!(result.is_err());
        match result {
            Err(ConsequenceError::ExecutionFailed(msg)) => {
                assert!(msg.contains("Simulated failure"));
            }
            _ => panic!("Expected ExecutionFailed"),
        }
    }

    #[test]
    fn simulated_executor_name() {
        let executor = SimulatedExecutor::new(true);
        assert_eq!(executor.name(), "simulated-executor");
    }

    #[test]
    fn execution_result_fields() {
        let result = ExecutionResult {
            success: true,
            tests_passed: 10,
            tests_failed: 0,
            output: "All good".into(),
            duration_ms: 500,
        };
        assert!(result.success);
        assert_eq!(result.tests_passed, 10);
        assert_eq!(result.duration_ms, 500);
    }
}
