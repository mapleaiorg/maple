use maple_waf_context_graph::{ContentHash, GovernanceTier, SubstrateType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A synthesis hypothesis — a candidate code change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hypothesis {
    /// Unique ID for this hypothesis.
    pub id: String,
    /// Description of the proposed change.
    pub description: String,
    /// Target substrate.
    pub substrate: SubstrateType,
    /// Synthesized code/diff.
    pub code: String,
    /// Confidence score [0.0, 1.0].
    pub confidence: f64,
    /// Expected impact on target metrics.
    pub expected_impact: HashMap<String, f64>,
    /// Safety score [0.0, 1.0] — higher is safer.
    pub safety_score: f64,
    /// Governance tier required for this change.
    pub governance_tier: GovernanceTier,
}

impl Hypothesis {
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        substrate: SubstrateType,
        code: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            substrate,
            code: code.into(),
            confidence: 0.5,
            expected_impact: HashMap::new(),
            safety_score: 1.0,
            governance_tier: GovernanceTier::Tier0,
        }
    }

    pub fn with_confidence(mut self, c: f64) -> Self {
        self.confidence = c.clamp(0.0, 1.0);
        self
    }

    pub fn with_safety_score(mut self, s: f64) -> Self {
        self.safety_score = s.clamp(0.0, 1.0);
        self
    }

    pub fn with_impact(mut self, metric: impl Into<String>, value: f64) -> Self {
        self.expected_impact.insert(metric.into(), value);
        self
    }

    pub fn with_governance_tier(mut self, tier: GovernanceTier) -> Self {
        self.governance_tier = tier;
        self
    }

    /// Composite score: safety-weighted confidence.
    pub fn composite_score(&self) -> f64 {
        self.confidence * self.safety_score
    }
}

/// Result of a synthesis operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynthesisResult {
    /// Intent hash that triggered synthesis.
    pub intent_hash: ContentHash,
    /// Generated hypotheses, ordered by composite score (best first).
    pub hypotheses: Vec<Hypothesis>,
    /// Model that generated the hypotheses.
    pub model_id: String,
    /// Total synthesis time in milliseconds.
    pub synthesis_time_ms: u64,
}

/// Hardware context for synthesis.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HardwareContext {
    pub cpu_cores: usize,
    pub memory_mb: usize,
    pub gpu_available: bool,
    pub gpu_name: Option<String>,
    pub gpu_memory_mb: Option<usize>,
}

impl HardwareContext {
    pub fn simulated() -> Self {
        Self {
            cpu_cores: 8,
            memory_mb: 16384,
            gpu_available: false,
            gpu_name: None,
            gpu_memory_mb: None,
        }
    }

    pub fn with_gpu(mut self, name: impl Into<String>, memory_mb: usize) -> Self {
        self.gpu_available = true;
        self.gpu_name = Some(name.into());
        self.gpu_memory_mb = Some(memory_mb);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hypothesis_builder() {
        let h = Hypothesis::new(
            "h1",
            "optimize allocator",
            SubstrateType::Rust,
            "fn alloc() {}",
        )
        .with_confidence(0.8)
        .with_safety_score(0.9)
        .with_impact("latency_ms", -20.0)
        .with_governance_tier(GovernanceTier::Tier1);
        assert_eq!(h.confidence, 0.8);
        assert_eq!(h.safety_score, 0.9);
        assert!((h.composite_score() - 0.72).abs() < 0.001);
    }

    #[test]
    fn hypothesis_confidence_clamped() {
        let h = Hypothesis::new("h", "d", SubstrateType::Rust, "c").with_confidence(1.5);
        assert_eq!(h.confidence, 1.0);
    }

    #[test]
    fn hardware_context_simulated() {
        let ctx = HardwareContext::simulated();
        assert_eq!(ctx.cpu_cores, 8);
        assert!(!ctx.gpu_available);
    }

    #[test]
    fn hardware_context_with_gpu() {
        let ctx = HardwareContext::simulated().with_gpu("RTX 4090", 24576);
        assert!(ctx.gpu_available);
        assert_eq!(ctx.gpu_name.unwrap(), "RTX 4090");
    }

    #[test]
    fn hypothesis_serde() {
        let h = Hypothesis::new("h1", "test", SubstrateType::Wasm, "code");
        let json = serde_json::to_string(&h).unwrap();
        let restored: Hypothesis = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "h1");
    }

    #[test]
    fn synthesis_result_serde() {
        let r = SynthesisResult {
            intent_hash: ContentHash::hash(b"intent"),
            hypotheses: vec![],
            model_id: "test".into(),
            synthesis_time_ms: 100,
        };
        let json = serde_json::to_string(&r).unwrap();
        let restored: SynthesisResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.model_id, "test");
    }
}
