//! Rollback executor — trait and simulated implementation.
//!
//! The `DeploymentRollbackExecutor` trait abstracts rollback execution.
//! Real implementations would revert commits, restore previous deployments,
//! or switch traffic back. The `SimulatedRollback` returns configurable results.

use maple_worldline_intent::proposal::{RollbackPlan, RollbackStrategy};

use crate::error::DeploymentResult;

// ── Rollback Result ────────────────────────────────────────────────────

/// Result of executing a rollback.
#[derive(Clone, Debug)]
pub struct DeploymentRollbackResult {
    /// Whether the rollback succeeded.
    pub success: bool,
    /// Number of rollback steps executed.
    pub steps_executed: usize,
    /// Total number of rollback steps.
    pub total_steps: usize,
    /// The rollback strategy used.
    pub strategy: RollbackStrategy,
    /// Output/log message.
    pub output: String,
    /// Duration in milliseconds.
    pub duration_ms: i64,
}

// ── DeploymentRollbackExecutor Trait ───────────────────────────────────

/// Trait for executing deployment rollbacks.
///
/// Real implementations would execute the steps in the RollbackPlan:
/// git reverts, file restores, traffic switches, etc.
pub trait DeploymentRollbackExecutor: Send + Sync {
    /// Execute rollback using the provided plan.
    fn rollback(
        &self,
        plan: &RollbackPlan,
        deployed_files: &[String],
    ) -> DeploymentResult<DeploymentRollbackResult>;

    /// Name of this rollback executor for logging.
    fn name(&self) -> &str;
}

// ── Simulated Rollback ─────────────────────────────────────────────────

/// A simulated rollback executor for testing.
///
/// Configurable to succeed or fail.
pub struct SimulatedRollback {
    should_succeed: bool,
}

impl SimulatedRollback {
    /// Create a rollback executor that always succeeds.
    pub fn succeeding() -> Self {
        Self {
            should_succeed: true,
        }
    }

    /// Create a rollback executor that always fails.
    pub fn failing() -> Self {
        Self {
            should_succeed: false,
        }
    }
}

impl DeploymentRollbackExecutor for SimulatedRollback {
    fn rollback(
        &self,
        plan: &RollbackPlan,
        deployed_files: &[String],
    ) -> DeploymentResult<DeploymentRollbackResult> {
        let total_steps = plan.steps.len();
        let steps_executed = if self.should_succeed { total_steps } else { 0 };

        Ok(DeploymentRollbackResult {
            success: self.should_succeed,
            steps_executed,
            total_steps,
            strategy: plan.strategy.clone(),
            output: if self.should_succeed {
                format!(
                    "Rolled back {} files using {:?}",
                    deployed_files.len(),
                    plan.strategy
                )
            } else {
                "Simulated rollback failure".into()
            },
            duration_ms: if self.should_succeed { 200 } else { 50 },
        })
    }

    fn name(&self) -> &str {
        "simulated-rollback"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rollback_plan() -> RollbackPlan {
        RollbackPlan {
            strategy: RollbackStrategy::GitRevert,
            steps: vec!["git revert HEAD".into(), "verify build".into()],
            estimated_duration_secs: 60,
        }
    }

    #[test]
    fn simulated_rollback_success() {
        let rb = SimulatedRollback::succeeding();
        let plan = make_rollback_plan();
        let files = vec!["src/config.rs".into()];
        let result = rb.rollback(&plan, &files).unwrap();
        assert!(result.success);
        assert_eq!(result.steps_executed, 2);
        assert_eq!(result.total_steps, 2);
        assert!(matches!(result.strategy, RollbackStrategy::GitRevert));
    }

    #[test]
    fn simulated_rollback_failure() {
        let rb = SimulatedRollback::failing();
        let plan = make_rollback_plan();
        let files = vec!["src/config.rs".into()];
        let result = rb.rollback(&plan, &files).unwrap();
        assert!(!result.success);
        assert_eq!(result.steps_executed, 0);
    }

    #[test]
    fn rollback_name() {
        let rb = SimulatedRollback::succeeding();
        assert_eq!(rb.name(), "simulated-rollback");
    }

    #[test]
    fn rollback_result_fields() {
        let result = DeploymentRollbackResult {
            success: true,
            steps_executed: 3,
            total_steps: 3,
            strategy: RollbackStrategy::GitRevert,
            output: "ok".into(),
            duration_ms: 150,
        };
        assert_eq!(result.steps_executed, result.total_steps);
        assert!(result.duration_ms > 0);
    }
}
