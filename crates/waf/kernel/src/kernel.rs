use crate::error::KernelError;
use crate::metrics::KernelMetrics;
use maple_waf_context_graph::{
    ContextGraphManager, InMemoryContextGraphManager, IntentNode, NodeContent,
};
use maple_waf_evolution_engine::{HardwareContext, HypothesisEvaluator, SimulatedSynthesizer, Synthesizer};
use maple_waf_genesis::{GenesisPhase, SeedConfig, Worldline};
use maple_waf_resonance_monitor::{DissonanceThresholds, MonitorOrchestrator, SystemMetrics};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use worldline_types::{EventId, TemporalAnchor, WorldlineId};

/// Result of a single evolution step.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EvolutionStepResult {
    /// System is healthy, no evolution needed.
    Healthy { resonance: f64 },
    /// Evolution was performed successfully.
    Evolved { resonance: f64, description: String },
    /// Evidence gathering failed.
    EvidenceFailed { reason: String },
    /// Governance denied the evolution.
    Denied { reason: String },
    /// System rolled back after degradation.
    RolledBack { reason: String },
}

/// The Autopoietic Kernel — continuous self-evolution loop.
pub struct AutopoieticKernel {
    pub worldline_id: WorldlineId,
    pub config: SeedConfig,
    pub metrics: KernelMetrics,
    pub context_graph: InMemoryContextGraphManager,
    pub monitor: MonitorOrchestrator,
    pub evaluator: HypothesisEvaluator,
    pub hardware: HardwareContext,
    emergency_stop: Arc<AtomicBool>,
    step_count: u64,
}

impl AutopoieticKernel {
    /// Create a kernel from a genesis-booted worldline.
    pub fn from_worldline(worldline: Worldline) -> Result<Self, KernelError> {
        if worldline.phase != GenesisPhase::Complete {
            return Err(KernelError::NotInitialized);
        }

        Ok(Self {
            worldline_id: worldline.id,
            config: worldline.config,
            metrics: KernelMetrics::new(100),
            context_graph: InMemoryContextGraphManager::new(),
            monitor: MonitorOrchestrator::new(DissonanceThresholds::default()),
            evaluator: HypothesisEvaluator::new(),
            hardware: HardwareContext::simulated(),
            emergency_stop: Arc::new(AtomicBool::new(false)),
            step_count: 0,
        })
    }

    /// Get the emergency stop handle.
    pub fn emergency_stop_handle(&self) -> Arc<AtomicBool> {
        self.emergency_stop.clone()
    }

    /// Trigger emergency stop.
    pub fn trigger_emergency_stop(&self) {
        self.emergency_stop.store(true, Ordering::SeqCst);
    }

    /// Is emergency stopped?
    pub fn is_stopped(&self) -> bool {
        self.emergency_stop.load(Ordering::SeqCst)
    }

    /// Execute a single evolution step.
    pub async fn step_evolution(
        &mut self,
        metrics: &SystemMetrics,
    ) -> Result<EvolutionStepResult, KernelError> {
        // Check emergency stop.
        if self.is_stopped() {
            return Err(KernelError::EmergencyStop);
        }

        // Check max steps.
        if self.step_count >= self.config.max_evolution_steps {
            return Err(KernelError::MaxStepsReached(self.config.max_evolution_steps));
        }

        self.step_count += 1;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_millis() as u64;

        // Detect dissonance.
        let intents = self.monitor.process_metrics(metrics, now_ms);

        if intents.is_empty() {
            // No dissonance — system is healthy.
            self.metrics.record_resonance(metrics.resonance);
            return Ok(EvolutionStepResult::Healthy {
                resonance: metrics.resonance,
            });
        }

        // Check resonance threshold.
        if metrics.resonance < self.config.resonance_min {
            self.metrics.record_resonance(metrics.resonance);
            return Err(KernelError::ResonanceBelowMinimum {
                current: metrics.resonance,
                minimum: self.config.resonance_min,
            });
        }

        // Record the first intent in the context graph.
        let intent = &intents[0];
        let intent_node = IntentNode::new(
            EventId::new(),
            intent.description.clone(),
            intent.governance_tier,
        );

        let _intent_id = self
            .context_graph
            .append(
                self.worldline_id.clone(),
                NodeContent::Intent(intent_node),
                vec![],
                TemporalAnchor::now(0),
                intent.governance_tier,
            )
            .await
            .map_err(|e| KernelError::EvolutionFailed(format!("{}", e)))?;

        // Synthesize hypotheses.
        let synthesizer = SimulatedSynthesizer::new();
        let synthesis = synthesizer
            .synthesize(intent, &self.hardware)
            .await
            .map_err(|e| KernelError::EvolutionFailed(format!("{}", e)))?;

        // Evaluate and select best.
        let best = self.evaluator.select_best(&synthesis.hypotheses);

        match best {
            Some(hypothesis) => {
                self.metrics.record_success();
                self.metrics.record_resonance(metrics.resonance);
                Ok(EvolutionStepResult::Evolved {
                    resonance: metrics.resonance,
                    description: hypothesis.description,
                })
            }
            None => {
                self.metrics.record_failure();
                self.metrics.record_resonance(metrics.resonance);
                Ok(EvolutionStepResult::EvidenceFailed {
                    reason: "no viable hypothesis found".into(),
                })
            }
        }
    }

    /// Get current metrics.
    pub fn metrics(&self) -> &KernelMetrics {
        &self.metrics
    }

    /// Current step count.
    pub fn step_count(&self) -> u64 {
        self.step_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_waf_genesis::{create_worldline, SeedConfig};

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

    #[tokio::test]
    async fn kernel_from_worldline() {
        let wl = create_worldline(SeedConfig::default()).await.unwrap();
        let kernel = AutopoieticKernel::from_worldline(wl).unwrap();
        assert_eq!(kernel.step_count(), 0);
    }

    #[tokio::test]
    async fn step_healthy_no_evolution() {
        let wl = create_worldline(SeedConfig::default()).await.unwrap();
        let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();
        let result = kernel.step_evolution(&healthy_metrics()).await.unwrap();
        assert!(matches!(result, EvolutionStepResult::Healthy { .. }));
    }

    #[tokio::test]
    async fn step_stressed_evolves() {
        let wl = create_worldline(SeedConfig::default()).await.unwrap();
        let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();
        let result = kernel.step_evolution(&stressed_metrics()).await.unwrap();
        assert!(matches!(result, EvolutionStepResult::Evolved { .. }));
    }

    #[tokio::test]
    async fn emergency_stop_blocks_step() {
        let wl = create_worldline(SeedConfig::default()).await.unwrap();
        let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();
        kernel.trigger_emergency_stop();
        let result = kernel.step_evolution(&healthy_metrics()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn max_steps_limit() {
        let config = SeedConfig {
            max_evolution_steps: 2,
            ..SeedConfig::default()
        };
        let wl = create_worldline(config).await.unwrap();
        let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();
        kernel.step_evolution(&healthy_metrics()).await.unwrap();
        kernel.step_evolution(&healthy_metrics()).await.unwrap();
        let result = kernel.step_evolution(&healthy_metrics()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn metrics_tracked() {
        let wl = create_worldline(SeedConfig::default()).await.unwrap();
        let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();
        kernel.step_evolution(&stressed_metrics()).await.unwrap();
        assert_eq!(kernel.metrics().steps_attempted, 1);
    }

    #[tokio::test]
    async fn multiple_steps() {
        let wl = create_worldline(SeedConfig::default()).await.unwrap();
        let mut kernel = AutopoieticKernel::from_worldline(wl).unwrap();
        for _ in 0..5 {
            kernel.step_evolution(&healthy_metrics()).await.unwrap();
        }
        assert_eq!(kernel.step_count(), 5);
    }
}
