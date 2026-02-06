//! Observability and Metrics for MAPLE Resonators
//!
//! This module implements observability infrastructure for the Resonance Architecture.
//! It provides metrics collection, tracing, and monitoring capabilities for tracking
//! the health and performance of Resonators.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    OBSERVABILITY SYSTEM                         │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │   ┌───────────────┐   ┌───────────────┐   ┌───────────────┐   │
//! │   │    Metrics    │   │    Tracing    │   │    Alerts     │   │
//! │   │   Collector   │   │    System     │   │    Engine     │   │
//! │   └───────┬───────┘   └───────┬───────┘   └───────┬───────┘   │
//! │           │                   │                   │             │
//! │           └───────────────────┼───────────────────┘             │
//! │                               │                                 │
//! │                               ▼                                 │
//! │   ┌─────────────────────────────────────────────────────────┐ │
//! │   │                  Telemetry Aggregator                   │ │
//! │   │         (unified view of system health)                 │ │
//! │   └─────────────────────────────────────────────────────────┘ │
//! │                               │                                 │
//! │                               ▼                                 │
//! │   ┌─────────────────────────────────────────────────────────┐ │
//! │   │                    Metric Exporters                     │ │
//! │   │          (Prometheus, OpenTelemetry, etc.)              │ │
//! │   └─────────────────────────────────────────────────────────┘ │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Components
//!
//! - [`MetricsCollector`]: Collects and aggregates metrics
//! - [`SpanTracker`]: Tracks operation spans with timing
//! - [`AlertEngine`]: Monitors metrics and triggers alerts
//! - [`TelemetryAggregator`]: Unified telemetry view
//!
//! # Metrics Categories
//!
//! - Pipeline metrics: timing through resonance stages
//! - Commitment metrics: contract lifecycle statistics
//! - Consequence metrics: effect tracking statistics
//! - Memory metrics: tier usage and consolidation
//! - Conversation metrics: session statistics

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Observability system errors.
#[derive(Debug, Error)]
pub enum ObservabilityError {
    #[error("Metric not found: {0}")]
    MetricNotFound(String),

    #[error("Invalid metric value: {0}")]
    InvalidValue(String),

    #[error("Alert threshold exceeded: {0}")]
    AlertTriggered(String),

    #[error("Lock error")]
    LockError,
}

/// Result type for observability operations.
pub type ObservabilityResult<T> = Result<T, ObservabilityError>;

// ============================================================================
// Metric Types
// ============================================================================

/// Metric data type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    /// Counter (monotonically increasing).
    Counter,
    /// Gauge (can go up or down).
    Gauge,
    /// Histogram for distribution.
    Histogram,
    /// Summary with quantiles.
    Summary,
}

/// A metric label (key-value pair).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetricLabel {
    pub key: String,
    pub value: String,
}

impl MetricLabel {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

/// Metric descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDescriptor {
    /// Metric name.
    pub name: String,
    /// Metric type.
    pub metric_type: MetricType,
    /// Description.
    pub description: String,
    /// Unit (e.g., "ms", "bytes", "count").
    pub unit: String,
    /// Labels for this metric.
    pub labels: Vec<MetricLabel>,
}

/// A counter metric.
#[derive(Debug, Default)]
pub struct Counter {
    value: AtomicU64,
}

impl Counter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_by(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

/// A gauge metric.
#[derive(Debug, Default)]
pub struct Gauge {
    value: AtomicU64,
}

impl Gauge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, value: u64) {
        self.value.store(value, Ordering::Relaxed);
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

/// Histogram bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub upper_bound: f64,
    pub count: u64,
}

/// A histogram metric for distributions.
#[derive(Debug)]
pub struct Histogram {
    buckets: RwLock<Vec<HistogramBucket>>,
    count: AtomicU64,
    sum: RwLock<f64>,
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0])
    }
}

impl Histogram {
    pub fn new(bucket_bounds: Vec<f64>) -> Self {
        let buckets: Vec<_> = bucket_bounds
            .into_iter()
            .map(|b| HistogramBucket {
                upper_bound: b,
                count: 0,
            })
            .collect();

        Self {
            buckets: RwLock::new(buckets),
            count: AtomicU64::new(0),
            sum: RwLock::new(0.0),
        }
    }

    pub fn observe(&self, value: f64) {
        self.count.fetch_add(1, Ordering::Relaxed);

        if let Ok(mut sum) = self.sum.write() {
            *sum += value;
        }

        if let Ok(mut buckets) = self.buckets.write() {
            for bucket in buckets.iter_mut() {
                if value <= bucket.upper_bound {
                    bucket.count += 1;
                }
            }
        }
    }

    pub fn get_count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    pub fn get_sum(&self) -> f64 {
        self.sum.read().map(|s| *s).unwrap_or(0.0)
    }

    pub fn get_buckets(&self) -> Vec<HistogramBucket> {
        self.buckets.read().map(|b| b.clone()).unwrap_or_default()
    }
}

// ============================================================================
// Span Tracking
// ============================================================================

/// A unique span identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(pub String);

impl SpanId {
    pub fn generate() -> Self {
        Self(format!("span-{}", uuid::Uuid::new_v4()))
    }
}

/// Span status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanStatus {
    /// In progress.
    InProgress,
    /// Completed successfully.
    Ok,
    /// Completed with error.
    Error(String),
}

/// A span representing a traced operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Unique span ID.
    pub id: SpanId,
    /// Parent span ID (for nested spans).
    pub parent_id: Option<SpanId>,
    /// Operation name.
    pub name: String,
    /// Start time.
    pub start_time: DateTime<Utc>,
    /// End time.
    pub end_time: Option<DateTime<Utc>>,
    /// Duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// Status.
    pub status: SpanStatus,
    /// Attributes.
    pub attributes: HashMap<String, String>,
    /// Events within the span.
    pub events: Vec<SpanEvent>,
}

/// An event within a span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    /// Event name.
    pub name: String,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// Attributes.
    pub attributes: HashMap<String, String>,
}

impl Span {
    /// Create a new span.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: SpanId::generate(),
            parent_id: None,
            name: name.into(),
            start_time: Utc::now(),
            end_time: None,
            duration_ms: None,
            status: SpanStatus::InProgress,
            attributes: HashMap::new(),
            events: Vec::new(),
        }
    }

    /// Create a child span.
    pub fn child(&self, name: impl Into<String>) -> Self {
        let mut child = Self::new(name);
        child.parent_id = Some(self.id.clone());
        child
    }

    /// Add an attribute.
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Add an event.
    pub fn add_event(&mut self, name: impl Into<String>) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp: Utc::now(),
            attributes: HashMap::new(),
        });
    }

    /// Mark the span as complete.
    pub fn complete(&mut self) {
        let now = Utc::now();
        self.end_time = Some(now);
        self.duration_ms = Some((now - self.start_time).num_milliseconds());
        self.status = SpanStatus::Ok;
    }

    /// Mark the span as errored.
    pub fn error(&mut self, msg: impl Into<String>) {
        let now = Utc::now();
        self.end_time = Some(now);
        self.duration_ms = Some((now - self.start_time).num_milliseconds());
        self.status = SpanStatus::Error(msg.into());
    }
}

/// Span tracker for managing spans.
pub struct SpanTracker {
    spans: RwLock<HashMap<SpanId, Span>>,
    completed: RwLock<Vec<Span>>,
    max_completed: usize,
}

impl Default for SpanTracker {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl SpanTracker {
    pub fn new(max_completed: usize) -> Self {
        Self {
            spans: RwLock::new(HashMap::new()),
            completed: RwLock::new(Vec::new()),
            max_completed,
        }
    }

    /// Start a new span.
    pub fn start_span(&self, name: impl Into<String>) -> Span {
        let span = Span::new(name);
        if let Ok(mut spans) = self.spans.write() {
            spans.insert(span.id.clone(), span.clone());
        }
        span
    }

    /// Complete a span.
    pub fn complete_span(&self, span_id: &SpanId) -> ObservabilityResult<Span> {
        let span = {
            let mut spans = self.spans.write().map_err(|_| ObservabilityError::LockError)?;
            let mut span = spans
                .remove(span_id)
                .ok_or_else(|| ObservabilityError::MetricNotFound(span_id.0.clone()))?;
            span.complete();
            span
        };

        // Store in completed
        if let Ok(mut completed) = self.completed.write() {
            completed.push(span.clone());
            if completed.len() > self.max_completed {
                completed.remove(0);
            }
        }

        Ok(span)
    }

    /// Get recent completed spans.
    pub fn recent_spans(&self, limit: usize) -> Vec<Span> {
        self.completed
            .read()
            .map(|c| c.iter().rev().take(limit).cloned().collect())
            .unwrap_or_default()
    }
}

// ============================================================================
// Metrics Collector
// ============================================================================

/// Resonance pipeline stage for metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineStage {
    Presence,
    Coupling,
    Meaning,
    Intent,
    Commitment,
    Consequence,
}

impl std::fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Metrics collector for the Resonator system.
pub struct MetricsCollector {
    // Pipeline metrics
    pipeline_requests: RwLock<HashMap<PipelineStage, Counter>>,
    pipeline_latency: RwLock<HashMap<PipelineStage, Histogram>>,
    pipeline_errors: RwLock<HashMap<PipelineStage, Counter>>,

    // Commitment metrics
    commitments_created: Counter,
    commitments_completed: Counter,
    commitments_failed: Counter,
    active_commitments: Gauge,

    // Consequence metrics
    consequences_recorded: Counter,
    consequences_failed: Counter,

    // Memory metrics
    memory_items: RwLock<HashMap<String, Gauge>>,
    memory_operations: Counter,

    // Conversation metrics
    active_sessions: Gauge,
    total_turns: Counter,

    // General metrics
    start_time: DateTime<Utc>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            pipeline_requests: RwLock::new(HashMap::new()),
            pipeline_latency: RwLock::new(HashMap::new()),
            pipeline_errors: RwLock::new(HashMap::new()),
            commitments_created: Counter::new(),
            commitments_completed: Counter::new(),
            commitments_failed: Counter::new(),
            active_commitments: Gauge::new(),
            consequences_recorded: Counter::new(),
            consequences_failed: Counter::new(),
            memory_items: RwLock::new(HashMap::new()),
            memory_operations: Counter::new(),
            active_sessions: Gauge::new(),
            total_turns: Counter::new(),
            start_time: Utc::now(),
        }
    }

    // Pipeline metrics
    pub fn record_pipeline_request(&self, stage: PipelineStage) {
        if let Ok(mut requests) = self.pipeline_requests.write() {
            requests
                .entry(stage)
                .or_insert_with(Counter::new)
                .inc();
        }
    }

    pub fn record_pipeline_latency(&self, stage: PipelineStage, duration_ms: f64) {
        if let Ok(mut latency) = self.pipeline_latency.write() {
            latency
                .entry(stage)
                .or_insert_with(Histogram::default)
                .observe(duration_ms);
        }
    }

    pub fn record_pipeline_error(&self, stage: PipelineStage) {
        if let Ok(mut errors) = self.pipeline_errors.write() {
            errors
                .entry(stage)
                .or_insert_with(Counter::new)
                .inc();
        }
    }

    // Commitment metrics
    pub fn record_commitment_created(&self) {
        self.commitments_created.inc();
        self.active_commitments.inc();
    }

    pub fn record_commitment_completed(&self) {
        self.commitments_completed.inc();
        self.active_commitments.dec();
    }

    pub fn record_commitment_failed(&self) {
        self.commitments_failed.inc();
        self.active_commitments.dec();
    }

    pub fn get_active_commitments(&self) -> u64 {
        self.active_commitments.get()
    }

    // Consequence metrics
    pub fn record_consequence(&self) {
        self.consequences_recorded.inc();
    }

    pub fn record_consequence_failure(&self) {
        self.consequences_failed.inc();
    }

    // Memory metrics
    pub fn set_memory_tier_size(&self, tier: &str, size: u64) {
        if let Ok(mut items) = self.memory_items.write() {
            items
                .entry(tier.to_string())
                .or_insert_with(Gauge::new)
                .set(size);
        }
    }

    pub fn record_memory_operation(&self) {
        self.memory_operations.inc();
    }

    // Conversation metrics
    pub fn record_session_start(&self) {
        self.active_sessions.inc();
    }

    pub fn record_session_end(&self) {
        self.active_sessions.dec();
    }

    pub fn record_turn(&self) {
        self.total_turns.inc();
    }

    pub fn get_active_sessions(&self) -> u64 {
        self.active_sessions.get()
    }

    /// Get uptime in seconds.
    pub fn uptime_seconds(&self) -> i64 {
        (Utc::now() - self.start_time).num_seconds()
    }

    /// Get a snapshot of all metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let pipeline_stats: HashMap<_, _> = [
            PipelineStage::Presence,
            PipelineStage::Coupling,
            PipelineStage::Meaning,
            PipelineStage::Intent,
            PipelineStage::Commitment,
            PipelineStage::Consequence,
        ]
        .into_iter()
        .map(|stage| {
            let requests = self
                .pipeline_requests
                .read()
                .ok()
                .and_then(|r| r.get(&stage).map(|c| c.get()))
                .unwrap_or(0);

            let errors = self
                .pipeline_errors
                .read()
                .ok()
                .and_then(|e| e.get(&stage).map(|c| c.get()))
                .unwrap_or(0);

            let avg_latency = self
                .pipeline_latency
                .read()
                .ok()
                .and_then(|l| {
                    l.get(&stage).map(|h| {
                        let count = h.get_count();
                        if count > 0 {
                            h.get_sum() / count as f64
                        } else {
                            0.0
                        }
                    })
                })
                .unwrap_or(0.0);

            (
                stage,
                PipelineStageStats {
                    requests,
                    errors,
                    avg_latency_ms: avg_latency,
                },
            )
        })
        .collect();

        MetricsSnapshot {
            timestamp: Utc::now(),
            uptime_seconds: self.uptime_seconds(),
            pipeline: pipeline_stats,
            commitments: CommitmentStats {
                created: self.commitments_created.get(),
                completed: self.commitments_completed.get(),
                failed: self.commitments_failed.get(),
                active: self.active_commitments.get(),
            },
            consequences: ConsequenceStats {
                recorded: self.consequences_recorded.get(),
                failed: self.consequences_failed.get(),
            },
            memory: MemoryStats {
                operations: self.memory_operations.get(),
            },
            conversations: ConversationStats {
                active_sessions: self.active_sessions.get(),
                total_turns: self.total_turns.get(),
            },
        }
    }
}

/// A snapshot of all metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: i64,
    pub pipeline: HashMap<PipelineStage, PipelineStageStats>,
    pub commitments: CommitmentStats,
    pub consequences: ConsequenceStats,
    pub memory: MemoryStats,
    pub conversations: ConversationStats,
}

/// Stats for a pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageStats {
    pub requests: u64,
    pub errors: u64,
    pub avg_latency_ms: f64,
}

/// Commitment statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentStats {
    pub created: u64,
    pub completed: u64,
    pub failed: u64,
    pub active: u64,
}

/// Consequence statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsequenceStats {
    pub recorded: u64,
    pub failed: u64,
}

/// Memory statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub operations: u64,
}

/// Conversation statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationStats {
    pub active_sessions: u64,
    pub total_turns: u64,
}

// ============================================================================
// Alert Engine
// ============================================================================

/// Alert severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// An alert rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Severity.
    pub severity: AlertSeverity,
    /// Metric name to monitor.
    pub metric: String,
    /// Threshold value.
    pub threshold: f64,
    /// Comparison operator.
    pub operator: AlertOperator,
    /// Enabled flag.
    pub enabled: bool,
}

/// Alert comparison operator.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AlertOperator {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
}

/// A triggered alert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Alert ID.
    pub id: String,
    /// Rule that triggered.
    pub rule_name: String,
    /// Severity.
    pub severity: AlertSeverity,
    /// Message.
    pub message: String,
    /// Current value.
    pub current_value: f64,
    /// Threshold.
    pub threshold: f64,
    /// When triggered.
    pub triggered_at: DateTime<Utc>,
    /// Acknowledged flag.
    pub acknowledged: bool,
}

/// Alert engine for monitoring metrics.
pub struct AlertEngine {
    rules: RwLock<Vec<AlertRule>>,
    alerts: RwLock<Vec<Alert>>,
    max_alerts: usize,
}

impl Default for AlertEngine {
    fn default() -> Self {
        Self::new(100)
    }
}

impl AlertEngine {
    pub fn new(max_alerts: usize) -> Self {
        Self {
            rules: RwLock::new(Vec::new()),
            alerts: RwLock::new(Vec::new()),
            max_alerts,
        }
    }

    /// Add an alert rule.
    pub fn add_rule(&self, rule: AlertRule) -> ObservabilityResult<()> {
        let mut rules = self.rules.write().map_err(|_| ObservabilityError::LockError)?;
        rules.push(rule);
        Ok(())
    }

    /// Check a metric value against rules.
    pub fn check(&self, metric: &str, value: f64) -> Vec<Alert> {
        let rules = match self.rules.read() {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        let mut triggered = Vec::new();

        for rule in rules.iter() {
            if !rule.enabled || rule.metric != metric {
                continue;
            }

            let should_alert = match rule.operator {
                AlertOperator::GreaterThan => value > rule.threshold,
                AlertOperator::LessThan => value < rule.threshold,
                AlertOperator::Equal => (value - rule.threshold).abs() < f64::EPSILON,
                AlertOperator::NotEqual => (value - rule.threshold).abs() >= f64::EPSILON,
            };

            if should_alert {
                let alert = Alert {
                    id: format!("alert-{}", uuid::Uuid::new_v4()),
                    rule_name: rule.name.clone(),
                    severity: rule.severity,
                    message: format!(
                        "{}: {} is {} (threshold: {})",
                        rule.description, metric, value, rule.threshold
                    ),
                    current_value: value,
                    threshold: rule.threshold,
                    triggered_at: Utc::now(),
                    acknowledged: false,
                };

                triggered.push(alert.clone());

                // Store alert
                if let Ok(mut alerts) = self.alerts.write() {
                    alerts.push(alert);
                    if alerts.len() > self.max_alerts {
                        alerts.remove(0);
                    }
                }
            }
        }

        triggered
    }

    /// Get active (unacknowledged) alerts.
    pub fn active_alerts(&self) -> Vec<Alert> {
        self.alerts
            .read()
            .map(|a| a.iter().filter(|a| !a.acknowledged).cloned().collect())
            .unwrap_or_default()
    }

    /// Acknowledge an alert.
    pub fn acknowledge(&self, alert_id: &str) -> ObservabilityResult<()> {
        let mut alerts = self.alerts.write().map_err(|_| ObservabilityError::LockError)?;
        for alert in alerts.iter_mut() {
            if alert.id == alert_id {
                alert.acknowledged = true;
                return Ok(());
            }
        }
        Err(ObservabilityError::MetricNotFound(alert_id.to_string()))
    }
}

// ============================================================================
// Telemetry Aggregator
// ============================================================================

/// Health status of the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Unified telemetry aggregator.
pub struct TelemetryAggregator {
    pub metrics: MetricsCollector,
    pub spans: SpanTracker,
    pub alerts: AlertEngine,
}

impl Default for TelemetryAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryAggregator {
    pub fn new() -> Self {
        Self {
            metrics: MetricsCollector::new(),
            spans: SpanTracker::new(1000),
            alerts: AlertEngine::new(100),
        }
    }

    /// Get overall health status.
    pub fn health_status(&self) -> HealthStatus {
        let active_alerts = self.alerts.active_alerts();

        let critical_count = active_alerts
            .iter()
            .filter(|a| a.severity == AlertSeverity::Critical)
            .count();

        let error_count = active_alerts
            .iter()
            .filter(|a| a.severity == AlertSeverity::Error)
            .count();

        if critical_count > 0 {
            HealthStatus::Unhealthy
        } else if error_count > 0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Get a full telemetry report.
    pub fn report(&self) -> TelemetryReport {
        TelemetryReport {
            health: self.health_status(),
            metrics: self.metrics.snapshot(),
            recent_spans: self.spans.recent_spans(10),
            active_alerts: self.alerts.active_alerts(),
        }
    }
}

/// A full telemetry report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryReport {
    pub health: HealthStatus,
    pub metrics: MetricsSnapshot,
    pub recent_spans: Vec<Span>,
    pub active_alerts: Vec<Alert>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new();
        assert_eq!(counter.get(), 0);
        counter.inc();
        assert_eq!(counter.get(), 1);
        counter.inc_by(5);
        assert_eq!(counter.get(), 6);
    }

    #[test]
    fn test_gauge() {
        let gauge = Gauge::new();
        assert_eq!(gauge.get(), 0);
        gauge.set(10);
        assert_eq!(gauge.get(), 10);
        gauge.inc();
        assert_eq!(gauge.get(), 11);
        gauge.dec();
        assert_eq!(gauge.get(), 10);
    }

    #[test]
    fn test_histogram() {
        let histogram = Histogram::new(vec![10.0, 50.0, 100.0]);
        histogram.observe(5.0);
        histogram.observe(25.0);
        histogram.observe(75.0);

        assert_eq!(histogram.get_count(), 3);
        assert!((histogram.get_sum() - 105.0).abs() < f64::EPSILON);

        let buckets = histogram.get_buckets();
        assert_eq!(buckets.len(), 3);
        assert_eq!(buckets[0].count, 1); // <= 10
        assert_eq!(buckets[1].count, 2); // <= 50
        assert_eq!(buckets[2].count, 3); // <= 100
    }

    #[test]
    fn test_span() {
        let mut span = Span::new("test_operation");
        assert!(matches!(span.status, SpanStatus::InProgress));

        span.add_event("started");
        span.add_event("processing");
        span.complete();

        assert!(matches!(span.status, SpanStatus::Ok));
        assert!(span.duration_ms.is_some());
        assert_eq!(span.events.len(), 2);
    }

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        collector.record_pipeline_request(PipelineStage::Meaning);
        collector.record_pipeline_latency(PipelineStage::Meaning, 50.0);
        collector.record_commitment_created();

        let snapshot = collector.snapshot();
        assert!(snapshot.uptime_seconds >= 0);
        assert_eq!(snapshot.commitments.created, 1);
        assert_eq!(snapshot.commitments.active, 1);
    }

    #[test]
    fn test_alert_engine() {
        let engine = AlertEngine::new(10);

        engine
            .add_rule(AlertRule {
                name: "high_latency".to_string(),
                description: "Pipeline latency too high".to_string(),
                severity: AlertSeverity::Warning,
                metric: "pipeline_latency".to_string(),
                threshold: 100.0,
                operator: AlertOperator::GreaterThan,
                enabled: true,
            })
            .unwrap();

        // Should not trigger
        let alerts = engine.check("pipeline_latency", 50.0);
        assert!(alerts.is_empty());

        // Should trigger
        let alerts = engine.check("pipeline_latency", 150.0);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Warning);
    }

    #[test]
    fn test_telemetry_aggregator() {
        let aggregator = TelemetryAggregator::new();

        // Record some metrics
        aggregator
            .metrics
            .record_pipeline_request(PipelineStage::Intent);
        aggregator.metrics.record_session_start();

        // Start and complete a span
        let span = aggregator.spans.start_span("test_op");
        aggregator.spans.complete_span(&span.id).unwrap();

        // Get report
        let report = aggregator.report();
        assert_eq!(report.health, HealthStatus::Healthy);
        assert_eq!(report.recent_spans.len(), 1);
    }
}
