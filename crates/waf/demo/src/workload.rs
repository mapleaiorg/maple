//! Simulated workload generator for the WAF demo.
//!
//! Provides varying [`SystemMetrics`] snapshots that simulate healthy,
//! stressed, and gradually degrading system conditions without requiring
//! any external services.

use maple_waf_resonance_monitor::SystemMetrics;

/// Generates simulated [`SystemMetrics`] for demo purposes.
///
/// Each method produces a deterministic metrics snapshot representing a
/// different operating regime, allowing the demo to exercise the full
/// range of kernel behaviour (healthy idle, stressed evolution, and
/// gradual degradation).
pub struct SimulatedWorkload;

impl SimulatedWorkload {
    /// Metrics representing a healthy, lightly loaded system.
    ///
    /// All values are well within normal thresholds and resonance is high.
    pub fn healthy_metrics() -> SystemMetrics {
        SystemMetrics {
            cpu_usage_pct: 35.0,
            memory_usage_mb: 1800.0,
            latency_p50_ms: 8.0,
            latency_p99_ms: 42.0,
            error_rate: 0.005,
            throughput_rps: 1200.0,
            api_friction_score: 0.08,
            policy_denial_rate: 0.01,
            resonance: 0.92,
        }
    }

    /// Metrics representing a system under heavy stress.
    ///
    /// CPU and memory are elevated, latencies are high, error rate is
    /// above the default threshold, and resonance is reduced.  This
    /// triggers dissonance detection and evolution synthesis.
    pub fn stressed_metrics() -> SystemMetrics {
        SystemMetrics {
            cpu_usage_pct: 92.0,
            memory_usage_mb: 5500.0,
            latency_p50_ms: 95.0,
            latency_p99_ms: 750.0,
            error_rate: 0.12,
            throughput_rps: 450.0,
            api_friction_score: 0.45,
            policy_denial_rate: 0.18,
            resonance: 0.65,
        }
    }

    /// Metrics that degrade over time based on the evolution step number.
    ///
    /// At `step == 0` the system is near-healthy; as `step` increases the
    /// metrics deteriorate smoothly.  Resonance drops linearly from 0.88
    /// down towards 0.55 over 10 steps.
    pub fn degrading_metrics(step: usize) -> SystemMetrics {
        let t = (step as f64 / 10.0).min(1.0);
        SystemMetrics {
            cpu_usage_pct: 40.0 + 50.0 * t,
            memory_usage_mb: 2000.0 + 3500.0 * t,
            latency_p50_ms: 10.0 + 85.0 * t,
            latency_p99_ms: 50.0 + 650.0 * t,
            error_rate: 0.01 + 0.12 * t,
            throughput_rps: 1100.0 - 700.0 * t,
            api_friction_score: 0.1 + 0.4 * t,
            policy_denial_rate: 0.02 + 0.16 * t,
            resonance: 0.88 - 0.33 * t,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_metrics_are_within_bounds() {
        let m = SimulatedWorkload::healthy_metrics();
        assert!(m.cpu_usage_pct > 0.0 && m.cpu_usage_pct < 100.0);
        assert!(m.memory_usage_mb > 0.0);
        assert!(m.resonance > 0.8);
        assert!(m.error_rate < 0.05);
    }

    #[test]
    fn stressed_metrics_show_pressure() {
        let m = SimulatedWorkload::stressed_metrics();
        assert!(m.cpu_usage_pct > 80.0);
        assert!(m.error_rate > 0.05);
        assert!(m.resonance < 0.8);
    }

    #[test]
    fn degrading_metrics_worsen_over_steps() {
        let early = SimulatedWorkload::degrading_metrics(0);
        let late = SimulatedWorkload::degrading_metrics(8);

        assert!(late.cpu_usage_pct > early.cpu_usage_pct);
        assert!(late.error_rate > early.error_rate);
        assert!(late.resonance < early.resonance);
    }

    #[test]
    fn degrading_metrics_clamp_at_max() {
        let m10 = SimulatedWorkload::degrading_metrics(10);
        let m20 = SimulatedWorkload::degrading_metrics(20);

        // After step 10 the metrics should plateau.
        assert!((m10.cpu_usage_pct - m20.cpu_usage_pct).abs() < f64::EPSILON);
        assert!((m10.resonance - m20.resonance).abs() < f64::EPSILON);
    }
}
