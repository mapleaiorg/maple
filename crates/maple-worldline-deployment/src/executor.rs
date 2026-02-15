//! Deployment executor — trait and simulated implementation.
//!
//! The `DeploymentExecutor` trait abstracts the actual deployment of files,
//! health checking, and traffic switching. Real implementations would
//! write files to a target environment, run health probes, and update
//! load balancer configuration. The `SimulatedDeploymentExecutor` returns
//! configurable results for testing.

use crate::error::DeploymentResult;
use crate::types::HealthSnapshot;

// ── File Deploy Result ─────────────────────────────────────────────────

/// Result of deploying files at a specific traffic fraction.
#[derive(Clone, Debug)]
pub struct FileDeployResult {
    /// Number of files written/deployed.
    pub files_written: usize,
    /// Whether the deployment succeeded.
    pub success: bool,
    /// Output/log message.
    pub output: String,
    /// Duration in milliseconds.
    pub duration_ms: i64,
}

// ── DeploymentExecutor Trait ───────────────────────────────────────────

/// Trait for executing deployments.
///
/// Real implementations would write files, configure traffic splitting,
/// and run health probes. The simulated implementation returns configurable
/// results for deterministic testing.
pub trait DeploymentExecutor: Send + Sync {
    /// Deploy files at a specific traffic fraction.
    fn deploy_files(
        &self,
        files: &[String],
        traffic_fraction: f64,
    ) -> DeploymentResult<FileDeployResult>;

    /// Check health of deployed files at the current traffic fraction.
    fn check_health(
        &self,
        files: &[String],
        traffic_fraction: f64,
    ) -> DeploymentResult<Vec<HealthSnapshot>>;

    /// Switch all traffic to the new deployment.
    fn switch_traffic(&self) -> DeploymentResult<()>;

    /// Name of this executor for logging.
    fn name(&self) -> &str;
}

// ── Simulated Deployment Executor ──────────────────────────────────────

/// A simulated deployment executor for testing.
///
/// Configurable to succeed or fail, with optional failure at specific
/// traffic fractions (e.g., canary deployment fails at promotion).
pub struct SimulatedDeploymentExecutor {
    healthy: bool,
    deploy_succeeds: bool,
    /// Optional: fail health check at this traffic fraction.
    fail_at_fraction: Option<f64>,
}

impl SimulatedDeploymentExecutor {
    /// Create a healthy, succeeding executor.
    pub fn healthy() -> Self {
        Self {
            healthy: true,
            deploy_succeeds: true,
            fail_at_fraction: None,
        }
    }

    /// Create an unhealthy executor (health checks fail).
    pub fn unhealthy() -> Self {
        Self {
            healthy: false,
            deploy_succeeds: true,
            fail_at_fraction: None,
        }
    }

    /// Create an executor that fails deployment.
    pub fn deploy_fails() -> Self {
        Self {
            healthy: true,
            deploy_succeeds: false,
            fail_at_fraction: None,
        }
    }

    /// Create an executor that fails health checks at a specific traffic fraction.
    pub fn failing_at_fraction(fraction: f64) -> Self {
        Self {
            healthy: true,
            deploy_succeeds: true,
            fail_at_fraction: Some(fraction),
        }
    }
}

impl DeploymentExecutor for SimulatedDeploymentExecutor {
    fn deploy_files(
        &self,
        files: &[String],
        traffic_fraction: f64,
    ) -> DeploymentResult<FileDeployResult> {
        Ok(FileDeployResult {
            files_written: files.len(),
            success: self.deploy_succeeds,
            output: if self.deploy_succeeds {
                format!(
                    "Deployed {} files at {:.0}% traffic",
                    files.len(),
                    traffic_fraction * 100.0
                )
            } else {
                "Simulated deploy failure".into()
            },
            duration_ms: 100,
        })
    }

    fn check_health(
        &self,
        files: &[String],
        traffic_fraction: f64,
    ) -> DeploymentResult<Vec<HealthSnapshot>> {
        let should_be_healthy = if let Some(fail_fraction) = self.fail_at_fraction {
            // Fail at or above the specified fraction
            traffic_fraction < fail_fraction
        } else {
            self.healthy
        };

        Ok(files
            .iter()
            .map(|_| HealthSnapshot {
                metric: "latency_p99".into(),
                value: if should_be_healthy { 8.0 } else { 15.0 },
                baseline: 10.0,
                healthy: should_be_healthy,
                measured_at: chrono::Utc::now(),
            })
            .collect())
    }

    fn switch_traffic(&self) -> DeploymentResult<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "simulated-deployment-executor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_executor_deploys_successfully() {
        let executor = SimulatedDeploymentExecutor::healthy();
        let files = vec!["src/config.rs".into()];
        let result = executor.deploy_files(&files, 1.0).unwrap();
        assert!(result.success);
        assert_eq!(result.files_written, 1);
    }

    #[test]
    fn healthy_executor_health_checks_pass() {
        let executor = SimulatedDeploymentExecutor::healthy();
        let files = vec!["src/config.rs".into()];
        let snapshots = executor.check_health(&files, 1.0).unwrap();
        assert_eq!(snapshots.len(), 1);
        assert!(snapshots[0].healthy);
    }

    #[test]
    fn unhealthy_executor_health_checks_fail() {
        let executor = SimulatedDeploymentExecutor::unhealthy();
        let files = vec!["src/config.rs".into()];
        let snapshots = executor.check_health(&files, 0.5).unwrap();
        assert!(!snapshots[0].healthy);
    }

    #[test]
    fn deploy_fails_executor() {
        let executor = SimulatedDeploymentExecutor::deploy_fails();
        let files = vec!["src/config.rs".into()];
        let result = executor.deploy_files(&files, 1.0).unwrap();
        assert!(!result.success);
    }

    #[test]
    fn failing_at_fraction_executor() {
        let executor = SimulatedDeploymentExecutor::failing_at_fraction(0.5);
        let files = vec!["src/config.rs".into()];

        // Below 0.5 — should be healthy
        let snapshots = executor.check_health(&files, 0.1).unwrap();
        assert!(snapshots[0].healthy);

        // At 0.5 — should be unhealthy
        let snapshots = executor.check_health(&files, 0.5).unwrap();
        assert!(!snapshots[0].healthy);

        // Above 0.5 — should be unhealthy
        let snapshots = executor.check_health(&files, 1.0).unwrap();
        assert!(!snapshots[0].healthy);
    }

    #[test]
    fn executor_name() {
        let executor = SimulatedDeploymentExecutor::healthy();
        assert_eq!(executor.name(), "simulated-deployment-executor");
    }
}
