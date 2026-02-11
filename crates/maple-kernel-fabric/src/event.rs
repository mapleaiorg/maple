use serde::{Deserialize, Serialize};

use crate::hlc::HlcTimestamp;
use crate::types::{CommitmentId, CouplingId, CouplingScope, EventId, Hash, WorldlineId};

/// Resonance stage classification for events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResonanceStage {
    Presence,
    Coupling,
    Meaning,
    Intent,
    Commitment,
    Consequence,
    /// Policy/invariant events
    Governance,
    /// Internal kernel events
    System,
}

/// A kernel event — the atomic unit of state change in MWL.
///
/// Events are immutable once created. Every event has an integrity hash (BLAKE3)
/// and declares its causal parents.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KernelEvent {
    /// Unique event identifier
    pub id: EventId,
    /// HLC timestamp — causal ordering
    pub timestamp: HlcTimestamp,
    /// Which worldline (resonator) produced this event
    pub worldline_id: WorldlineId,
    /// Resonance stage classification
    pub stage: ResonanceStage,
    /// Event-specific payload
    pub payload: EventPayload,
    /// Causal parents — events that causally precede this one
    pub parents: Vec<EventId>,
    /// BLAKE3 hash of (id + timestamp + worldline_id + stage + payload + parents)
    pub integrity_hash: Hash,
}

impl KernelEvent {
    /// Create a new event and compute its integrity hash.
    pub fn new(
        id: EventId,
        timestamp: HlcTimestamp,
        worldline_id: WorldlineId,
        stage: ResonanceStage,
        payload: EventPayload,
        parents: Vec<EventId>,
    ) -> Self {
        let integrity_hash = Self::compute_hash(&id, &timestamp, &worldline_id, &stage, &payload, &parents);
        Self {
            id,
            timestamp,
            worldline_id,
            stage,
            payload,
            parents,
            integrity_hash,
        }
    }

    /// Verify the integrity hash of this event.
    pub fn verify_integrity(&self) -> bool {
        let expected = Self::compute_hash(
            &self.id,
            &self.timestamp,
            &self.worldline_id,
            &self.stage,
            &self.payload,
            &self.parents,
        );
        self.integrity_hash == expected
    }

    fn compute_hash(
        id: &EventId,
        timestamp: &HlcTimestamp,
        worldline_id: &WorldlineId,
        stage: &ResonanceStage,
        payload: &EventPayload,
        parents: &[EventId],
    ) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"kernel-event-v1:");

        // Event ID
        hasher.update(id.0.as_bytes());

        // Timestamp
        hasher.update(&timestamp.physical.to_le_bytes());
        hasher.update(&timestamp.logical.to_le_bytes());
        hasher.update(&timestamp.node_id.0.to_le_bytes());

        // WorldlineId
        hasher.update(worldline_id.identity_hash());

        // Stage (as ordinal)
        let stage_byte = match stage {
            ResonanceStage::Presence => 0u8,
            ResonanceStage::Coupling => 1,
            ResonanceStage::Meaning => 2,
            ResonanceStage::Intent => 3,
            ResonanceStage::Commitment => 4,
            ResonanceStage::Consequence => 5,
            ResonanceStage::Governance => 6,
            ResonanceStage::System => 7,
        };
        hasher.update(&[stage_byte]);

        // Payload (serialize to JSON for deterministic hashing)
        if let Ok(payload_bytes) = serde_json::to_vec(payload) {
            hasher.update(&payload_bytes);
        }

        // Parents
        let parent_count = parents.len() as u32;
        hasher.update(&parent_count.to_le_bytes());
        for parent in parents {
            hasher.update(parent.0.as_bytes());
        }

        Hash::from_bytes(*hasher.finalize().as_bytes())
    }
}

/// Event payloads typed by resonance stage.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EventPayload {
    // Presence events
    PresenceAsserted {
        discoverability: f64,
        responsiveness: f64,
    },
    PresenceWithdrawn {
        reason: String,
    },

    // Coupling events
    CouplingEstablished {
        target: WorldlineId,
        intensity: f64,
        scope: CouplingScope,
    },
    CouplingModified {
        coupling_id: CouplingId,
        new_intensity: f64,
    },
    CouplingSevered {
        coupling_id: CouplingId,
        reason: String,
    },

    // Meaning events
    MeaningFormed {
        interpretation_count: u32,
        confidence: f64,
        ambiguity_preserved: bool,
    },
    MeaningRevised {
        previous_confidence: f64,
        new_confidence: f64,
    },

    // Intent events
    IntentStabilized {
        direction: String,
        confidence: f64,
        conditions: Vec<String>,
    },
    IntentDeferred {
        reason: String,
    },
    IntentAbandoned {
        reason: String,
    },

    // Commitment events — CRITICAL: these cross the Commitment Boundary
    CommitmentDeclared {
        commitment_id: CommitmentId,
        scope: serde_json::Value,
        parties: Vec<WorldlineId>,
    },
    CommitmentApproved {
        commitment_id: CommitmentId,
        decision_card: serde_json::Value,
    },
    CommitmentDenied {
        commitment_id: CommitmentId,
        rationale: String,
    },
    CommitmentFulfilled {
        commitment_id: CommitmentId,
    },
    CommitmentFailed {
        commitment_id: CommitmentId,
        reason: String,
    },

    // Consequence events
    ConsequenceObserved {
        commitment_id: CommitmentId,
        state_changes: serde_json::Value,
    },

    // Governance events
    PolicyEvaluated {
        policy_id: String,
        result: String,
    },
    InvariantChecked {
        invariant_id: String,
        passed: bool,
    },
    CapabilityGranted {
        capability_id: String,
        worldline_id: WorldlineId,
    },
    CapabilityRevoked {
        capability_id: String,
        reason: String,
    },

    // System events
    WorldlineCreated {
        profile: String,
    },
    WorldlineDestroyed {
        reason: String,
    },
    CheckpointCreated {
        sequence_number: u64,
    },

    // Generic extension point
    Custom {
        type_name: String,
        data: serde_json::Value,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NodeId;

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&maple_mwl_types::IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn test_timestamp() -> HlcTimestamp {
        HlcTimestamp {
            physical: 1000,
            logical: 0,
            node_id: NodeId(1),
        }
    }

    #[test]
    fn event_integrity_hash_verifies() {
        let event = KernelEvent::new(
            EventId::new(),
            test_timestamp(),
            test_worldline(),
            ResonanceStage::Meaning,
            EventPayload::MeaningFormed {
                interpretation_count: 3,
                confidence: 0.85,
                ambiguity_preserved: true,
            },
            vec![],
        );
        assert!(event.verify_integrity());
    }

    #[test]
    fn event_integrity_hash_is_deterministic() {
        let id = EventId::new();
        let ts = test_timestamp();
        let wid = test_worldline();
        let payload = EventPayload::MeaningFormed {
            interpretation_count: 1,
            confidence: 0.5,
            ambiguity_preserved: false,
        };

        let e1 = KernelEvent::new(id.clone(), ts.clone(), wid.clone(), ResonanceStage::Meaning, payload.clone(), vec![]);
        let e2 = KernelEvent::new(id, ts, wid, ResonanceStage::Meaning, payload, vec![]);
        assert_eq!(e1.integrity_hash, e2.integrity_hash);
    }

    #[test]
    fn tampered_event_fails_integrity() {
        let mut event = KernelEvent::new(
            EventId::new(),
            test_timestamp(),
            test_worldline(),
            ResonanceStage::Meaning,
            EventPayload::MeaningFormed {
                interpretation_count: 1,
                confidence: 0.5,
                ambiguity_preserved: false,
            },
            vec![],
        );
        // Tamper with stage
        event.stage = ResonanceStage::Commitment;
        assert!(!event.verify_integrity());
    }

    #[test]
    fn all_payload_variants_serialize() {
        let payloads = vec![
            EventPayload::PresenceAsserted { discoverability: 0.8, responsiveness: 0.9 },
            EventPayload::PresenceWithdrawn { reason: "shutdown".into() },
            EventPayload::MeaningFormed { interpretation_count: 2, confidence: 0.7, ambiguity_preserved: true },
            EventPayload::IntentStabilized { direction: "forward".into(), confidence: 0.9, conditions: vec![] },
            EventPayload::CommitmentDeclared {
                commitment_id: CommitmentId::new(),
                scope: serde_json::json!({"type": "test"}),
                parties: vec![test_worldline()],
            },
            EventPayload::ConsequenceObserved {
                commitment_id: CommitmentId::new(),
                state_changes: serde_json::json!({}),
            },
            EventPayload::PolicyEvaluated { policy_id: "P1".into(), result: "pass".into() },
            EventPayload::WorldlineCreated { profile: "test".into() },
            EventPayload::CheckpointCreated { sequence_number: 42 },
            EventPayload::Custom { type_name: "test".into(), data: serde_json::json!(null) },
        ];

        for p in &payloads {
            let json = serde_json::to_string(p).unwrap();
            let _: EventPayload = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn event_serialization_roundtrip() {
        let event = KernelEvent::new(
            EventId::new(),
            test_timestamp(),
            test_worldline(),
            ResonanceStage::Commitment,
            EventPayload::CommitmentDeclared {
                commitment_id: CommitmentId::new(),
                scope: serde_json::json!({"domain": "financial"}),
                parties: vec![test_worldline()],
            },
            vec![EventId::new()],
        );

        let json = serde_json::to_string(&event).unwrap();
        let restored: KernelEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.id, restored.id);
        assert_eq!(event.integrity_hash, restored.integrity_hash);
        assert!(restored.verify_integrity());
    }
}
