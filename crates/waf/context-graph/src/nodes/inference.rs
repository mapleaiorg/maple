use crate::types::ContentHash;
use serde::{Deserialize, Serialize};

/// Captures the reasoning trace — which model reasoned, what alternatives
/// were considered and rejected, and what confidence the system has.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferenceNode {
    /// ID of the model/agent that produced this reasoning.
    pub model_id: String,
    /// Structured rationale (NOT raw chain-of-thought — see privacy notes).
    pub reasoning: StructuredRationale,
    /// Alternative hypotheses that were considered and rejected.
    pub alternatives: Vec<RejectedAlternative>,
    /// Confidence score [0.0, 1.0].
    pub confidence_score: f64,
    /// Evidence references supporting this inference.
    pub supporting_evidence: Vec<ContentHash>,
}

/// Structured rationale — the safe, storable form of reasoning.
/// Raw chain-of-thought MUST NOT be stored in the ledger.
/// Full traces go to a secured vault referenced by hash.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructuredRationale {
    /// Key decision points in the reasoning.
    pub decision_points: Vec<DecisionPoint>,
    /// Cited evidence references.
    pub cited_evidence: Vec<ContentHash>,
    /// Hash pointer to full trace in secured vault (optional).
    pub vault_trace_hash: Option<ContentHash>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionPoint {
    pub question: String,
    pub conclusion: String,
    pub confidence: f64,
    pub evidence_ids: Vec<ContentHash>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RejectedAlternative {
    pub description: String,
    pub rejection_reason: String,
    pub estimated_cost: f64,
}

impl InferenceNode {
    pub fn new(model_id: impl Into<String>, confidence: f64) -> Self {
        Self {
            model_id: model_id.into(),
            reasoning: StructuredRationale {
                decision_points: Vec::new(),
                cited_evidence: Vec::new(),
                vault_trace_hash: None,
            },
            alternatives: Vec::new(),
            confidence_score: confidence.clamp(0.0, 1.0),
            supporting_evidence: Vec::new(),
        }
    }

    pub fn alternatives_count(&self) -> usize {
        self.alternatives.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inference_node_builder() {
        let node = InferenceNode::new("llama3.2", 0.85);
        assert_eq!(node.model_id, "llama3.2");
        assert_eq!(node.confidence_score, 0.85);
        assert_eq!(node.alternatives_count(), 0);
    }

    #[test]
    fn confidence_clamped() {
        let node = InferenceNode::new("test", 1.5);
        assert_eq!(node.confidence_score, 1.0);
        let node2 = InferenceNode::new("test", -0.5);
        assert_eq!(node2.confidence_score, 0.0);
    }

    #[test]
    fn inference_serde_roundtrip() {
        let mut node = InferenceNode::new("model", 0.7);
        node.alternatives.push(RejectedAlternative {
            description: "option B".into(),
            rejection_reason: "too risky".into(),
            estimated_cost: 100.0,
        });
        let json = serde_json::to_string(&node).unwrap();
        let restored: InferenceNode = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.alternatives.len(), 1);
        assert_eq!(restored.alternatives[0].description, "option B");
    }

    #[test]
    fn decision_point_with_evidence() {
        let dp = DecisionPoint {
            question: "Which optimizer?".into(),
            conclusion: "Adam".into(),
            confidence: 0.9,
            evidence_ids: vec![ContentHash::hash(b"paper1")],
        };
        assert_eq!(dp.evidence_ids.len(), 1);
    }
}
