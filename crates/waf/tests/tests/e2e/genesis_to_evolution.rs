//! End-to-end test: Genesis boot -> Kernel creation -> Multiple evolution steps -> Metrics tracking.
//!
//! Verifies the full WAF lifecycle from initial boot through sustained autopoietic evolution.

use maple_waf_genesis::{create_worldline, genesis_boot, GenesisPhase, SeedConfig};
use maple_waf_kernel::{AutopoieticKernel, EvolutionStepResult};
use maple_waf_resonance_monitor::SystemMetrics;

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
async fn genesis_boot_completes_all_phases() {
    let result = genesis_boot(SeedConfig::default()).await.unwrap();

    assert_eq!(result.phase_reached, GenesisPhase::Complete);
    assert_eq!(result.invariants_verified, 14);
    assert!(result.initial_resonance >= 0.6);
    assert!(result.genesis_duration_ms < 10_000);
}

#[tokio::test]
async fn genesis_demo_and_production_configs_boot() {
    let demo = genesis_boot(SeedConfig::demo()).await.unwrap();
    assert_eq!(demo.phase_reached, GenesisPhase::Complete);

    let prod = genesis_boot(SeedConfig::production()).await.unwrap();
    assert_eq!(prod.phase_reached, GenesisPhase::Complete);

    // Production has stricter resonance than demo.
    assert!(prod.initial_resonance >= demo.initial_resonance);
}

#[tokio::test]
async fn create_worldline_produces_complete_worldline() {
    let wl = create_worldline(SeedConfig::default()).await.unwrap();

    assert_eq!(wl.phase, GenesisPhase::Complete);
    assert!(wl.config.resonance_min > 0.0);
}

#[tokio::test]
async fn kernel_from_worldline_starts_at_step_zero() {
    let wl = create_worldline(SeedConfig::default()).await.unwrap();
    let kernel = AutopoieticKernel::from_worldline(wl).unwrap();

    assert_eq!(kernel.step_count(), 0);
    assert_eq!(kernel.metrics().steps_attempted, 0);
    assert_eq!(kernel.metrics().evolutions_succeeded, 0);
    assert_eq!(kernel.metrics().evolutions_failed, 0);
}

#[tokio::test]
async fn healthy_metrics_produce_healthy_result() {
    let wl = create_worldline(SeedConfig::default()).await.unwrap();
    let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();

    let result = kernel.step_evolution(&healthy_metrics()).await.unwrap();
    assert!(matches!(result, EvolutionStepResult::Healthy { .. }));
    assert_eq!(kernel.step_count(), 1);
}

#[tokio::test]
async fn stressed_metrics_trigger_evolution() {
    let wl = create_worldline(SeedConfig::default()).await.unwrap();
    let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();

    let result = kernel.step_evolution(&stressed_metrics()).await.unwrap();
    assert!(matches!(result, EvolutionStepResult::Evolved { .. }));
    assert_eq!(kernel.step_count(), 1);
}

#[tokio::test]
async fn multiple_healthy_steps_tracked_correctly() {
    let wl = create_worldline(SeedConfig::default()).await.unwrap();
    let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();

    for _ in 0..10 {
        let result = kernel.step_evolution(&healthy_metrics()).await.unwrap();
        assert!(matches!(result, EvolutionStepResult::Healthy { .. }));
    }

    assert_eq!(kernel.step_count(), 10);
}

#[tokio::test]
async fn mixed_steps_track_successes_and_healthy() {
    let wl = create_worldline(SeedConfig::default()).await.unwrap();
    let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();

    // Alternate healthy and stressed metrics.
    kernel.step_evolution(&healthy_metrics()).await.unwrap();
    kernel.step_evolution(&stressed_metrics()).await.unwrap();
    kernel.step_evolution(&healthy_metrics()).await.unwrap();
    kernel.step_evolution(&stressed_metrics()).await.unwrap();

    assert_eq!(kernel.step_count(), 4);
    // At least one evolution should have succeeded from stressed metrics.
    assert!(kernel.metrics().evolutions_succeeded >= 1);
}

#[tokio::test]
async fn resonance_history_recorded_across_steps() {
    let wl = create_worldline(SeedConfig::default()).await.unwrap();
    let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();

    kernel.step_evolution(&healthy_metrics()).await.unwrap();
    kernel.step_evolution(&stressed_metrics()).await.unwrap();

    let history = &kernel.metrics().resonance_history;
    assert!(history.len() >= 2);
    // First step had healthy resonance 0.9, second had stressed 0.7.
    assert!(history.contains(&0.9));
    assert!(history.contains(&0.7));
}

#[tokio::test]
async fn max_steps_limit_enforced() {
    let config = SeedConfig {
        max_evolution_steps: 3,
        ..SeedConfig::default()
    };
    let wl = create_worldline(config).await.unwrap();
    let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();

    // Execute up to the limit.
    for _ in 0..3 {
        kernel.step_evolution(&healthy_metrics()).await.unwrap();
    }

    // The fourth step should fail.
    let result = kernel.step_evolution(&healthy_metrics()).await;
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("max"));
}

#[tokio::test]
async fn genesis_result_serializes_to_json() {
    let result = genesis_boot(SeedConfig::default()).await.unwrap();
    let json = serde_json::to_string(&result).unwrap();

    assert!(json.contains("worldline_id"));
    assert!(json.contains("Complete"));

    let restored: maple_waf_genesis::GenesisResult = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.phase_reached, GenesisPhase::Complete);
    assert_eq!(restored.invariants_verified, 14);
}
