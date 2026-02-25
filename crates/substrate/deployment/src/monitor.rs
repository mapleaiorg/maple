//! Deployment monitor — health tracking and regression detection.
//!
//! The `DeploymentMonitor` evaluates health snapshots taken during deployment
//! to produce a `MonitoringVerdict`: Healthy, Degraded, or Regression.
//! It also determines whether a rollback should be triggered and computes
//! performance deltas for the deployment record.

use maple_worldline_self_mod_gate::ledger::PerformanceDelta;

use crate::types::{DeploymentConfig, HealthSnapshot};

// ── Monitoring Verdict ─────────────────────────────────────────────────

/// Verdict from evaluating health during deployment monitoring.
#[derive(Clone, Debug, PartialEq)]
pub enum MonitoringVerdict {
    /// All metrics are healthy.
    Healthy,
    /// Some metrics show degradation but within tolerance.
    Degraded {
        /// Metrics that show degradation.
        metrics: Vec<String>,
    },
    /// A metric shows significant regression beyond threshold.
    Regression {
        /// The metric that regressed.
        metric: String,
        /// The regression percentage.
        regression_pct: f64,
    },
}

impl std::fmt::Display for MonitoringVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded { metrics } => {
                write!(f, "degraded({})", metrics.join(", "))
            }
            Self::Regression {
                metric,
                regression_pct,
            } => write!(f, "regression({}: {:.1}%)", metric, regression_pct),
        }
    }
}

// ── Deployment Monitor ─────────────────────────────────────────────────

/// Evaluates deployment health from snapshots.
///
/// Uses the configured `max_regression_pct` threshold to determine
/// whether health degradation constitutes a regression requiring rollback.
pub struct DeploymentMonitor {
    /// Maximum acceptable regression percentage.
    max_regression_pct: f64,
}

impl DeploymentMonitor {
    /// Create a new monitor with the given configuration.
    pub fn new(config: &DeploymentConfig) -> Self {
        Self {
            max_regression_pct: config.max_regression_pct,
        }
    }

    /// Create a monitor with a specific regression threshold.
    pub fn with_threshold(max_regression_pct: f64) -> Self {
        Self { max_regression_pct }
    }

    /// Evaluate a set of health snapshots and produce a verdict.
    pub fn evaluate(&self, snapshots: &[HealthSnapshot]) -> MonitoringVerdict {
        if snapshots.is_empty() {
            return MonitoringVerdict::Healthy;
        }

        // Check for regressions first (worst case)
        for snap in snapshots {
            let regression = snap.regression_pct();
            if regression > self.max_regression_pct {
                return MonitoringVerdict::Regression {
                    metric: snap.metric.clone(),
                    regression_pct: regression,
                };
            }
        }

        // Check for degradation (unhealthy but within tolerance)
        let degraded_metrics: Vec<String> = snapshots
            .iter()
            .filter(|s| !s.healthy)
            .map(|s| s.metric.clone())
            .collect();

        if !degraded_metrics.is_empty() {
            return MonitoringVerdict::Degraded {
                metrics: degraded_metrics,
            };
        }

        MonitoringVerdict::Healthy
    }

    /// Whether a rollback should be triggered for this verdict.
    pub fn should_rollback(&self, verdict: &MonitoringVerdict) -> bool {
        matches!(verdict, MonitoringVerdict::Regression { .. })
    }

    /// Compute performance deltas from health snapshots.
    ///
    /// Groups snapshots by metric and takes the latest measurement
    /// to compute the delta from baseline.
    pub fn compute_performance_deltas(
        &self,
        snapshots: &[HealthSnapshot],
    ) -> Vec<PerformanceDelta> {
        // Collect unique metrics and use the latest snapshot for each
        let mut latest: std::collections::HashMap<String, &HealthSnapshot> =
            std::collections::HashMap::new();

        for snap in snapshots {
            latest
                .entry(snap.metric.clone())
                .and_modify(|existing| {
                    if snap.measured_at > existing.measured_at {
                        *existing = snap;
                    }
                })
                .or_insert(snap);
        }

        latest
            .into_values()
            .map(|snap| PerformanceDelta {
                metric: snap.metric.clone(),
                before: snap.baseline,
                after: snap.value,
                unit: "auto".into(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_snapshot(metric: &str, value: f64, baseline: f64, healthy: bool) -> HealthSnapshot {
        HealthSnapshot {
            metric: metric.into(),
            value,
            baseline,
            healthy,
            measured_at: Utc::now(),
        }
    }

    #[test]
    fn monitor_healthy_verdict() {
        let monitor = DeploymentMonitor::with_threshold(5.0);
        let snapshots = vec![
            make_snapshot("latency", 9.5, 10.0, true),
            make_snapshot("throughput", 105.0, 100.0, true),
        ];
        let verdict = monitor.evaluate(&snapshots);
        assert_eq!(verdict, MonitoringVerdict::Healthy);
        assert!(!monitor.should_rollback(&verdict));
    }

    #[test]
    fn monitor_degraded_verdict() {
        let monitor = DeploymentMonitor::with_threshold(5.0);
        let snapshots = vec![
            make_snapshot("latency", 10.3, 10.0, false), // 3% regression, within tolerance
        ];
        let verdict = monitor.evaluate(&snapshots);
        assert!(matches!(verdict, MonitoringVerdict::Degraded { .. }));
        assert!(!monitor.should_rollback(&verdict));
    }

    #[test]
    fn monitor_regression_verdict() {
        let monitor = DeploymentMonitor::with_threshold(5.0);
        let snapshots = vec![
            make_snapshot("latency", 15.0, 10.0, false), // 50% regression!
        ];
        let verdict = monitor.evaluate(&snapshots);
        assert!(matches!(verdict, MonitoringVerdict::Regression { .. }));
        if let MonitoringVerdict::Regression { regression_pct, .. } = &verdict {
            assert!((*regression_pct - 50.0).abs() < 0.1);
        }
        assert!(monitor.should_rollback(&verdict));
    }

    #[test]
    fn monitor_empty_snapshots_healthy() {
        let monitor = DeploymentMonitor::with_threshold(5.0);
        let verdict = monitor.evaluate(&[]);
        assert_eq!(verdict, MonitoringVerdict::Healthy);
    }

    #[test]
    fn monitor_performance_deltas() {
        let monitor = DeploymentMonitor::with_threshold(5.0);
        let snapshots = vec![
            make_snapshot("latency", 9.0, 10.0, true),
            make_snapshot("throughput", 110.0, 100.0, true),
        ];
        let deltas = monitor.compute_performance_deltas(&snapshots);
        assert_eq!(deltas.len(), 2);
        for delta in &deltas {
            assert!(!delta.metric.is_empty());
        }
    }

    #[test]
    fn monitoring_verdict_display() {
        assert_eq!(MonitoringVerdict::Healthy.to_string(), "healthy");
        assert_eq!(
            MonitoringVerdict::Degraded {
                metrics: vec!["latency".into()]
            }
            .to_string(),
            "degraded(latency)"
        );
        assert!(MonitoringVerdict::Regression {
            metric: "latency".into(),
            regression_pct: 15.5,
        }
        .to_string()
        .contains("15.5%"));
    }
}
