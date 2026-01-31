//! Custom application-defined probes.
//!
//! Allows applications to define their own health probes that
//! integrate with the PALM health monitoring system.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use palm_types::InstanceId;
use tracing::{debug, instrument};

use super::{Probe, ProbeResult, ProbeType};
use crate::error::{HealthError, HealthResult};

/// Custom probe implementation.
///
/// Applications can create custom probes to measure domain-specific
/// health metrics that go beyond the standard presence/coupling/attention.
pub struct CustomProbe {
    /// Name of the custom probe.
    name: String,

    /// Minimum acceptable value (0.0-1.0).
    min_value: f64,

    /// Timeout for probe execution in milliseconds.
    timeout_ms: u64,

    /// The actual probe implementation.
    implementation: Arc<dyn CustomProbeImplementation>,
}

impl CustomProbe {
    /// Create a new custom probe.
    pub fn new(
        name: impl Into<String>,
        min_value: f64,
        timeout_ms: u64,
        implementation: Arc<dyn CustomProbeImplementation>,
    ) -> Self {
        Self {
            name: name.into(),
            min_value: min_value.clamp(0.0, 1.0),
            timeout_ms,
            implementation,
        }
    }

    /// Get the probe name.
    pub fn probe_name(&self) -> &str {
        &self.name
    }
}

#[async_trait]
impl Probe for CustomProbe {
    fn probe_type(&self) -> ProbeType {
        ProbeType::Custom
    }

    fn name(&self) -> &str {
        &self.name
    }

    #[instrument(skip(self), fields(probe = %self.name))]
    async fn execute(&self, instance_id: InstanceId) -> HealthResult<ProbeResult> {
        let start = Instant::now();

        debug!(
            instance_id = %instance_id,
            probe_name = %self.name,
            "Executing custom probe"
        );

        // Apply timeout to the measurement
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(self.timeout_ms),
            self.implementation.measure(&instance_id),
        )
        .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(value)) => {
                if value >= self.min_value {
                    Ok(ProbeResult::success(
                        instance_id,
                        ProbeType::Custom,
                        value,
                        latency_ms,
                    ))
                } else {
                    Ok(ProbeResult::failure(
                        instance_id,
                        ProbeType::Custom,
                        format!(
                            "Custom probe '{}' value {} below minimum {}",
                            self.name, value, self.min_value
                        ),
                        latency_ms,
                    ))
                }
            }
            Ok(Err(e)) => Err(HealthError::ProbeFailed {
                instance_id,
                reason: format!("Custom probe '{}' failed: {}", self.name, e),
            }),
            Err(_) => {
                Ok(ProbeResult::timeout(
                    instance_id,
                    ProbeType::Custom,
                    self.timeout_ms,
                ))
            }
        }
    }
}

/// Trait for custom probe implementations.
///
/// Implement this trait to create application-specific health probes.
#[async_trait]
pub trait CustomProbeImplementation: Send + Sync {
    /// Measure the health metric for an instance.
    ///
    /// Returns a value between 0.0 and 1.0.
    async fn measure(&self, instance_id: &InstanceId) -> Result<f64, String>;

    /// Get a description of what this probe measures.
    fn description(&self) -> &str {
        "Custom health metric"
    }
}

/// Factory for creating custom probes.
pub struct CustomProbeFactory;

impl CustomProbeFactory {
    /// Create a simple custom probe from a closure.
    pub fn from_fn<F, Fut>(
        name: impl Into<String>,
        min_value: f64,
        timeout_ms: u64,
        measure_fn: F,
    ) -> CustomProbe
    where
        F: Fn(InstanceId) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<f64, String>> + Send + 'static,
    {
        CustomProbe::new(
            name,
            min_value,
            timeout_ms,
            Arc::new(ClosureProbe { measure_fn }),
        )
    }
}

/// Probe implementation using a closure.
struct ClosureProbe<F> {
    measure_fn: F,
}

#[async_trait]
impl<F, Fut> CustomProbeImplementation for ClosureProbe<F>
where
    F: Fn(InstanceId) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<f64, String>> + Send,
{
    async fn measure(&self, instance_id: &InstanceId) -> Result<f64, String> {
        (self.measure_fn)(instance_id.clone()).await
    }
}

/// A no-op probe implementation for testing.
#[derive(Debug, Clone, Default)]
pub struct NoOpProbe {
    value: f64,
}

impl NoOpProbe {
    /// Create a no-op probe that always returns the given value.
    pub fn new(value: f64) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
        }
    }
}

#[async_trait]
impl CustomProbeImplementation for NoOpProbe {
    async fn measure(&self, _instance_id: &InstanceId) -> Result<f64, String> {
        Ok(self.value)
    }

    fn description(&self) -> &str {
        "No-op probe for testing"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_custom_probe_with_noop() {
        let noop = Arc::new(NoOpProbe::new(0.9));
        let probe = CustomProbe::new("test-probe", 0.5, 1000, noop);
        let instance_id = InstanceId::generate();

        let result = probe.execute(instance_id).await.unwrap();

        assert!(result.success);
        assert_eq!(result.probe_type, ProbeType::Custom);
        assert_eq!(result.value, Some(0.9));
    }

    #[tokio::test]
    async fn test_custom_probe_factory() {
        let probe = CustomProbeFactory::from_fn("factory-probe", 0.5, 1000, |_id| async {
            Ok(0.75)
        });

        let instance_id = InstanceId::generate();
        let result = probe.execute(instance_id).await.unwrap();

        assert!(result.success);
        assert_eq!(result.value, Some(0.75));
    }
}
