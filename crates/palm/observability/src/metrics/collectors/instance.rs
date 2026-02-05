//! Instance metrics

use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry};

/// Metrics for instance lifecycle
pub struct InstanceMetrics {
    /// Total instances by status
    pub instances_by_status: IntGaugeVec,

    /// Instance lifecycle events
    pub lifecycle_events_total: IntCounterVec,

    /// Instance startup duration
    pub startup_duration_seconds: HistogramVec,

    /// Instance termination duration
    pub termination_duration_seconds: HistogramVec,

    /// Migrations
    pub migrations_total: IntCounterVec,

    /// Migration duration
    pub migration_duration_seconds: HistogramVec,

    /// Restarts
    pub restarts_total: IntCounterVec,
}

impl InstanceMetrics {
    /// Create and register instance metrics
    pub fn new(registry: &Registry) -> Self {
        let instances_by_status = IntGaugeVec::new(
            Opts::new("instances_by_status", "Instances by status"),
            &["platform", "deployment_id", "status"],
        )
        .expect("Failed to create instances_by_status metric");
        registry
            .register(Box::new(instances_by_status.clone()))
            .expect("Failed to register instances_by_status");

        let lifecycle_events_total = IntCounterVec::new(
            Opts::new(
                "instance_lifecycle_events_total",
                "Instance lifecycle events",
            ),
            &["platform", "event_type"],
        )
        .expect("Failed to create lifecycle_events_total metric");
        registry
            .register(Box::new(lifecycle_events_total.clone()))
            .expect("Failed to register lifecycle_events_total");

        let startup_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "instance_startup_duration_seconds",
                "Instance startup duration",
            )
            .buckets(vec![0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]),
            &["platform"],
        )
        .expect("Failed to create startup_duration_seconds metric");
        registry
            .register(Box::new(startup_duration_seconds.clone()))
            .expect("Failed to register startup_duration_seconds");

        let termination_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "instance_termination_duration_seconds",
                "Instance termination duration",
            )
            .buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 120.0]),
            &["platform", "graceful"],
        )
        .expect("Failed to create termination_duration_seconds metric");
        registry
            .register(Box::new(termination_duration_seconds.clone()))
            .expect("Failed to register termination_duration_seconds");

        let migrations_total = IntCounterVec::new(
            Opts::new("instance_migrations_total", "Instance migrations"),
            &["platform", "outcome"],
        )
        .expect("Failed to create migrations_total metric");
        registry
            .register(Box::new(migrations_total.clone()))
            .expect("Failed to register migrations_total");

        let migration_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "instance_migration_duration_seconds",
                "Instance migration duration",
            )
            .buckets(vec![5.0, 10.0, 30.0, 60.0, 120.0, 300.0]),
            &["platform"],
        )
        .expect("Failed to create migration_duration_seconds metric");
        registry
            .register(Box::new(migration_duration_seconds.clone()))
            .expect("Failed to register migration_duration_seconds");

        let restarts_total = IntCounterVec::new(
            Opts::new("instance_restarts_total", "Instance restarts"),
            &["platform", "reason", "graceful"],
        )
        .expect("Failed to create restarts_total metric");
        registry
            .register(Box::new(restarts_total.clone()))
            .expect("Failed to register restarts_total");

        Self {
            instances_by_status,
            lifecycle_events_total,
            startup_duration_seconds,
            termination_duration_seconds,
            migrations_total,
            migration_duration_seconds,
            restarts_total,
        }
    }

    /// Record an instance startup
    pub fn record_startup(&self, platform: &str, duration_secs: f64) {
        self.startup_duration_seconds
            .with_label_values(&[platform])
            .observe(duration_secs);
        self.lifecycle_events_total
            .with_label_values(&[platform, "started"])
            .inc();
    }

    /// Record an instance termination
    pub fn record_termination(&self, platform: &str, graceful: bool, duration_secs: f64) {
        let graceful_str = if graceful { "true" } else { "false" };
        self.termination_duration_seconds
            .with_label_values(&[platform, graceful_str])
            .observe(duration_secs);
        self.lifecycle_events_total
            .with_label_values(&[platform, "terminated"])
            .inc();
    }

    /// Record an instance migration
    pub fn record_migration(&self, platform: &str, outcome: &str, duration_secs: f64) {
        self.migrations_total
            .with_label_values(&[platform, outcome])
            .inc();
        self.migration_duration_seconds
            .with_label_values(&[platform])
            .observe(duration_secs);
    }

    /// Record an instance restart
    pub fn record_restart(&self, platform: &str, reason: &str, graceful: bool) {
        let graceful_str = if graceful { "true" } else { "false" };
        self.restarts_total
            .with_label_values(&[platform, reason, graceful_str])
            .inc();
    }

    /// Set instance count by status
    pub fn set_status_count(&self, platform: &str, deployment_id: &str, status: &str, count: i64) {
        self.instances_by_status
            .with_label_values(&[platform, deployment_id, status])
            .set(count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_metrics() {
        let registry = Registry::new();
        let metrics = InstanceMetrics::new(&registry);

        metrics.record_startup("development", 2.5);
        metrics.record_termination("development", true, 5.0);
        metrics.record_restart("development", "oom", false);

        let families = registry.gather();
        assert!(!families.is_empty());
    }
}
