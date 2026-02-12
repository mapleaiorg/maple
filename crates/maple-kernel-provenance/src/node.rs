use maple_kernel_fabric::ResonanceStage;
use maple_mwl_types::{EventId, TemporalAnchor, WorldlineId};
use serde::{Deserialize, Serialize};

use crate::checkpoint::CheckpointRef;

/// A node in the Provenance DAG.
///
/// Each node corresponds to a KernelEvent and tracks its causal relationships.
/// Parents are direct causal ancestors; children are maintained lazily as new
/// events are added.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProvenanceNode {
    /// The event this node represents
    pub event_id: EventId,
    /// Direct causal parents (from KernelEvent.parents)
    pub parents: Vec<EventId>,
    /// Direct causal children (computed lazily as events are added)
    pub children: Vec<EventId>,
    /// Which worldline produced this event
    pub worldline: WorldlineId,
    /// Resonance stage classification
    pub resonance_stage: ResonanceStage,
    /// When this event occurred
    pub timestamp: TemporalAnchor,
    /// If this node was compressed into a checkpoint
    pub checkpoint: Option<CheckpointRef>,
    /// Optional commitment_id extracted from payload (for audit_trail queries)
    pub commitment_id: Option<maple_mwl_types::CommitmentId>,
    /// Optional policy_id extracted from payload (for regulatory_slice queries)
    pub policy_id: Option<String>,
}

impl ProvenanceNode {
    /// Check if this is a genesis event (no parents).
    pub fn is_genesis(&self) -> bool {
        self.parents.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_mwl_types::IdentityMaterial;

    #[test]
    fn genesis_node_has_no_parents() {
        let node = ProvenanceNode {
            event_id: EventId::new(),
            parents: vec![],
            children: vec![],
            worldline: WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32])),
            resonance_stage: ResonanceStage::System,
            timestamp: TemporalAnchor::now(0),
            checkpoint: None,
            commitment_id: None,
            policy_id: None,
        };
        assert!(node.is_genesis());
    }

    #[test]
    fn non_genesis_node_has_parents() {
        let node = ProvenanceNode {
            event_id: EventId::new(),
            parents: vec![EventId::new()],
            children: vec![],
            worldline: WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32])),
            resonance_stage: ResonanceStage::Meaning,
            timestamp: TemporalAnchor::now(0),
            checkpoint: None,
            commitment_id: None,
            policy_id: None,
        };
        assert!(!node.is_genesis());
    }
}
