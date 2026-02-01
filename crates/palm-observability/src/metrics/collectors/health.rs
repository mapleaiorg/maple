//! Health metrics

use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry};

/// Metrics for health monitoring
pub struct HealthMetrics {
    /// Health check results
    pub health_checks_total: IntCounterVec,

    /// Current health status by deployment
    pub health_status: IntGaugeVec,

    /// Health check duration
    pub health_check_duration_seconds: HistogramVec,

    /// Consecutive failures
    pub consecutive_failures: IntGaugeVec,

    /// Recovery events
    pub recoveries_total: IntCounterVec,

    /// Time to recovery
    pub time_to_recovery_seconds: HistogramVec,

    /// Degradation events
    pub degradations_total: IntCounterVec,
}

impl HealthMetrics {
    /// Create and register health metrics
    pub fn new(registry: &Registry) -> Self {
        let health_checks_total = IntCounterVec::new(
            Opts::new("health_checks_total", "Total health checks performed"),
            &["platform", "deployment_id", "result"],
        )
        .expect("Failed to create health_checks_total metric");
        registry
            .register(Box::new(health_checks_total.clone()))
            .expect("Failed to register health_checks_total");

        let health_status = IntGaugeVec::new(
            Opts::new("health_status", "Current health status (0=unknown, 1=healthy, 2=degraded, 3=unhealthy)"),
            &["platform", "deployment_id", "instance_id"],
        )
        .expect("Failed to create health_status metric");
        registry
            .register(Box::new(health_status.clone()))
            .expect("Failed to register health_status");

        let health_check_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "health_check_duration_seconds",
                "Health check duration",
            )
            .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]),
            &["platform", "check_type"],
        )
        .expect("Failed to create health_check_duration_seconds metric");
        registry
            .register(Box::new(health_check_duration_seconds.clone()))
            .expect("Failed to register health_check_duration_seconds");

        let consecutive_failures = IntGaugeVec::new(
            Opts::new("consecutive_health_failures", "Consecutive health check failures"),
            &["platform", "deployment_id", "instance_id"],
        )
        .expect("Failed to create consecutive_failures metric");
        registry
            .register(Box::new(consecutive_failures.clone()))
            .expect("Failed to register consecutive_failures");

        let recoveries_total = IntCounterVec::new(
            Opts::new("health_recoveries_total", "Total recovery events"),
            &["platform", "deployment_id"],
        )
        .expect("Failed to create recoveries_total metric");
        registry
            .register(Box::new(recoveries_total.clone()))
            .expect("Failed to register recoveries_total");

        let time_to_recovery_seconds = HistogramVec::new(
            HistogramOpts::new(
                "time_to_recovery_seconds",
                "Time from unhealthy to healthy",
            )
            .buckets(vec![10.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0]),
            &["platform", "deployment_id"],
        )
        .expect("Failed to create time_to_recovery_seconds metric");
        registry
            .register(Box::new(time_to_recovery_seconds.clone()))
            .expect("Failed to register time_to_recovery_seconds");

        let degradations_total = IntCounterVec::new(
            Opts::new("health_degradations_total", "Total degradation events"),
            &["platform", "deployment_id", "reason"],
        )
        .expect("Failed to create degradations_total metric");
        registry
            .register(Box::new(degradations_total.clone()))
            .expect("Failed to register degradations_total");

        Self {
            health_checks_total,
            health_status,
            health_check_duration_seconds,
            consecutive_failures,
            recoveries_total,
            time_to_recovery_seconds,
            degradations_total,
        }
    }

    /// Record a health check result
    pub fn record_health_check(
        &self,
        platform: &str,
        deployment_id: &str,
        result: HealthCheckResult,
        duration_secs: f64,
        check_type: &str,
    ) {
        let result_str = match result {
            HealthCheckResult::Healthy => "healthy",
            HealthCheckResult::Degraded => "degraded",
            HealthCheckResult::Unhealthy => "unhealthy",
            HealthCheckResult::Timeout => "timeout",
            HealthCheckResult::Error => "error",
        };

        self.health_checks_total
            .with_label_values(&[platform, deployment_id, result_str])
            .inc();

        self.health_check_duration_seconds
            .with_label_values(&[platform, check_type])
            .observe(duration_secs);
    }

    /// Set the current health status for an instance
    pub fn set_health_status(
        &self,
        platform: &str,
        deployment_id: &str,
        instance_id: &str,
        status: HealthStatusValue,
    ) {
        let value = match status {
            HealthStatusValue::Unknown => 0,
            HealthStatusValue::Healthy => 1,
            HealthStatusValue::Degraded => 2,
            HealthStatusValue::Unhealthy => 3,
        };

        self.health_status
            .with_label_values(&[platform, deployment_id, instance_id])
            .set(value);
    }

    /// Set consecutive failure count
    pub fn set_consecutive_failures(
        &self,
        platform: &str,
        deployment_id: &str,
        instance_id: &str,
        count: i64,
    ) {
        self.consecutive_failures
            .with_label_values(&[platform, deployment_id, instance_id])
            .set(count);
    }

    /// Record a recovery event
    pub fn record_recovery(&self, platform: &str, deployment_id: &str, time_to_recover_secs: f64) {
        self.recoveries_total
            .with_label_values(&[platform, deployment_id])
            .inc();

        self.time_to_recovery_seconds
            .with_label_values(&[platform, deployment_id])
            .observe(time_to_recover_secs);
    }

    /// Record a degradation event
    pub fn record_degradation(&self, platform: &str, deployment_id: &str, reason: &str) {
        self.degradations_total
            .with_label_values(&[platform, deployment_id, reason])
            .inc();
    }
}

/// Health check result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthCheckResult {
    Healthy,
    Degraded,
    Unhealthy,
    Timeout,
    Error,
}

/// Health status value for metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatusValue {
    Unknown,
    Healthy,
    Degraded,
    Unhealthy,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_metrics() {
        let registry = Registry::new();
        let metrics = HealthMetrics::new(&registry);

        metrics.record_health_check(
            "development",
            "deploy-1",
            HealthCheckResult::Healthy,
            0.05,
            "http",
        );
        metrics.set_health_status(
            "development",
            "deploy-1",
            "instance-1",
            HealthStatusValue::Healthy,
        );
        metrics.record_recovery("development", "deploy-1", 30.0);

        let families = registry.gather();
        assert!(!families.is_empty());
    }
}
