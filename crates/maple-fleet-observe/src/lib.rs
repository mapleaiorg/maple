//! MAPLE Fleet Observe -- fleet observability, metrics, and alerting.
//!
//! Collects metrics, evaluates alert rules, and provides dashboard data
//! for fleet-wide monitoring.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ObserveError {
    #[error("metric not found: {0}")]
    MetricNotFound(String),
    #[error("alert rule not found: {0}")]
    AlertRuleNotFound(String),
    #[error("invalid query: {0}")]
    InvalidQuery(String),
}

pub type ObserveResult<T> = Result<T, ObserveError>;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Severity level for alerts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// A single metric data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

impl MetricPoint {
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
            labels: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    pub fn with_label(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.labels.insert(key.into(), val.into());
        self
    }
}

/// Condition for when an alert should fire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    GreaterThan(f64),
    LessThan(f64),
    EqualTo(f64),
}

impl AlertCondition {
    pub fn evaluate(&self, value: f64) -> bool {
        match self {
            Self::GreaterThan(threshold) => value > *threshold,
            Self::LessThan(threshold) => value < *threshold,
            Self::EqualTo(threshold) => (value - threshold).abs() < f64::EPSILON,
        }
    }
}

/// An alert rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub metric_name: String,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub description: String,
}

/// A fired alert instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub rule_id: String,
    pub metric_name: String,
    pub value: f64,
    pub severity: AlertSeverity,
    pub description: String,
    pub fired_at: DateTime<Utc>,
}

/// Fleet-level metrics overview.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FleetMetrics {
    pub agent_count: u64,
    pub active_instances: u64,
    pub error_rate: f64,
    pub avg_latency_ms: f64,
    pub total_requests: u64,
}

/// Dashboard data for fleet overview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub fleet_metrics: FleetMetrics,
    pub active_alerts: Vec<Alert>,
    pub recent_metrics: Vec<MetricPoint>,
    pub generated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Metrics Collector
// ---------------------------------------------------------------------------

/// Collects and stores metrics, evaluates alert rules.
pub struct MetricsCollector {
    points: Vec<MetricPoint>,
    alert_rules: Vec<AlertRule>,
    fired_alerts: Vec<Alert>,
    fleet_metrics: FleetMetrics,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            alert_rules: Vec::new(),
            fired_alerts: Vec::new(),
            fleet_metrics: FleetMetrics::default(),
        }
    }

    /// Record a metric data point and evaluate alert rules.
    pub fn record(&mut self, point: MetricPoint) {
        // Check alert rules
        for rule in &self.alert_rules {
            if rule.metric_name == point.name && rule.condition.evaluate(point.value) {
                self.fired_alerts.push(Alert {
                    rule_id: rule.id.clone(),
                    metric_name: point.name.clone(),
                    value: point.value,
                    severity: rule.severity,
                    description: rule.description.clone(),
                    fired_at: Utc::now(),
                });
            }
        }
        self.points.push(point);
    }

    /// Add an alert rule.
    pub fn add_alert_rule(&mut self, rule: AlertRule) {
        self.alert_rules.push(rule);
    }

    /// Query metrics by name and optional time range.
    pub fn query(
        &self,
        metric_name: &str,
        since: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
    ) -> Vec<&MetricPoint> {
        self.points
            .iter()
            .filter(|p| {
                p.name == metric_name
                    && since.map_or(true, |s| p.timestamp >= s)
                    && until.map_or(true, |u| p.timestamp <= u)
            })
            .collect()
    }

    /// Export all metrics.
    pub fn export(&self) -> &[MetricPoint] {
        &self.points
    }

    /// Get all fired alerts.
    pub fn alerts(&self) -> &[Alert] {
        &self.fired_alerts
    }

    /// Update fleet-level metrics.
    pub fn update_fleet_metrics(&mut self, metrics: FleetMetrics) {
        self.fleet_metrics = metrics;
    }

    /// Generate dashboard data.
    pub fn dashboard(&self) -> DashboardData {
        let recent: Vec<MetricPoint> = self.points.iter().rev().take(50).cloned().collect();
        DashboardData {
            fleet_metrics: self.fleet_metrics.clone(),
            active_alerts: self.fired_alerts.clone(),
            recent_metrics: recent,
            generated_at: Utc::now(),
        }
    }

    /// Clear all alerts.
    pub fn clear_alerts(&mut self) {
        self.fired_alerts.clear();
    }

    /// Return the total number of recorded metrics.
    pub fn metric_count(&self) -> usize {
        self.points.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_metric() {
        let mut collector = MetricsCollector::new();
        collector.record(MetricPoint::new("cpu_usage", 45.0));
        assert_eq!(collector.metric_count(), 1);
    }

    #[test]
    fn test_query_metrics() {
        let mut collector = MetricsCollector::new();
        collector.record(MetricPoint::new("cpu_usage", 45.0));
        collector.record(MetricPoint::new("memory_usage", 70.0));
        collector.record(MetricPoint::new("cpu_usage", 55.0));
        let cpu = collector.query("cpu_usage", None, None);
        assert_eq!(cpu.len(), 2);
    }

    #[test]
    fn test_alert_fires() {
        let mut collector = MetricsCollector::new();
        collector.add_alert_rule(AlertRule {
            id: "high-cpu".into(),
            metric_name: "cpu_usage".into(),
            condition: AlertCondition::GreaterThan(80.0),
            severity: AlertSeverity::Warning,
            description: "CPU usage is too high".into(),
        });
        collector.record(MetricPoint::new("cpu_usage", 90.0));
        assert_eq!(collector.alerts().len(), 1);
        assert_eq!(collector.alerts()[0].severity, AlertSeverity::Warning);
    }

    #[test]
    fn test_alert_does_not_fire_below_threshold() {
        let mut collector = MetricsCollector::new();
        collector.add_alert_rule(AlertRule {
            id: "high-cpu".into(),
            metric_name: "cpu_usage".into(),
            condition: AlertCondition::GreaterThan(80.0),
            severity: AlertSeverity::Critical,
            description: "CPU too high".into(),
        });
        collector.record(MetricPoint::new("cpu_usage", 50.0));
        assert!(collector.alerts().is_empty());
    }

    #[test]
    fn test_alert_condition_less_than() {
        let cond = AlertCondition::LessThan(10.0);
        assert!(cond.evaluate(5.0));
        assert!(!cond.evaluate(15.0));
    }

    #[test]
    fn test_metric_point_with_labels() {
        let point = MetricPoint::new("request_count", 100.0)
            .with_label("agent", "agent-1")
            .with_label("region", "us-east");
        assert_eq!(point.labels.len(), 2);
        assert_eq!(point.labels["agent"], "agent-1");
    }

    #[test]
    fn test_dashboard() {
        let mut collector = MetricsCollector::new();
        collector.update_fleet_metrics(FleetMetrics {
            agent_count: 5,
            active_instances: 12,
            error_rate: 0.02,
            avg_latency_ms: 150.0,
            total_requests: 10000,
        });
        collector.record(MetricPoint::new("cpu", 30.0));
        let dash = collector.dashboard();
        assert_eq!(dash.fleet_metrics.agent_count, 5);
        assert_eq!(dash.recent_metrics.len(), 1);
    }

    #[test]
    fn test_clear_alerts() {
        let mut collector = MetricsCollector::new();
        collector.add_alert_rule(AlertRule {
            id: "a".into(),
            metric_name: "x".into(),
            condition: AlertCondition::GreaterThan(0.0),
            severity: AlertSeverity::Info,
            description: "test".into(),
        });
        collector.record(MetricPoint::new("x", 1.0));
        assert_eq!(collector.alerts().len(), 1);
        collector.clear_alerts();
        assert!(collector.alerts().is_empty());
    }

    #[test]
    fn test_export() {
        let mut collector = MetricsCollector::new();
        collector.record(MetricPoint::new("a", 1.0));
        collector.record(MetricPoint::new("b", 2.0));
        assert_eq!(collector.export().len(), 2);
    }

    #[test]
    fn test_fleet_metrics_default() {
        let fm = FleetMetrics::default();
        assert_eq!(fm.agent_count, 0);
        assert_eq!(fm.active_instances, 0);
        assert!((fm.error_rate - 0.0).abs() < f64::EPSILON);
    }
}
