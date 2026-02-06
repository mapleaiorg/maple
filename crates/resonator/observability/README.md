# Resonator Observability (`crates/resonator/observability`)

Comprehensive observability infrastructure for MAPLE Resonator systems.

## Overview

This crate provides metrics collection, distributed tracing, alerting, and telemetry aggregation for monitoring Resonator behavior and ensuring system health.

## Features

- **Metrics Collection**: Counters, gauges, and histograms for all pipeline stages
- **Span Tracking**: Distributed tracing with parent-child relationships
- **Alert Engine**: Rule-based alerting with severity levels
- **Telemetry Aggregation**: Unified collection and export of observability data

## Quick Start

```rust
use resonator_observability::{
    MetricsCollector, SpanTracker, AlertEngine, TelemetryAggregator,
    Severity, AlertRule,
};

// Create observability components
let metrics = MetricsCollector::new();
let spans = SpanTracker::new();
let mut alerts = AlertEngine::new();

// Configure alerts
alerts.add_rule(AlertRule::new(
    "high_failure_rate",
    "commitment_failures > 10",
    Severity::Warning,
));

// Aggregate into unified telemetry
let telemetry = TelemetryAggregator::new(metrics, spans, alerts);
```

## Metrics

### Pipeline Metrics

Track the full resonance pipeline:

```rust
// Increment pipeline stage counters
metrics.increment_counter("pipeline.presence.signals");
metrics.increment_counter("pipeline.coupling.established");
metrics.increment_counter("pipeline.meaning.formed");
metrics.increment_counter("pipeline.intent.stabilized");
metrics.increment_counter("pipeline.commitment.created");
metrics.increment_counter("pipeline.consequence.executed");

// Record durations
metrics.record_histogram("pipeline.meaning.formation_ms", 45.0);
metrics.record_histogram("pipeline.intent.stabilization_ms", 120.0);
```

### Commitment Metrics

```rust
// Track commitment lifecycle
metrics.increment_counter("commitment.created");
metrics.increment_counter("commitment.accepted");
metrics.increment_counter("commitment.completed");
metrics.increment_counter("commitment.failed");
metrics.increment_counter("commitment.disputed");

// Record commitment durations
metrics.record_histogram("commitment.duration_ms", 5230.0);
```

### Consequence Metrics

```rust
// Track consequences
metrics.increment_counter("consequence.executed");
metrics.increment_counter("consequence.success");
metrics.increment_counter("consequence.failure");

// Attribution tracking
metrics.increment_counter("consequence.attributed");
```

### Memory Metrics

```rust
// Track memory usage
metrics.set_gauge("memory.short_term.entries", 150);
metrics.set_gauge("memory.working.entries", 45);
metrics.set_gauge("memory.long_term.entries", 1200);
metrics.set_gauge("memory.episodic.entries", 89);

// Memory operations
metrics.increment_counter("memory.store");
metrics.increment_counter("memory.retrieve");
metrics.increment_counter("memory.consolidate");
```

### Conversation Metrics

```rust
// Track conversations
metrics.set_gauge("conversation.active", 25);
metrics.increment_counter("conversation.started");
metrics.increment_counter("conversation.completed");

// Turn tracking
metrics.record_histogram("conversation.turns", 12.0);
metrics.record_histogram("conversation.duration_ms", 45000.0);
```

## Tracing

### Creating Spans

```rust
use resonator_observability::{SpanTracker, SpanContext};

let tracker = SpanTracker::new();

// Start a root span
let root = tracker.start_span("commitment.create", None);

// Create child spans
let validation = tracker.start_span("commitment.validate", Some(&root));
tracker.end_span(&validation);

let storage = tracker.start_span("commitment.store", Some(&root));
tracker.end_span(&storage);

tracker.end_span(&root);

// Export traces
let traces = tracker.export_traces();
```

### Span Attributes

```rust
let mut span = tracker.start_span("consequence.execute", None);
span.set_attribute("commitment_id", "abc123");
span.set_attribute("effect_domain", "data_modification");
span.set_attribute("resonator_id", "res_456");
tracker.end_span(&span);
```

## Alerting

### Defining Rules

```rust
use resonator_observability::{AlertEngine, AlertRule, Severity};

let mut engine = AlertEngine::new();

// Warning for high latency
engine.add_rule(AlertRule::new(
    "high_latency",
    "pipeline.meaning.formation_ms > 1000",
    Severity::Warning,
));

// Error for commitment failures
engine.add_rule(AlertRule::new(
    "commitment_failures",
    "commitment.failed > 5",
    Severity::Error,
));

// Critical for invariant violations
engine.add_rule(AlertRule::new(
    "invariant_violation",
    "invariant.violations > 0",
    Severity::Critical,
));
```

### Checking Alerts

```rust
// Check all rules against current metrics
let triggered = engine.check_rules(&metrics);

for alert in triggered {
    println!("Alert: {} ({})", alert.rule_id, alert.severity);
    println!("  Message: {}", alert.message);
    println!("  Triggered at: {}", alert.triggered_at);
}
```

### Severity Levels

| Level | Description |
|-------|-------------|
| `Info` | Informational, no action required |
| `Warning` | Degraded performance, investigate |
| `Error` | Failure condition, intervention needed |
| `Critical` | System integrity at risk, immediate action |

## Telemetry Aggregation

### Unified Export

```rust
use resonator_observability::TelemetryAggregator;

let telemetry = TelemetryAggregator::new(metrics, spans, alerts);

// Get unified snapshot
let snapshot = telemetry.snapshot();
println!("Metrics: {:?}", snapshot.metrics);
println!("Active Spans: {:?}", snapshot.active_spans);
println!("Triggered Alerts: {:?}", snapshot.alerts);

// Export as JSON
let json = telemetry.export_json()?;

// Export to endpoint
telemetry.export_to("http://collector:4317").await?;
```

### Integration with OpenTelemetry

```rust
use resonator_observability::otel::OtelExporter;

// Create OTLP exporter
let exporter = OtelExporter::new("http://jaeger:4317");

// Export metrics
exporter.export_metrics(&metrics).await?;

// Export traces
exporter.export_traces(&spans).await?;
```

## Dashboard Integration

### Prometheus Metrics

```rust
use resonator_observability::prometheus::PrometheusExporter;

let exporter = PrometheusExporter::new();

// Expose on /metrics endpoint
axum::Router::new()
    .route("/metrics", get(|| async {
        exporter.export(&metrics)
    }));
```

### Grafana Annotations

```rust
use resonator_observability::grafana::GrafanaClient;

let grafana = GrafanaClient::new("http://grafana:3000", "api-key");

// Create annotation for deployment
grafana.annotate("Deployment v1.2.3", vec!["deployment"]).await?;

// Create annotation for alert
grafana.annotate_alert(&alert).await?;
```

## Best Practices

### Metric Naming

Follow these conventions:
- Use dots for hierarchy: `component.subcomponent.metric`
- Use snake_case for names
- Include units in names: `duration_ms`, `size_bytes`

### Span Naming

- Use `component.operation` format
- Be specific but not too verbose
- Example: `commitment.validate`, `memory.consolidate`

### Alert Rules

- Start with Warning, escalate to Error/Critical
- Include actionable information in messages
- Set appropriate thresholds based on baseline

## See Also

- [Resonator Architecture](../README.md)
- [Conformance Testing](../conformance/README.md)
- [Runtime Invariants](../../maple-runtime/README.md#invariants)
