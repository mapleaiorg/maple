//! Central metrics registry for PALM

use prometheus::{Encoder, Registry, TextEncoder};
use std::sync::Arc;

/// Central metrics registry for PALM
pub struct MetricsRegistry {
    registry: Arc<Registry>,
    palm_metrics: super::collectors::PalmMetrics,
}

impl MetricsRegistry {
    /// Create a new metrics registry with default prefix "palm"
    pub fn new() -> Self {
        Self::with_prefix("palm")
    }

    /// Create a new metrics registry with custom prefix
    pub fn with_prefix(prefix: &str) -> Self {
        let registry = Arc::new(
            Registry::new_custom(Some(prefix.to_string()), None)
                .expect("Failed to create metrics registry"),
        );
        let palm_metrics = super::collectors::PalmMetrics::new(&registry);

        Self {
            registry,
            palm_metrics,
        }
    }

    /// Get the PALM-specific metrics
    pub fn palm(&self) -> &super::collectors::PalmMetrics {
        &self.palm_metrics
    }

    /// Export metrics in Prometheus text format
    pub fn export(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder
            .encode(&metric_families, &mut buffer)
            .expect("Failed to encode metrics");
        String::from_utf8(buffer).expect("Metrics output is not valid UTF-8")
    }

    /// Get the underlying registry for custom metrics
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = MetricsRegistry::new();
        // Record a metric value to ensure export works
        registry
            .palm()
            .deployment
            .record_operation("development", "create", "success", 1.0);
        let output = registry.export();
        // Should have some metrics
        assert!(!output.is_empty());
        assert!(output.contains("deployment_operations_total"));
    }

    #[test]
    fn test_registry_with_prefix() {
        let registry = MetricsRegistry::with_prefix("test");
        // Record a metric value to ensure export works
        registry.palm().instance.record_startup("development", 1.0);
        let output = registry.export();
        assert!(!output.is_empty());
        assert!(output.contains("instance_startup_duration_seconds"));
    }
}
