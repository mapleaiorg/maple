//! # Observability Demo Example
//!
//! This example demonstrates:
//! - Metrics collection (counters, gauges, histograms)
//! - Distributed tracing with spans
//! - Alert rules and triggering
//! - Telemetry aggregation
//!
//! Run with: `cargo run --example 09_observability_demo`

use resonator_observability::{
    MetricsCollector, SpanTracker, AlertEngine, TelemetryAggregator,
    AlertRule, Severity, MetricType,
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🍁 MAPLE - Observability Demo Example\n");

    // Initialize observability components
    println!("📊 Initializing Observability Components");
    println!("═══════════════════════════════════════════════════════\n");

    let metrics = MetricsCollector::new();
    let spans = SpanTracker::new();
    let mut alerts = AlertEngine::new();

    println!("   ✅ MetricsCollector initialized");
    println!("   ✅ SpanTracker initialized");
    println!("   ✅ AlertEngine initialized\n");

    // Configure alert rules
    println!("🚨 Configuring Alert Rules");
    println!("═══════════════════════════════════════════════════════\n");

    alerts.add_rule(AlertRule::new(
        "high_error_rate",
        "commitment.failed > 5",
        Severity::Warning,
    ));
    println!("   Added: high_error_rate (Warning) - commitment.failed > 5");

    alerts.add_rule(AlertRule::new(
        "critical_failures",
        "commitment.failed > 10",
        Severity::Critical,
    ));
    println!("   Added: critical_failures (Critical) - commitment.failed > 10");

    alerts.add_rule(AlertRule::new(
        "slow_meaning_formation",
        "pipeline.meaning.formation_ms > 1000",
        Severity::Warning,
    ));
    println!("   Added: slow_meaning_formation (Warning) - formation > 1000ms");

    alerts.add_rule(AlertRule::new(
        "invariant_violation",
        "invariant.violations > 0",
        Severity::Critical,
    ));
    println!("   Added: invariant_violation (Critical) - any violation\n");

    // Simulate pipeline activity with metrics
    println!("🔄 Simulating Pipeline Activity");
    println!("═══════════════════════════════════════════════════════\n");

    // Start a root span for the entire operation
    let root_span = spans.start_span("pipeline.full_cycle", None);
    println!("   Started span: pipeline.full_cycle");

    // Presence stage
    let presence_span = spans.start_span("pipeline.presence", Some(&root_span));
    metrics.increment_counter("pipeline.presence.signals");
    sleep(Duration::from_millis(10)).await;
    spans.end_span(&presence_span);
    println!("   ✓ Presence signaled");

    // Coupling stage
    let coupling_span = spans.start_span("pipeline.coupling", Some(&root_span));
    metrics.increment_counter("pipeline.coupling.established");
    metrics.set_gauge("coupling.active_count", 5);
    sleep(Duration::from_millis(20)).await;
    spans.end_span(&coupling_span);
    println!("   ✓ Coupling established (5 active)");

    // Meaning formation stage
    let meaning_span = spans.start_span("pipeline.meaning", Some(&root_span));
    metrics.increment_counter("pipeline.meaning.formed");
    metrics.record_histogram("pipeline.meaning.formation_ms", 45.0);
    sleep(Duration::from_millis(45)).await;
    spans.end_span(&meaning_span);
    println!("   ✓ Meaning formed (45ms)");

    // Intent stabilization stage
    let intent_span = spans.start_span("pipeline.intent", Some(&root_span));
    metrics.increment_counter("pipeline.intent.stabilized");
    metrics.record_histogram("pipeline.intent.stabilization_ms", 120.0);
    sleep(Duration::from_millis(120)).await;
    spans.end_span(&intent_span);
    println!("   ✓ Intent stabilized (120ms)");

    // Commitment stage
    let commitment_span = spans.start_span("pipeline.commitment", Some(&root_span));

    // Simulate multiple commitments
    for i in 0..15 {
        metrics.increment_counter("commitment.created");
        if i % 3 == 0 {
            metrics.increment_counter("commitment.completed");
        } else if i % 5 == 0 {
            metrics.increment_counter("commitment.failed");
        }
    }

    sleep(Duration::from_millis(50)).await;
    spans.end_span(&commitment_span);
    println!("   ✓ Commitments processed (15 created, some completed/failed)");

    // Consequence stage
    let consequence_span = spans.start_span("pipeline.consequence", Some(&root_span));
    metrics.increment_counter("consequence.executed");
    metrics.increment_counter("consequence.success");
    sleep(Duration::from_millis(30)).await;
    spans.end_span(&consequence_span);
    println!("   ✓ Consequence executed");

    // End root span
    spans.end_span(&root_span);
    println!("\n   Total pipeline time: ~275ms\n");

    // Check alerts
    println!("🚨 Checking Alert Rules");
    println!("═══════════════════════════════════════════════════════\n");

    let triggered = alerts.check_rules(&metrics);
    if triggered.is_empty() {
        println!("   No alerts triggered");
    } else {
        for alert in &triggered {
            let severity_icon = match alert.severity {
                Severity::Info => "ℹ️ ",
                Severity::Warning => "⚠️ ",
                Severity::Error => "❌",
                Severity::Critical => "🔴",
            };
            println!("   {} [{}] {}", severity_icon, alert.severity, alert.rule_id);
            println!("      Condition: {}", alert.condition);
        }
    }
    println!();

    // Display metrics summary
    println!("📈 Metrics Summary");
    println!("═══════════════════════════════════════════════════════\n");

    println!("   Pipeline Counters:");
    for (name, value) in metrics.get_counters() {
        if name.starts_with("pipeline.") {
            println!("     {}: {}", name, value);
        }
    }

    println!("\n   Commitment Counters:");
    for (name, value) in metrics.get_counters() {
        if name.starts_with("commitment.") || name.starts_with("consequence.") {
            println!("     {}: {}", name, value);
        }
    }

    println!("\n   Gauges:");
    for (name, value) in metrics.get_gauges() {
        println!("     {}: {}", name, value);
    }

    println!("\n   Histograms:");
    for (name, stats) in metrics.get_histogram_stats() {
        println!("     {}:", name);
        println!("       count: {}, min: {:.2}, max: {:.2}, avg: {:.2}",
            stats.count, stats.min, stats.max, stats.avg);
    }

    // Display trace summary
    println!("\n📍 Trace Summary");
    println!("═══════════════════════════════════════════════════════\n");

    let traces = spans.get_completed_spans();
    println!("   Completed spans: {}", traces.len());
    for trace in traces.iter().take(5) {
        let duration_ms = trace.duration.as_millis();
        println!("     {} ({} ms)", trace.name, duration_ms);
    }

    // Create telemetry aggregator
    println!("\n📦 Telemetry Aggregation");
    println!("═══════════════════════════════════════════════════════\n");

    let telemetry = TelemetryAggregator::new(
        metrics.clone(),
        spans.clone(),
        alerts.clone(),
    );

    let snapshot = telemetry.snapshot();
    println!("   Snapshot created at: {}", snapshot.timestamp);
    println!("   Total metrics: {}", snapshot.metric_count);
    println!("   Active spans: {}", snapshot.active_span_count);
    println!("   Completed spans: {}", snapshot.completed_span_count);
    println!("   Triggered alerts: {}", snapshot.alert_count);

    // Export example
    println!("\n   Exporting as JSON...");
    let json = telemetry.export_json()?;
    println!("   Export size: {} bytes", json.len());
    println!("   ✅ Export complete\n");

    println!("🎉 Observability demo completed successfully!");

    Ok(())
}
