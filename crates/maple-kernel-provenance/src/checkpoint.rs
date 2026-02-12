use maple_mwl_types::{EventId, TemporalAnchor};
use serde::{Deserialize, Serialize};

/// Reference to a checkpoint (used for compressed provenance).
///
/// Per I.PVP-1 (Compression Integrity): Checkpoint compression preserves causal linkage.
/// When events are compressed into a checkpoint, the boundary events are preserved
/// so that causal paths can still be traced across checkpoint boundaries.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckpointRef {
    /// Unique checkpoint identifier
    pub checkpoint_id: uuid::Uuid,
    /// When this checkpoint was created
    pub created_at: TemporalAnchor,
    /// Number of events compressed into this checkpoint
    pub event_count: usize,
}

/// A checkpoint — compressed range of provenance nodes.
///
/// Preserves boundary events (entry/exit points) so causal paths
/// across checkpoint boundaries remain traceable.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: uuid::Uuid,
    /// All events compressed before this temporal anchor
    pub before: TemporalAnchor,
    /// Boundary events preserved for causal linking (events that have
    /// children outside the checkpoint or are referenced by external events)
    pub boundary_events: Vec<EventId>,
    /// Number of events compressed
    pub compressed_count: usize,
    /// When checkpoint was created
    pub created_at: TemporalAnchor,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_ref_serialization() {
        let cr = CheckpointRef {
            checkpoint_id: uuid::Uuid::new_v4(),
            created_at: TemporalAnchor::now(0),
            event_count: 100,
        };
        let json = serde_json::to_string(&cr).unwrap();
        let restored: CheckpointRef = serde_json::from_str(&json).unwrap();
        assert_eq!(cr.checkpoint_id, restored.checkpoint_id);
        assert_eq!(cr.event_count, restored.event_count);
    }
}
