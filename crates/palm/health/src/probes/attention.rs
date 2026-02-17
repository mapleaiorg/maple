//! Attention budget probe.
//!
//! Measures an agent's remaining attention budget. Attention is
//! a finite resource in the MAPLE framework that governs how much
//! cognitive work an agent can perform.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use palm_types::InstanceId;
use tracing::{debug, instrument};

use super::{Probe, ProbeResult, ProbeType};
use crate::error::{HealthError, HealthResult};

/// Probe for measuring attention budget.
///
/// Attention budget indicates how much "cognitive capacity" an
/// agent has remaining. When attention is depleted, the agent
/// cannot process new requests and must wait for replenishment.
pub struct AttentionProbe {
    /// Minimum acceptable attention budget (0.0-1.0).
    min_budget: f64,

    /// Timeout for probe execution in milliseconds.
    timeout_ms: u64,

    /// Optional external metric reader for real runtime integrations.
    metric_reader: Option<Arc<AttentionMetricReader>>,
}

type AttentionMetricReader = dyn Fn(&InstanceId) -> HealthResult<f64> + Send + Sync;

impl AttentionProbe {
    /// Create a new attention probe with default settings.
    pub fn new() -> Self {
        Self {
            min_budget: 0.1,
            timeout_ms: 5000,
            metric_reader: None,
        }
    }

    /// Create an attention probe with custom settings.
    pub fn with_settings(min_budget: f64, timeout_ms: u64) -> Self {
        Self {
            min_budget: min_budget.clamp(0.0, 1.0),
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

    /// Measure the attention budget for an instance.
    ///
    /// Uses an injected metric reader when available; otherwise falls back to
    /// a lightweight synthetic probe suitable for local development.
    async fn measure_budget(&self, instance_id: &InstanceId) -> HealthResult<f64> {
        if let Some(reader) = &self.metric_reader {
            return reader(instance_id);
        }

        debug!(
            instance_id = %instance_id,
            "Measuring attention budget (synthetic fallback)"
        );

        // Simulate attention budget measurement
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;

        // Return simulated healthy budget
        // Real implementation would return actual measured value
        Ok(0.72)
    }
}

impl Default for AttentionProbe {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Probe for AttentionProbe {
    fn probe_type(&self) -> ProbeType {
        ProbeType::Attention
    }

    #[instrument(skip(self), fields(probe = "attention"))]
    async fn execute(&self, instance_id: InstanceId) -> HealthResult<ProbeResult> {
        let start = Instant::now();

        // Apply timeout to the measurement
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(self.timeout_ms),
            self.measure_budget(&instance_id),
        )
        .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(budget)) => {
                if budget >= self.min_budget {
                    Ok(ProbeResult::success(
                        instance_id,
                        ProbeType::Attention,
                        budget,
                        latency_ms,
                    ))
                } else {
                    Ok(ProbeResult::failure(
                        instance_id,
                        ProbeType::Attention,
                        format!(
                            "Attention budget {} below minimum {}",
                            budget, self.min_budget
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
                ProbeType::Attention,
                self.timeout_ms,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_attention_probe() {
        let probe = AttentionProbe::new();
        let instance_id = InstanceId::generate();

        let result = probe.execute(instance_id.clone()).await.unwrap();

        assert!(result.success);
        assert_eq!(result.probe_type, ProbeType::Attention);
        assert!(result.value.unwrap() > 0.0);
    }

    #[tokio::test]
    async fn test_attention_probe_custom_reader_failure() {
        let probe = AttentionProbe::with_settings(0.9, 1000).with_metric_reader(|_| Ok(0.2));
        let instance_id = InstanceId::generate();

        let result = probe.execute(instance_id).await.unwrap();
        assert!(!result.success);
    }
}
