//! End-to-end test: Emergency stop propagation through the kernel.
//!
//! Verifies that emergency stop halts all evolution processing and that
//! the stop flag propagates correctly across shared handles.

use maple_waf_genesis::{create_worldline, SeedConfig};
use maple_waf_kernel::{AutopoieticKernel, EvolutionStepResult};
use maple_waf_resonance_monitor::{DissonanceThresholds, MonitorOrchestrator, SystemMetrics};
use std::sync::atomic::Ordering;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn healthy_metrics() -> SystemMetrics {
    SystemMetrics {
        cpu_usage_pct: 40.0,
        memory_usage_mb: 2048.0,
        latency_p50_ms: 10.0,
        latency_p99_ms: 50.0,
        error_rate: 0.01,
        throughput_rps: 1000.0,
        api_friction_score: 0.1,
        policy_denial_rate: 0.02,
        resonance: 0.9,
    }
}

fn stressed_metrics() -> SystemMetrics {
    SystemMetrics {
        cpu_usage_pct: 95.0,
        memory_usage_mb: 6000.0,
        latency_p50_ms: 100.0,
        latency_p99_ms: 800.0,
        error_rate: 0.15,
        throughput_rps: 500.0,
        api_friction_score: 0.5,
        policy_denial_rate: 0.2,
        resonance: 0.7,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn emergency_stop_blocks_evolution_step() {
    let wl = create_worldline(SeedConfig::default()).await.unwrap();
    let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();

    kernel.trigger_emergency_stop();
    let result = kernel.step_evolution(&healthy_metrics()).await;
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("emergency") || err_msg.contains("stop"));
}

#[tokio::test]
async fn emergency_stop_via_shared_handle() {
    let wl = create_worldline(SeedConfig::default()).await.unwrap();
    let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();

    // Get the emergency stop handle and trigger from "outside".
    let handle = kernel.emergency_stop_handle();
    handle.store(true, Ordering::SeqCst);

    assert!(kernel.is_stopped());

    let result = kernel.step_evolution(&healthy_metrics()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn emergency_stop_after_successful_steps() {
    let wl = create_worldline(SeedConfig::default()).await.unwrap();
    let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();

    // Run a few successful steps.
    kernel.step_evolution(&healthy_metrics()).await.unwrap();
    kernel.step_evolution(&stressed_metrics()).await.unwrap();
    assert_eq!(kernel.step_count(), 2);

    // Trigger emergency stop.
    kernel.trigger_emergency_stop();

    // Subsequent steps should fail.
    let result = kernel.step_evolution(&healthy_metrics()).await;
    assert!(result.is_err());

    // Step count should not have incremented.
    assert_eq!(kernel.step_count(), 2);
}

#[tokio::test]
async fn emergency_stop_is_initially_false() {
    let wl = create_worldline(SeedConfig::default()).await.unwrap();
    let kernel = AutopoieticKernel::from_worldline(wl).unwrap();

    assert!(!kernel.is_stopped());
}

#[tokio::test]
async fn monitor_orchestrator_emergency_stop_blocks_intents() {
    let mut orch = MonitorOrchestrator::new(DissonanceThresholds::default());

    // Verify it produces intents normally under stress.
    let intents = orch.process_metrics(&stressed_metrics(), 1000);
    assert!(!intents.is_empty());

    // Trigger emergency stop.
    orch.trigger_emergency_stop();
    assert!(orch.is_stopped());

    // Now it should produce no intents even under stress.
    let intents = orch.process_metrics(&stressed_metrics(), 10000);
    assert!(intents.is_empty());
}

#[tokio::test]
async fn monitor_emergency_stop_clear_and_resume() {
    let mut orch = MonitorOrchestrator::new(DissonanceThresholds::default());

    orch.trigger_emergency_stop();
    assert!(orch.is_stopped());

    orch.clear_emergency_stop();
    assert!(!orch.is_stopped());

    // Should produce intents again.
    let intents = orch.process_metrics(&stressed_metrics(), 20000);
    assert!(!intents.is_empty());
}

#[tokio::test]
async fn monitor_emergency_stop_via_shared_handle() {
    let orch = MonitorOrchestrator::new(DissonanceThresholds::default());
    let handle = orch.emergency_stop_handle();

    assert!(!orch.is_stopped());

    handle.store(true, Ordering::SeqCst);
    assert!(orch.is_stopped());

    handle.store(false, Ordering::SeqCst);
    assert!(!orch.is_stopped());
}
