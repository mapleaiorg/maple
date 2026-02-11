use maple_mwl_identity::ContinuityContext;
use maple_mwl_types::{CapabilityId, ResonanceType, TemporalAnchor, WorldlineId};
use serde::{Deserialize, Serialize};

use crate::payloads::{CommitmentPayload, ConsequencePayload, IntentPayload, MeaningPayload};

/// MRP Envelope — the fundamental message unit in the system.
///
/// Per Whitepaper §4.4: Every envelope declares its semantic role explicitly.
/// The resonance_type in the header is THE authoritative classification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MrpEnvelope {
    pub header: EnvelopeHeader,
    pub body: TypedPayload,
    pub integrity: IntegrityBlock,
}

/// Envelope header — metadata and routing information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvelopeHeader {
    pub envelope_id: uuid::Uuid,
    /// THE key field — declares the semantic role of this message.
    pub resonance_type: ResonanceType,
    pub schema_version: u32,
    pub timestamp: TemporalAnchor,
    /// Time-to-live in milliseconds
    pub ttl_ms: u64,
    /// Origin WorldLine identity
    pub origin: WorldlineId,
    /// Continuity context (for identity attribution)
    pub continuity_context: Option<ContinuityContext>,
    /// Trace ID for distributed tracing
    pub trace_id: uuid::Uuid,
    /// Routing constraints
    pub routing_constraints: RoutingConstraints,
    /// Capability references (only meaningful for COMMITMENT envelopes)
    pub capability_refs: Vec<CapabilityId>,
}

/// Typed payload — content MUST match the resonance type in the header.
///
/// I.MRP-1 (Non-Escalation): The payload variant determines the actual type.
/// If header.resonance_type disagrees with the payload variant, the envelope is invalid.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TypedPayload {
    Meaning(MeaningPayload),
    Intent(IntentPayload),
    Commitment(CommitmentPayload),
    Consequence(ConsequencePayload),
}

impl TypedPayload {
    /// Get the resonance type this payload actually represents.
    pub fn resonance_type(&self) -> ResonanceType {
        match self {
            TypedPayload::Meaning(_) => ResonanceType::Meaning,
            TypedPayload::Intent(_) => ResonanceType::Intent,
            TypedPayload::Commitment(_) => ResonanceType::Commitment,
            TypedPayload::Consequence(_) => ResonanceType::Consequence,
        }
    }
}

/// Integrity verification block.
///
/// Contains a BLAKE3 hash of header + body for tamper detection,
/// and an optional Ed25519 signature for origin authentication.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrityBlock {
    /// BLAKE3 hash of serialized header + body
    pub hash: [u8; 32],
    /// Optional cryptographic signature
    pub signature: Option<Vec<u8>>,
}

/// Routing constraints for envelope delivery.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RoutingConstraints {
    /// Required destination WorldLines
    pub required_destinations: Vec<WorldlineId>,
    /// Forbidden destinations (e.g., for isolation)
    pub forbidden_destinations: Vec<WorldlineId>,
    /// Whether delivery must be guaranteed
    pub guaranteed_delivery: bool,
    /// Whether ordering must be preserved
    pub ordered: bool,
}

impl MrpEnvelope {
    /// Check if this envelope has expired based on its TTL.
    pub fn is_expired(&self) -> bool {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let created_ms = self.header.timestamp.physical_ms;
        now_ms > created_ms + self.header.ttl_ms
    }

    /// Verify that the header resonance type matches the payload variant.
    pub fn is_type_consistent(&self) -> bool {
        self.header.resonance_type == self.body.resonance_type()
    }

    /// Compute the expected integrity hash for this envelope.
    pub fn compute_hash(&self) -> [u8; 32] {
        let header_bytes = serde_json::to_vec(&self.header).unwrap_or_default();
        let body_bytes = serde_json::to_vec(&self.body).unwrap_or_default();

        let mut hasher = blake3::Hasher::new();
        hasher.update(&header_bytes);
        hasher.update(&body_bytes);
        *hasher.finalize().as_bytes()
    }

    /// Verify the integrity hash matches the computed hash.
    pub fn verify_integrity(&self) -> bool {
        let expected = self.compute_hash();
        self.integrity.hash == expected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payloads::MeaningPayload;
    use maple_mwl_types::{EventId, IdentityMaterial};

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn meaning_envelope() -> MrpEnvelope {
        let header = EnvelopeHeader {
            envelope_id: uuid::Uuid::new_v4(),
            resonance_type: ResonanceType::Meaning,
            schema_version: 1,
            timestamp: TemporalAnchor::now(0),
            ttl_ms: 60_000,
            origin: test_worldline(),
            continuity_context: None,
            trace_id: uuid::Uuid::new_v4(),
            routing_constraints: RoutingConstraints::default(),
            capability_refs: vec![],
        };
        let body = TypedPayload::Meaning(MeaningPayload {
            interpretation: "test meaning".into(),
            confidence: 0.9,
            ambiguity_preserved: true,
            evidence_refs: vec![EventId::new()],
        });
        let mut envelope = MrpEnvelope {
            header,
            body,
            integrity: IntegrityBlock {
                hash: [0u8; 32],
                signature: None,
            },
        };
        envelope.integrity.hash = envelope.compute_hash();
        envelope
    }

    #[test]
    fn type_consistency_matching() {
        let env = meaning_envelope();
        assert!(env.is_type_consistent());
    }

    #[test]
    fn type_consistency_mismatch() {
        let mut env = meaning_envelope();
        env.header.resonance_type = ResonanceType::Intent; // mismatch!
        assert!(!env.is_type_consistent());
    }

    #[test]
    fn integrity_verification_valid() {
        let env = meaning_envelope();
        assert!(env.verify_integrity());
    }

    #[test]
    fn integrity_verification_detects_tampering() {
        let mut env = meaning_envelope();
        // Tamper with the body after hash was computed
        env.body = TypedPayload::Meaning(MeaningPayload {
            interpretation: "TAMPERED".into(),
            confidence: 0.1,
            ambiguity_preserved: false,
            evidence_refs: vec![],
        });
        assert!(!env.verify_integrity());
    }

    #[test]
    fn payload_resonance_type_correct() {
        assert_eq!(
            TypedPayload::Meaning(MeaningPayload {
                interpretation: "".into(),
                confidence: 0.0,
                ambiguity_preserved: false,
                evidence_refs: vec![],
            })
            .resonance_type(),
            ResonanceType::Meaning
        );
    }

    #[test]
    fn envelope_serialization_roundtrip() {
        let env = meaning_envelope();
        let json = serde_json::to_string(&env).unwrap();
        let restored: MrpEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(env.header.envelope_id, restored.header.envelope_id);
        assert!(restored.verify_integrity());
    }

    #[test]
    fn ttl_expiration() {
        let mut env = meaning_envelope();
        // Set timestamp far in the past with short TTL
        env.header.timestamp = TemporalAnchor::new(1000, 0, 0);
        env.header.ttl_ms = 1;
        assert!(env.is_expired());
    }

    #[test]
    fn ttl_not_expired() {
        let env = meaning_envelope();
        assert!(!env.is_expired());
    }
}
