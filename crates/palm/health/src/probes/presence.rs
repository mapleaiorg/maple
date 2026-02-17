//! Presence gradient probe.
//!
//! Measures whether an agent is "present" in the resonance field.
//! This is NOT a simple liveness check - it measures the agent's
//! presence gradient, which indicates how strongly the agent is
//! manifesting in the shared reality.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use palm_types::InstanceId;
use tracing::{debug, instrument};

use super::{Probe, ProbeResult, ProbeType};
use crate::error::{HealthError, HealthResult};

/// Probe for measuring presence gradient.
///
/// Presence gradient indicates how "present" an agent is in the
/// resonance field. A high gradient means the agent is strongly
/// manifesting; a low gradient means the agent is fading.
pub struct PresenceProbe {
    /// Minimum acceptable presence gradient (0.0-1.0).
    min_gradient: f64,

    /// Timeout for probe execution in milliseconds.
    timeout_ms: u64,

    /// Optional external metric reader for real runtime integrations.
    metric_reader: Option<Arc<PresenceMetricReader>>,
}

type PresenceMetricReader = dyn Fn(&InstanceId) -> HealthResult<f64> + Send + Sync;

impl PresenceProbe {
    /// Create a new presence probe with default settings.
    pub fn new() -> Self {
        Self {
            min_gradient: 0.1,
            timeout_ms: 5000,
            metric_reader: None,
        }
    }

    /// Create a presence probe with custom settings.
    pub fn with_settings(min_gradient: f64, timeout_ms: u64) -> Self {
        Self {
            min_gradient: min_gradient.clamp(0.0, 1.0),
            timeout_ms,
            metric_reader: None,
        }
    }

    /// Attach an external metric reader for production/runtime integrations.
    pub fn with_metric_reader<F>(mut self, reader: F) -> Self
    where
        F: Fn(&InstanceId) -> HealthResult<f64> + Send + Sync + 'static,
    {
        self.metric_reader = Some(Arc::new(reader));
        self
    }

    /// Measure the presence gradient for an instance.
    ///
    /// Uses an injected metric reader when available; otherwise falls back to
    /// a lightweight synthetic probe suitable for local development.
    async fn measure_gradient(&self, instance_id: &InstanceId) -> HealthResult<f64> {
        if let Some(reader) = &self.metric_reader {
            return reader(instance_id);
        }

        debug!(
            instance_id = %instance_id,
            "Measuring presence gradient (synthetic fallback)"
        );

        // Simulate presence measurement
        // In reality, this would be an async call to resonator-runtime
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Return simulated healthy gradient
        // Real implementation would return actual measured value
        Ok(0.85)
    }
}

impl Default for PresenceProbe {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Probe for PresenceProbe {
    fn probe_type(&self) -> ProbeType {
        ProbeType::Presence
    }

    #[instrument(skip(self), fields(probe = "presence"))]
    async fn execute(&self, instance_id: InstanceId) -> HealthResult<ProbeResult> {
        let start = Instant::now();

        // Apply timeout to the measurement
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(self.timeout_ms),
            self.measure_gradient(&instance_id),
        )
        .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(gradient)) => {
                if gradient >= self.min_gradient {
                    Ok(ProbeResult::success(
                        instance_id,
                        ProbeType::Presence,
                        gradient,
                        latency_ms,
                    ))
                } else {
                    Ok(ProbeResult::failure(
                        instance_id,
                        ProbeType::Presence,
                        format!(
                            "Presence gradient {} below minimum {}",
                            gradient, self.min_gradient
                        ),
                        latency_ms,
                    ))
                }
            }
            Ok(Err(e)) => Err(HealthError::ProbeFailed {
                instance_id,
                reason: e.to_string(),
            }),
            Err(_) => Ok(ProbeResult::timeout(
                instance_id,
                ProbeType::Presence,
                self.timeout_ms,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_presence_probe() {
        let probe = PresenceProbe::new();
        let instance_id = InstanceId::generate();

        let result = probe.execute(instance_id.clone()).await.unwrap();

        assert!(result.success);
        assert_eq!(result.probe_type, ProbeType::Presence);
        assert!(result.value.unwrap() > 0.0);
    }

    #[tokio::test]
    async fn test_presence_probe_custom_reader_failure() {
        let probe = PresenceProbe::with_settings(0.9, 1000).with_metric_reader(|_| Ok(0.4));
        let instance_id = InstanceId::generate();

        let result = probe.execute(instance_id).await.unwrap();
        assert!(!result.success);
    }
}
