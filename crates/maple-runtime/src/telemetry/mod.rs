//! Telemetry and observability for MAPLE Resonance Runtime

use std::sync::RwLock;
use crate::runtime_core::ResonatorHandle;
use crate::config::TelemetryConfig;

/// Runtime telemetry system
///
/// Uses `RwLock` for thread-safe interior mutability, enabling
/// the runtime to be shared across async tasks (Send + Sync).
pub struct RuntimeTelemetry {
    config: TelemetryConfig,
    metrics: RwLock<MetricsCollector>,
}

impl RuntimeTelemetry {
    pub fn new(config: &TelemetryConfig) -> Self {
        Self {
            config: config.clone(),
            metrics: RwLock::new(MetricsCollector::new()),
        }
    }

    /// Record Resonator registration
    pub fn resonator_registered(&self, handle: &ResonatorHandle) {
        if !self.config.metrics_enabled {
            return;
        }

        tracing::info!("Resonator registered: {}", handle.id);
        self.metrics.write().unwrap().increment("resonator_registrations");
    }

    /// Record Resonator resume
    pub fn resonator_resumed(&self, handle: &ResonatorHandle) {
        if !self.config.metrics_enabled {
            return;
        }

        tracing::info!("Resonator resumed: {}", handle.id);
        self.metrics.write().unwrap().increment("resonator_resumes");
    }

    /// Flush telemetry data
    pub async fn flush(&self) {
        if !self.config.enabled {
            return;
        }

        tracing::debug!("Flushing telemetry");
        self.metrics.read().unwrap().flush();
    }

    /// Record coupling establishment
    pub fn coupling_established(&self, source: &str, target: &str, strength: f64) {
        if !self.config.metrics_enabled {
            return;
        }

        tracing::debug!(
            "Coupling established: {} -> {} (strength: {})",
            source,
            target,
            strength
        );
        self.metrics.write().unwrap().increment("coupling_establishments");
    }

    /// Record attention allocation
    pub fn attention_allocated(&self, resonator: &str, amount: u64) {
        if !self.config.detailed_metrics {
            return;
        }

        tracing::trace!("Attention allocated: {} (amount: {})", resonator, amount);
        self.metrics.write().unwrap().record_gauge("attention_allocated", amount as f64);
    }

    /// Record invariant violation
    pub fn invariant_violated(&self, invariant: &str) {
        if !self.config.metrics_enabled {
            return;
        }

        tracing::error!("Invariant violated: {}", invariant);
        self.metrics.write().unwrap().increment("invariant_violations");
    }
}

/// Metrics collector
struct MetricsCollector {
    counters: std::collections::HashMap<String, u64>,
    gauges: std::collections::HashMap<String, f64>,
}

impl MetricsCollector {
    fn new() -> Self {
        Self {
            counters: std::collections::HashMap::new(),
            gauges: std::collections::HashMap::new(),
        }
    }

    fn increment(&mut self, metric: &str) {
        *self.counters.entry(metric.to_string()).or_insert(0) += 1;
    }

    fn record_gauge(&mut self, metric: &str, value: f64) {
        self.gauges.insert(metric.to_string(), value);
    }

    fn flush(&self) {
        // In real implementation, would flush to metrics backend
        tracing::trace!("Metrics flushed (placeholder)");
    }
}
