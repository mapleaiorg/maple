//! Presence gradient probe.
//!
//! Measures whether an agent is "present" in the resonance field.
//! This is NOT a simple liveness check - it measures the agent's
//! presence gradient, which indicates how strongly the agent is
//! manifesting in the shared reality.

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
}

impl PresenceProbe {
    /// Create a new presence probe with default settings.
    pub fn new() -> Self {
        Self {
            min_gradient: 0.1,
            timeout_ms: 5000,
        }
    }

    /// Create a presence probe with custom settings.
    pub fn with_settings(min_gradient: f64, timeout_ms: u64) -> Self {
        Self {
            min_gradient: min_gradient.clamp(0.0, 1.0),
            timeout_ms,
        }
    }

    /// Measure the presence gradient for an instance.
    ///
    /// In a real implementation, this would query the resonator-runtime
    /// to get the actual presence gradient from the resonance field.
    async fn measure_gradient(&self, instance_id: &InstanceId) -> HealthResult<f64> {
        // TODO: Integration point with resonator-runtime
        // This would call into the resonance field to measure actual presence
        //
        // For now, simulate a measurement. In production:
        // 1. Query the instance's presence in the resonance field
        // 2. Measure signal strength and coherence
        // 3. Return normalized gradient (0.0-1.0)

        debug!(
            instance_id = %instance_id,
            "Measuring presence gradient"
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
            Err(_) => {
                Ok(ProbeResult::timeout(
                    instance_id,
                    ProbeType::Presence,
                    self.timeout_ms,
                ))
            }
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
}
