//! Regeneration proposal — concrete specification for a self-modification.
//!
//! A proposal details exactly what code changes are needed, what tests to run,
//! what performance gates must be met, and how to roll back if things go wrong.

use serde::{Deserialize, Serialize};

use maple_worldline_meaning::MeaningId;

use crate::intent::ImprovementEstimate;
use crate::types::{CodeChangeType, ProposalId};

// ── Test Spec ──────────────────────────────────────────────────────────

/// Type of test required by a proposal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestType {
    /// Unit test for isolated component.
    Unit,
    /// Integration test across components.
    Integration,
    /// Performance/benchmark test.
    Performance,
    /// Safety invariant test.
    Safety,
}

impl std::fmt::Display for TestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => write!(f, "unit"),
            Self::Integration => write!(f, "integration"),
            Self::Performance => write!(f, "performance"),
            Self::Safety => write!(f, "safety"),
        }
    }
}

/// Specification for a test that must pass before a proposal is committed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestSpec {
    /// Test name/identifier.
    pub name: String,
    /// What the test validates.
    pub description: String,
    /// Type of test.
    pub test_type: TestType,
}

// ── Performance Gate ───────────────────────────────────────────────────

/// How a performance metric is compared against a threshold.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Comparison {
    /// Metric must be less than threshold.
    LessThan,
    /// Metric must be greater than threshold.
    GreaterThan,
    /// Metric must be within ±tolerance of threshold.
    Within(f64),
}

impl std::fmt::Display for Comparison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LessThan => write!(f, "<"),
            Self::GreaterThan => write!(f, ">"),
            Self::Within(tol) => write!(f, "±{:.2}", tol),
        }
    }
}

/// A performance gate that must be satisfied before committing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceGate {
    /// Metric being measured.
    pub metric: String,
    /// Threshold value.
    pub threshold: f64,
    /// How to compare.
    pub comparison: Comparison,
}

impl std::fmt::Display for PerformanceGate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {:.2}", self.metric, self.comparison, self.threshold)
    }
}

// ── Safety Check ───────────────────────────────────────────────────────

/// A safety invariant that must be verified.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SafetyCheck {
    /// The invariant being checked.
    pub invariant: String,
    /// Human-readable description.
    pub description: String,
}

// ── Code Change Spec ───────────────────────────────────────────────────

/// Specification for a single code change within a proposal.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeChangeSpec {
    /// File path affected.
    pub file_path: String,
    /// Type of code change.
    pub change_type: CodeChangeType,
    /// Description of the change.
    pub description: String,
    /// Code regions affected (e.g., function names, line ranges).
    pub affected_regions: Vec<String>,
    /// Meaning IDs that justify this change.
    pub provenance: Vec<MeaningId>,
}

// ── Rollback Plan ──────────────────────────────────────────────────────

/// Strategy for rolling back a change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RollbackStrategy {
    /// Revert via git.
    GitRevert,
    /// Restore configuration from backup.
    ConfigRestore,
    /// Roll back operator to previous version.
    OperatorRollback,
    /// Full redeployment from known-good state.
    FullRedeploy,
}

impl std::fmt::Display for RollbackStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitRevert => write!(f, "git-revert"),
            Self::ConfigRestore => write!(f, "config-restore"),
            Self::OperatorRollback => write!(f, "operator-rollback"),
            Self::FullRedeploy => write!(f, "full-redeploy"),
        }
    }
}

/// Plan for reverting a change if it causes problems.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RollbackPlan {
    /// Rollback strategy.
    pub strategy: RollbackStrategy,
    /// Ordered steps for rollback.
    pub steps: Vec<String>,
    /// Estimated time to complete rollback (seconds).
    pub estimated_duration_secs: u64,
}

// ── Regeneration Proposal ──────────────────────────────────────────────

/// A concrete, detailed proposal for a self-modification.
///
/// Contains everything needed to evaluate, execute, and if necessary
/// revert a regeneration: code changes, tests, performance gates,
/// safety checks, and a rollback plan.
#[derive(Clone, Debug)]
pub struct RegenerationProposal {
    /// Unique identifier.
    pub id: ProposalId,
    /// Brief summary of the change.
    pub summary: String,
    /// Rationale for the change.
    pub rationale: String,
    /// Components affected.
    pub affected_components: Vec<String>,
    /// Detailed code changes.
    pub code_changes: Vec<CodeChangeSpec>,
    /// Tests that must pass.
    pub required_tests: Vec<TestSpec>,
    /// Performance gates to satisfy.
    pub performance_gates: Vec<PerformanceGate>,
    /// Safety invariants to verify.
    pub safety_checks: Vec<SafetyCheck>,
    /// Expected improvement.
    pub estimated_improvement: ImprovementEstimate,
    /// Overall risk score (0.0–1.0).
    pub risk_score: f64,
    /// How to revert if needed.
    pub rollback_plan: RollbackPlan,
}

impl RegenerationProposal {
    /// Whether this proposal has a rollback plan with steps.
    pub fn has_rollback(&self) -> bool {
        !self.rollback_plan.steps.is_empty()
    }

    /// Number of code changes in this proposal.
    pub fn change_count(&self) -> usize {
        self.code_changes.len()
    }

    /// Whether safety checks are present.
    pub fn has_safety_checks(&self) -> bool {
        !self.safety_checks.is_empty()
    }
}

impl std::fmt::Display for RegenerationProposal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} ({} changes, risk={:.2})",
            self.id,
            self.summary,
            self.code_changes.len(),
            self.risk_score,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proposal() -> RegenerationProposal {
        RegenerationProposal {
            id: ProposalId::new(),
            summary: "Optimize gate scheduling".into(),
            rationale: "Reduce latency by 20%".into(),
            affected_components: vec!["gate".into(), "scheduler".into()],
            code_changes: vec![CodeChangeSpec {
                file_path: "src/gate/scheduler.rs".into(),
                change_type: CodeChangeType::ModifyFunction {
                    function_name: "schedule".into(),
                },
                description: "Replace linear scan with binary heap".into(),
                affected_regions: vec!["schedule()".into()],
                provenance: vec![MeaningId::new()],
            }],
            required_tests: vec![
                TestSpec {
                    name: "test_schedule_ordering".into(),
                    description: "Verify correct scheduling order".into(),
                    test_type: TestType::Unit,
                },
                TestSpec {
                    name: "test_schedule_perf".into(),
                    description: "Verify latency improvement".into(),
                    test_type: TestType::Performance,
                },
            ],
            performance_gates: vec![PerformanceGate {
                metric: "schedule_latency_p99".into(),
                threshold: 5.0,
                comparison: Comparison::LessThan,
            }],
            safety_checks: vec![SafetyCheck {
                invariant: "no_task_loss".into(),
                description: "All submitted tasks must be scheduled".into(),
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
                steps: vec!["git revert HEAD".into(), "redeploy".into()],
                estimated_duration_secs: 300,
            },
        }
    }

    #[test]
    fn proposal_has_rollback() {
        let p = make_proposal();
        assert!(p.has_rollback());
    }

    #[test]
    fn proposal_change_count() {
        let p = make_proposal();
        assert_eq!(p.change_count(), 1);
    }

    #[test]
    fn proposal_has_safety_checks() {
        let p = make_proposal();
        assert!(p.has_safety_checks());
    }

    #[test]
    fn proposal_display_format() {
        let p = make_proposal();
        let display = p.to_string();
        assert!(display.contains("proposal:"));
        assert!(display.contains("Optimize gate scheduling"));
        assert!(display.contains("1 changes"));
    }

    #[test]
    fn test_type_display() {
        assert_eq!(TestType::Unit.to_string(), "unit");
        assert_eq!(TestType::Performance.to_string(), "performance");
    }

    #[test]
    fn rollback_strategy_display() {
        assert_eq!(RollbackStrategy::GitRevert.to_string(), "git-revert");
        assert_eq!(RollbackStrategy::FullRedeploy.to_string(), "full-redeploy");
    }
}
