//! Coupling capacity probe.
//!
//! Measures an agent's ability to couple with other agents in the
//! resonance field. Coupling is the mechanism by which agents
//! coordinate and share state.

use std::time::Instant;

use async_trait::async_trait;
use palm_types::InstanceId;
use tracing::{debug, instrument};

use super::{Probe, ProbeResult, ProbeType};
use crate::error::{HealthError, HealthResult};

/// Probe for measuring coupling capacity.
///
/// Coupling capacity indicates how well an agent can form and
/// maintain couplings with other agents. High capacity means
/// the agent can participate in complex multi-agent interactions;
/// low capacity means the agent may be overwhelmed or degraded.
pub struct CouplingProbe {
    /// Minimum acceptable coupling capacity (0.0-1.0).
    min_capacity: f64,

    /// Timeout for probe execution in milliseconds.
    timeout_ms: u64,

    /// Number of test couplings to attempt.
    test_coupling_count: u32,
}

impl CouplingProbe {
    /// Create a new coupling probe with default settings.
    pub fn new() -> Self {
        Self {
            min_capacity: 0.2,
            timeout_ms: 5000,
            test_coupling_count: 3,
        }
    }

    /// Create a coupling probe with custom settings.
    pub fn with_settings(min_capacity: f64, timeout_ms: u64, test_coupling_count: u32) -> Self {
        Self {
            min_capacity: min_capacity.clamp(0.0, 1.0),
            timeout_ms,
            test_coupling_count: test_coupling_count.max(1),
        }
    }

    /// Measure the coupling capacity for an instance.
    ///
    /// In a real implementation, this would:
    /// 1. Attempt test couplings with synthetic or known-good agents
    /// 2. Measure coupling latency and success rate
    /// 3. Calculate overall coupling capacity
    async fn measure_capacity(&self, instance_id: &InstanceId) -> HealthResult<f64> {
        // TODO: Integration point with resonator-runtime
        // This would attempt actual couplings to measure capacity
        //
        // For now, simulate a measurement. In production:
        // 1. Create test coupling requests
        // 2. Measure time to establish coupling
        // 3. Verify coupling stability
        // 4. Calculate capacity based on success rate and latency

        debug!(
            instance_id = %instance_id,
            test_count = self.test_coupling_count,
            "Measuring coupling capacity"
        );

        // Simulate coupling capacity measurement
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // Return simulated healthy capacity
        // Real implementation would return actual measured value
        Ok(0.78)
    }
}

impl Default for CouplingProbe {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Probe for CouplingProbe {
    fn probe_type(&self) -> ProbeType {
        ProbeType::Coupling
    }

    #[instrument(skip(self), fields(probe = "coupling"))]
    async fn execute(&self, instance_id: InstanceId) -> HealthResult<ProbeResult> {
        let start = Instant::now();

        // Apply timeout to the measurement
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(self.timeout_ms),
            self.measure_capacity(&instance_id),
        )
        .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(capacity)) => {
                if capacity >= self.min_capacity {
                    Ok(ProbeResult::success(
                        instance_id,
                        ProbeType::Coupling,
                        capacity,
                        latency_ms,
                    ))
                } else {
                    Ok(ProbeResult::failure(
                        instance_id,
                        ProbeType::Coupling,
                        format!(
                            "Coupling capacity {} below minimum {}",
                            capacity, self.min_capacity
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
                ProbeType::Coupling,
                self.timeout_ms,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coupling_probe() {
        let probe = CouplingProbe::new();
        let instance_id = InstanceId::generate();

        let result = probe.execute(instance_id.clone()).await.unwrap();

        assert!(result.success);
        assert_eq!(result.probe_type, ProbeType::Coupling);
        assert!(result.value.unwrap() > 0.0);
    }
}
