//! Deployment metrics

use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry};

/// Metrics for deployment operations
pub struct DeploymentMetrics {
    /// Total number of deployments by platform
    pub deployments_total: IntGaugeVec,

    /// Deployments by status
    pub deployments_by_status: IntGaugeVec,

    /// Deployment operations counter
    pub operations_total: IntCounterVec,

    /// Deployment operation duration
    pub operation_duration_seconds: HistogramVec,

    /// Rollout progress (0-100%)
    pub rollout_progress: IntGaugeVec,

    /// Rollback events
    pub rollbacks_total: IntCounterVec,
}

impl DeploymentMetrics {
    /// Create and register deployment metrics
    pub fn new(registry: &Registry) -> Self {
        let deployments_total = IntGaugeVec::new(
            Opts::new("deployments_total", "Total number of deployments"),
            &["platform"],
        )
        .expect("Failed to create deployments_total metric");
        registry
            .register(Box::new(deployments_total.clone()))
            .expect("Failed to register deployments_total");

        let deployments_by_status = IntGaugeVec::new(
            Opts::new("deployments_by_status", "Deployments by status"),
            &["platform", "status"],
        )
        .expect("Failed to create deployments_by_status metric");
        registry
            .register(Box::new(deployments_by_status.clone()))
            .expect("Failed to register deployments_by_status");

        let operations_total = IntCounterVec::new(
            Opts::new("deployment_operations_total", "Deployment operations"),
            &["platform", "operation", "outcome"],
        )
        .expect("Failed to create operations_total metric");
        registry
            .register(Box::new(operations_total.clone()))
            .expect("Failed to register operations_total");

        let operation_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "deployment_operation_duration_seconds",
                "Deployment operation duration",
            )
            .buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0, 600.0]),
            &["platform", "operation"],
        )
        .expect("Failed to create operation_duration_seconds metric");
        registry
            .register(Box::new(operation_duration_seconds.clone()))
            .expect("Failed to register operation_duration_seconds");

        let rollout_progress = IntGaugeVec::new(
            Opts::new("deployment_rollout_progress", "Rollout progress percentage"),
            &["deployment_id", "strategy"],
        )
        .expect("Failed to create rollout_progress metric");
        registry
            .register(Box::new(rollout_progress.clone()))
            .expect("Failed to register rollout_progress");

        let rollbacks_total = IntCounterVec::new(
            Opts::new("deployment_rollbacks_total", "Deployment rollbacks"),
            &["platform", "reason"],
        )
        .expect("Failed to create rollbacks_total metric");
        registry
            .register(Box::new(rollbacks_total.clone()))
            .expect("Failed to register rollbacks_total");

        Self {
            deployments_total,
            deployments_by_status,
            operations_total,
            operation_duration_seconds,
            rollout_progress,
            rollbacks_total,
        }
    }

    /// Record a deployment operation
    pub fn record_operation(
        &self,
        platform: &str,
        operation: &str,
        outcome: &str,
        duration_secs: f64,
    ) {
        self.operations_total
            .with_label_values(&[platform, operation, outcome])
            .inc();
        self.operation_duration_seconds
            .with_label_values(&[platform, operation])
            .observe(duration_secs);
    }

    /// Set total deployment count for a platform
    pub fn set_deployment_count(&self, platform: &str, count: i64) {
        self.deployments_total
            .with_label_values(&[platform])
            .set(count);
    }

    /// Set deployment count by status
    pub fn set_status_count(&self, platform: &str, status: &str, count: i64) {
        self.deployments_by_status
            .with_label_values(&[platform, status])
            .set(count);
    }

    /// Set rollout progress for a deployment
    pub fn set_rollout_progress(&self, deployment_id: &str, strategy: &str, progress: i64) {
        self.rollout_progress
            .with_label_values(&[deployment_id, strategy])
            .set(progress);
    }

    /// Record a rollback event
    pub fn record_rollback(&self, platform: &str, reason: &str) {
        self.rollbacks_total
            .with_label_values(&[platform, reason])
            .inc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_metrics() {
        let registry = Registry::new();
        let metrics = DeploymentMetrics::new(&registry);

        metrics.set_deployment_count("development", 5);
        metrics.record_operation("development", "create", "success", 2.5);
        metrics.set_rollout_progress("deploy-123", "rolling", 75);

        // Verify metrics exist
        let families = registry.gather();
        assert!(!families.is_empty());
    }
}
