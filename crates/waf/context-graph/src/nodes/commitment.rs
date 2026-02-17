use crate::types::ContentHash;
use serde::{Deserialize, Serialize};
use worldline_types::TemporalAnchor;

/// Records the commitment decision for an evolution step.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentNode {
    /// Hash of the artifact being committed.
    pub artifact_hash: ContentHash,
    /// Hash of the evidence bundle that justified the commitment.
    pub evidence_hash: ContentHash,
    /// Equivalence tier that was verified.
    pub equivalence_tier: String,
    /// Summary of shadow execution results.
    pub shadow_summary: String,
    /// When the swap was executed.
    pub swap_timestamp: TemporalAnchor,
    /// Whether the swap was successful.
    pub swap_successful: bool,
}

impl CommitmentNode {
    pub fn new(artifact_hash: ContentHash, evidence_hash: ContentHash) -> Self {
        Self {
            artifact_hash,
            evidence_hash,
            equivalence_tier: "E0".into(),
            shadow_summary: String::new(),
            swap_timestamp: TemporalAnchor::now(0),
            swap_successful: false,
        }
    }

    pub fn with_equivalence_tier(mut self, tier: impl Into<String>) -> Self {
        self.equivalence_tier = tier.into();
        self
    }

    pub fn mark_successful(mut self) -> Self {
        self.swap_successful = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commitment_node_builder() {
        let node = CommitmentNode::new(
            ContentHash::hash(b"artifact"),
            ContentHash::hash(b"evidence"),
        )
        .with_equivalence_tier("E1")
        .mark_successful();

        assert!(node.swap_successful);
        assert_eq!(node.equivalence_tier, "E1");
    }

    #[test]
    fn commitment_node_serde() {
        let node = CommitmentNode::new(ContentHash::hash(b"a"), ContentHash::hash(b"e"));
        let json = serde_json::to_string(&node).unwrap();
        let restored: CommitmentNode = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.artifact_hash, node.artifact_hash);
    }
}
