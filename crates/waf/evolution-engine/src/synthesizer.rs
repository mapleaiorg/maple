use crate::error::EvolutionError;
use crate::types::{HardwareContext, Hypothesis, SynthesisResult};
use async_trait::async_trait;
#[cfg(test)]
use maple_waf_context_graph::GovernanceTier;
use maple_waf_context_graph::{ContentHash, IntentNode, SubstrateType};

/// Trait for synthesis providers (LLM-powered or simulated).
#[async_trait]
pub trait Synthesizer: Send + Sync {
    /// Generate hypotheses for a given intent.
    async fn synthesize(
        &self,
        intent: &IntentNode,
        hardware: &HardwareContext,
    ) -> Result<SynthesisResult, EvolutionError>;
}

/// Simulated synthesizer for testing — generates deterministic hypotheses.
pub struct SimulatedSynthesizer {
    model_id: String,
    hypothesis_count: usize,
    base_confidence: f64,
    base_safety: f64,
}

impl SimulatedSynthesizer {
    pub fn new() -> Self {
        Self {
            model_id: "simulated-synthesizer-v1".into(),
            hypothesis_count: 3,
            base_confidence: 0.8,
            base_safety: 0.9,
        }
    }

    pub fn with_hypothesis_count(mut self, count: usize) -> Self {
        self.hypothesis_count = count;
        self
    }

    pub fn with_base_confidence(mut self, c: f64) -> Self {
        self.base_confidence = c;
        self
    }

    pub fn with_base_safety(mut self, s: f64) -> Self {
        self.base_safety = s;
        self
    }
}

impl Default for SimulatedSynthesizer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Synthesizer for SimulatedSynthesizer {
    async fn synthesize(
        &self,
        intent: &IntentNode,
        _hardware: &HardwareContext,
    ) -> Result<SynthesisResult, EvolutionError> {
        let intent_hash = ContentHash::hash(intent.description.as_bytes());
        let mut hypotheses = Vec::new();

        for i in 0..self.hypothesis_count {
            let confidence = (self.base_confidence - (i as f64 * 0.1)).max(0.1);
            let safety = (self.base_safety - (i as f64 * 0.05)).max(0.1);

            hypotheses.push(
                Hypothesis::new(
                    format!("hyp_{}", i),
                    format!("Hypothesis {} for: {}", i, intent.description),
                    SubstrateType::Rust,
                    format!(
                        "// Simulated code for hypothesis {}\nfn optimized() {{ }}",
                        i
                    ),
                )
                .with_confidence(confidence)
                .with_safety_score(safety)
                .with_governance_tier(intent.governance_tier),
            );
        }

        // Sort by composite score (best first).
        hypotheses.sort_by(|a, b| {
            b.composite_score()
                .partial_cmp(&a.composite_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(SynthesisResult {
            intent_hash,
            hypotheses,
            model_id: self.model_id.clone(),
            synthesis_time_ms: 50,
        })
    }
}

/// Failing synthesizer for testing error paths.
pub struct FailingSynthesizer;

#[async_trait]
impl Synthesizer for FailingSynthesizer {
    async fn synthesize(
        &self,
        _intent: &IntentNode,
        _hardware: &HardwareContext,
    ) -> Result<SynthesisResult, EvolutionError> {
        Err(EvolutionError::LlmError("simulated LLM failure".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldline_types::EventId;

    fn test_intent() -> IntentNode {
        IntentNode::new(EventId::new(), "reduce latency", GovernanceTier::Tier1)
            .with_metric("latency_ms", -50.0)
    }

    #[tokio::test]
    async fn simulated_synthesizer_generates_hypotheses() {
        let synth = SimulatedSynthesizer::new();
        let result = synth
            .synthesize(&test_intent(), &HardwareContext::simulated())
            .await
            .unwrap();
        assert_eq!(result.hypotheses.len(), 3);
        assert_eq!(result.model_id, "simulated-synthesizer-v1");
    }

    #[tokio::test]
    async fn hypotheses_sorted_by_composite() {
        let synth = SimulatedSynthesizer::new();
        let result = synth
            .synthesize(&test_intent(), &HardwareContext::simulated())
            .await
            .unwrap();
        for w in result.hypotheses.windows(2) {
            assert!(w[0].composite_score() >= w[1].composite_score());
        }
    }

    #[tokio::test]
    async fn custom_hypothesis_count() {
        let synth = SimulatedSynthesizer::new().with_hypothesis_count(5);
        let result = synth
            .synthesize(&test_intent(), &HardwareContext::simulated())
            .await
            .unwrap();
        assert_eq!(result.hypotheses.len(), 5);
    }

    #[tokio::test]
    async fn failing_synthesizer_returns_error() {
        let synth = FailingSynthesizer;
        let result = synth
            .synthesize(&test_intent(), &HardwareContext::simulated())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn governance_tier_propagated() {
        let synth = SimulatedSynthesizer::new();
        let intent = IntentNode::new(EventId::new(), "test", GovernanceTier::Tier3);
        let result = synth
            .synthesize(&intent, &HardwareContext::simulated())
            .await
            .unwrap();
        for h in &result.hypotheses {
            assert_eq!(h.governance_tier, GovernanceTier::Tier3);
        }
    }
}
