//! Rollback execution for failed self-modifications.
//!
//! When a consequence execution fails and the configuration specifies
//! auto-rollback, the rollback executor applies the rollback plan from
//! the original regeneration proposal.

use maple_worldline_intent::proposal::RollbackPlan;

use crate::error::{ConsequenceError, ConsequenceResult};

// ── Rollback Result ─────────────────────────────────────────────────────

/// Result of a rollback execution.
#[derive(Clone, Debug)]
pub struct RollbackResult {
    /// Whether the rollback was successful.
    pub success: bool,
    /// Number of rollback steps executed.
    pub steps_executed: usize,
    /// Total number of rollback steps.
    pub total_steps: usize,
    /// Human-readable output/log of the rollback.
    pub output: String,
}

// ── Rollback Executor Trait ─────────────────────────────────────────────

/// Trait for executing rollback plans.
///
/// Implementations apply the rollback steps defined in a `RollbackPlan`
/// to revert a failed self-modification.
pub trait RollbackExecutor {
    /// Execute a rollback plan and return the result.
    fn rollback(&self, plan: &RollbackPlan) -> ConsequenceResult<RollbackResult>;

    /// Name of this rollback executor for logging.
    fn name(&self) -> &str;
}

// ── Simulated Rollback Executor ─────────────────────────────────────────

/// A simulated rollback executor for testing.
///
/// Configurable to succeed or fail.
pub struct SimulatedRollbackExecutor {
    should_succeed: bool,
}

impl SimulatedRollbackExecutor {
    /// Create a simulated rollback executor.
    pub fn new(should_succeed: bool) -> Self {
        Self { should_succeed }
    }
}

impl RollbackExecutor for SimulatedRollbackExecutor {
    fn rollback(&self, plan: &RollbackPlan) -> ConsequenceResult<RollbackResult> {
        let total_steps = plan.steps.len();

        if self.should_succeed {
            Ok(RollbackResult {
                success: true,
                steps_executed: total_steps,
                total_steps,
                output: format!(
                    "Simulated rollback via {:?}: {} steps completed",
                    plan.strategy, total_steps,
                ),
            })
        } else {
            Err(ConsequenceError::RollbackFailed(format!(
                "Simulated rollback failure via {:?}: step 1/{} failed",
                plan.strategy, total_steps,
            )))
        }
    }

    fn name(&self) -> &str {
        "simulated-rollback"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_intent::proposal::RollbackStrategy;

    fn make_rollback_plan() -> RollbackPlan {
        RollbackPlan {
            strategy: RollbackStrategy::GitRevert,
            steps: vec!["git revert HEAD".into(), "cargo test".into()],
            estimated_duration_secs: 120,
        }
    }

    #[test]
    fn simulated_rollback_success() {
        let executor = SimulatedRollbackExecutor::new(true);
        let plan = make_rollback_plan();
        let result = executor.rollback(&plan).unwrap();

        assert!(result.success);
        assert_eq!(result.steps_executed, 2);
        assert_eq!(result.total_steps, 2);
        assert!(result.output.contains("GitRevert"));
    }

    #[test]
    fn simulated_rollback_failure() {
        let executor = SimulatedRollbackExecutor::new(false);
        let plan = make_rollback_plan();
        let result = executor.rollback(&plan);

        assert!(result.is_err());
        match result {
            Err(ConsequenceError::RollbackFailed(msg)) => {
                assert!(msg.contains("Simulated rollback failure"));
            }
            _ => panic!("Expected RollbackFailed"),
        }
    }

    #[test]
    fn simulated_rollback_name() {
        let executor = SimulatedRollbackExecutor::new(true);
        assert_eq!(executor.name(), "simulated-rollback");
    }
}
