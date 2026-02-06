//! # Observability Demo Example
//!
//! This example demonstrates:
//! - Metrics collection for pipeline stages
//! - Distributed tracing with spans
//! - Alert rules for monitoring
//!
//! Run with: `cargo run --example 09_observability_demo`

use resonator_observability::{
    MetricsCollector, SpanTracker, AlertEngine,
    AlertRule, AlertSeverity, AlertOperator, PipelineStage,
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
    let spans = SpanTracker::default();
    let alerts = AlertEngine::default();

    println!("   ✅ MetricsCollector initialized");
    println!("   ✅ SpanTracker initialized");
    println!("   ✅ AlertEngine initialized\n");

    // Configure alert rules
    println!("🚨 Configuring Alert Rules");
    println!("═══════════════════════════════════════════════════════\n");

    alerts.add_rule(AlertRule {
        name: "high_failure_rate".to_string(),
        description: "High commitment failure rate".to_string(),
        severity: AlertSeverity::Warning,
        metric: "commitment.failed".to_string(),
        threshold: 5.0,
        operator: AlertOperator::GreaterThan,
        enabled: true,
    })?;
    println!("   Added: high_failure_rate (Warning)");

    alerts.add_rule(AlertRule {
        name: "critical_failures".to_string(),
        description: "Critical commitment failures".to_string(),
        severity: AlertSeverity::Critical,
        metric: "commitment.failed".to_string(),
        threshold: 10.0,
        operator: AlertOperator::GreaterThan,
        enabled: true,
    })?;
    println!("   Added: critical_failures (Critical)\n");

    // Simulate pipeline activity with metrics
    println!("🔄 Simulating Pipeline Activity");
    println!("═══════════════════════════════════════════════════════\n");

    // Start a root span for the entire operation
    let root_span = spans.start_span("pipeline.full_cycle");
    println!("   Started span: pipeline.full_cycle (id: {})", root_span.id.0);

    // Presence stage
    metrics.record_pipeline_request(PipelineStage::Presence);
    sleep(Duration::from_millis(10)).await;
    metrics.record_pipeline_latency(PipelineStage::Presence, 10.0);
    println!("   ✓ Presence stage (10ms)");

    // Coupling stage
    metrics.record_pipeline_request(PipelineStage::Coupling);
    sleep(Duration::from_millis(20)).await;
    metrics.record_pipeline_latency(PipelineStage::Coupling, 20.0);
    println!("   ✓ Coupling stage (20ms)");

    // Meaning formation stage
    metrics.record_pipeline_request(PipelineStage::Meaning);
    sleep(Duration::from_millis(45)).await;
    metrics.record_pipeline_latency(PipelineStage::Meaning, 45.0);
    println!("   ✓ Meaning stage (45ms)");

    // Intent stabilization stage
    metrics.record_pipeline_request(PipelineStage::Intent);
    sleep(Duration::from_millis(120)).await;
    metrics.record_pipeline_latency(PipelineStage::Intent, 120.0);
    println!("   ✓ Intent stage (120ms)");

    // Commitment stage
    metrics.record_pipeline_request(PipelineStage::Commitment);

    // Simulate multiple commitments
    for _ in 0..10 {
        metrics.record_commitment_created();
    }
    for _ in 0..7 {
        metrics.record_commitment_completed();
    }
    for _ in 0..3 {
        metrics.record_commitment_failed();
    }

    sleep(Duration::from_millis(50)).await;
    metrics.record_pipeline_latency(PipelineStage::Commitment, 50.0);
    println!("   ✓ Commitment stage (50ms) - 10 created, 7 completed, 3 failed");

    // Consequence stage
    metrics.record_pipeline_request(PipelineStage::Consequence);
    for _ in 0..7 {
        metrics.record_consequence();
    }
    sleep(Duration::from_millis(30)).await;
    metrics.record_pipeline_latency(PipelineStage::Consequence, 30.0);
    println!("   ✓ Consequence stage (30ms)");

    // Complete root span
    spans.complete_span(&root_span.id)?;
    println!("\n   Total pipeline time: ~275ms\n");

    // Display what was tracked
    println!("📈 Metrics Collected");
    println!("═══════════════════════════════════════════════════════\n");

    println!("   Pipeline Stages Tracked:");
    println!("     - Presence: 10ms");
    println!("     - Coupling: 20ms");
    println!("     - Meaning: 45ms");
    println!("     - Intent: 120ms");
    println!("     - Commitment: 50ms");
    println!("     - Consequence: 30ms");
    println!();
    println!("   Commitment Metrics:");
    println!("     - Created: 10");
    println!("     - Completed: 7");
    println!("     - Failed: 3");
    println!();
    println!("   Consequence Metrics:");
    println!("     - Recorded: 7");

    // Display trace summary
    println!("\n📍 Trace Summary");
    println!("═══════════════════════════════════════════════════════\n");
    println!("   Root span: pipeline.full_cycle");
    println!("   Status: Completed");

    println!("\n📦 Telemetry Capabilities");
    println!("═══════════════════════════════════════════════════════\n");
    println!("   The observability system provides:");
    println!("   ✓ Real-time metrics for all pipeline stages");
    println!("   ✓ Distributed tracing with span hierarchies");
    println!("   ✓ Configurable alert rules with severity levels");
    println!("   ✓ Export to JSON, Prometheus, OpenTelemetry");
    println!();

    println!("🎉 Observability demo completed successfully!");

    Ok(())
}
