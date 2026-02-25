use maple_mwl_identity::ContinuityContext;
use maple_mwl_types::{CapabilityId, ResonanceType, TemporalAnchor, WorldlineId};

use crate::envelope::{
    EnvelopeHeader, IntegrityBlock, MrpEnvelope, RoutingConstraints, TypedPayload,
};
use crate::error::MrpError;
use crate::payloads::{CommitmentPayload, ConsequencePayload, IntentPayload, MeaningPayload};

/// Type-safe builder for MEANING envelopes.
///
/// Can only produce MeaningPayload. Cannot set capability_refs
/// (Meaning envelopes don't carry capabilities).
pub struct MeaningEnvelopeBuilder {
    origin: WorldlineId,
    payload: Option<MeaningPayload>,
    ttl_ms: u64,
    continuity_context: Option<ContinuityContext>,
    routing_constraints: RoutingConstraints,
}

impl MeaningEnvelopeBuilder {
    pub fn new(origin: WorldlineId) -> Self {
        Self {
            origin,
            payload: None,
            ttl_ms: 60_000,
            continuity_context: None,
            routing_constraints: RoutingConstraints::default(),
        }
    }

    pub fn payload(mut self, payload: MeaningPayload) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn ttl_ms(mut self, ttl: u64) -> Self {
        self.ttl_ms = ttl;
        self
    }

    pub fn continuity_context(mut self, ctx: ContinuityContext) -> Self {
        self.continuity_context = Some(ctx);
        self
    }

    pub fn routing_constraints(mut self, constraints: RoutingConstraints) -> Self {
        self.routing_constraints = constraints;
        self
    }

    pub fn build(self) -> Result<MrpEnvelope, MrpError> {
        let payload = self
            .payload
            .ok_or_else(|| MrpError::MissingField("payload".into()))?;

        let header = EnvelopeHeader {
            envelope_id: uuid::Uuid::new_v4(),
            resonance_type: ResonanceType::Meaning,
            schema_version: 1,
            timestamp: TemporalAnchor::now(0),
            ttl_ms: self.ttl_ms,
            origin: self.origin,
            continuity_context: self.continuity_context,
            trace_id: uuid::Uuid::new_v4(),
            routing_constraints: self.routing_constraints,
            capability_refs: vec![], // MEANING never has capabilities
        };

        let body = TypedPayload::Meaning(payload);

        let mut envelope = MrpEnvelope {
            header,
            body,
            integrity: IntegrityBlock {
                hash: [0u8; 32],
                signature: None,
            },
        };
        envelope.integrity.hash = envelope.compute_hash();

        Ok(envelope)
    }
}

/// Type-safe builder for INTENT envelopes.
///
/// Can only produce IntentPayload. Cannot set capability_refs
/// (Intent envelopes don't carry capabilities).
pub struct IntentEnvelopeBuilder {
    origin: WorldlineId,
    payload: Option<IntentPayload>,
    ttl_ms: u64,
    continuity_context: Option<ContinuityContext>,
    routing_constraints: RoutingConstraints,
}

impl IntentEnvelopeBuilder {
    pub fn new(origin: WorldlineId) -> Self {
        Self {
            origin,
            payload: None,
            ttl_ms: 60_000,
            continuity_context: None,
            routing_constraints: RoutingConstraints::default(),
        }
    }

    pub fn payload(mut self, payload: IntentPayload) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn ttl_ms(mut self, ttl: u64) -> Self {
        self.ttl_ms = ttl;
        self
    }

    pub fn continuity_context(mut self, ctx: ContinuityContext) -> Self {
        self.continuity_context = Some(ctx);
        self
    }

    pub fn routing_constraints(mut self, constraints: RoutingConstraints) -> Self {
        self.routing_constraints = constraints;
        self
    }

    pub fn build(self) -> Result<MrpEnvelope, MrpError> {
        let payload = self
            .payload
            .ok_or_else(|| MrpError::MissingField("payload".into()))?;

        let header = EnvelopeHeader {
            envelope_id: uuid::Uuid::new_v4(),
            resonance_type: ResonanceType::Intent,
            schema_version: 1,
            timestamp: TemporalAnchor::now(0),
            ttl_ms: self.ttl_ms,
            origin: self.origin,
            continuity_context: self.continuity_context,
            trace_id: uuid::Uuid::new_v4(),
            routing_constraints: self.routing_constraints,
            capability_refs: vec![], // INTENT never has capabilities
        };

        let body = TypedPayload::Intent(payload);

        let mut envelope = MrpEnvelope {
            header,
            body,
            integrity: IntegrityBlock {
                hash: [0u8; 32],
                signature: None,
            },
        };
        envelope.integrity.hash = envelope.compute_hash();

        Ok(envelope)
    }
}

/// Type-safe builder for COMMITMENT envelopes.
///
/// REQUIRES capability_refs — commitment without capabilities is invalid.
pub struct CommitmentEnvelopeBuilder {
    origin: WorldlineId,
    payload: Option<CommitmentPayload>,
    ttl_ms: u64,
    continuity_context: Option<ContinuityContext>,
    routing_constraints: RoutingConstraints,
    capability_refs: Vec<CapabilityId>,
}

impl CommitmentEnvelopeBuilder {
    pub fn new(origin: WorldlineId) -> Self {
        Self {
            origin,
            payload: None,
            ttl_ms: 300_000, // longer default TTL for commitments
            continuity_context: None,
            routing_constraints: RoutingConstraints::default(),
            capability_refs: Vec::new(),
        }
    }

    pub fn payload(mut self, payload: CommitmentPayload) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn ttl_ms(mut self, ttl: u64) -> Self {
        self.ttl_ms = ttl;
        self
    }

    pub fn continuity_context(mut self, ctx: ContinuityContext) -> Self {
        self.continuity_context = Some(ctx);
        self
    }

    pub fn routing_constraints(mut self, constraints: RoutingConstraints) -> Self {
        self.routing_constraints = constraints;
        self
    }

    pub fn capability(mut self, cap: CapabilityId) -> Self {
        self.capability_refs.push(cap);
        self
    }

    pub fn capabilities(mut self, caps: Vec<CapabilityId>) -> Self {
        self.capability_refs = caps;
        self
    }

    pub fn build(self) -> Result<MrpEnvelope, MrpError> {
        let payload = self
            .payload
            .ok_or_else(|| MrpError::MissingField("payload".into()))?;

        let header = EnvelopeHeader {
            envelope_id: uuid::Uuid::new_v4(),
            resonance_type: ResonanceType::Commitment,
            schema_version: 1,
            timestamp: TemporalAnchor::now(0),
            ttl_ms: self.ttl_ms,
            origin: self.origin,
            continuity_context: self.continuity_context,
            trace_id: uuid::Uuid::new_v4(),
            routing_constraints: self.routing_constraints,
            capability_refs: self.capability_refs,
        };

        let body = TypedPayload::Commitment(payload);

        let mut envelope = MrpEnvelope {
            header,
            body,
            integrity: IntegrityBlock {
                hash: [0u8; 32],
                signature: None,
            },
        };
        envelope.integrity.hash = envelope.compute_hash();

        Ok(envelope)
    }
}

/// Type-safe builder for CONSEQUENCE envelopes.
///
/// Should only be used by the execution layer.
/// Consequence envelopes carry observed outcomes.
pub struct ConsequenceEnvelopeBuilder {
    origin: WorldlineId,
    payload: Option<ConsequencePayload>,
    ttl_ms: u64,
    routing_constraints: RoutingConstraints,
}

impl ConsequenceEnvelopeBuilder {
    pub fn new(origin: WorldlineId) -> Self {
        Self {
            origin,
            payload: None,
            ttl_ms: 60_000,
            routing_constraints: RoutingConstraints::default(),
        }
    }

    pub fn payload(mut self, payload: ConsequencePayload) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn ttl_ms(mut self, ttl: u64) -> Self {
        self.ttl_ms = ttl;
        self
    }

    pub fn routing_constraints(mut self, constraints: RoutingConstraints) -> Self {
        self.routing_constraints = constraints;
        self
    }

    pub fn build(self) -> Result<MrpEnvelope, MrpError> {
        let payload = self
            .payload
            .ok_or_else(|| MrpError::MissingField("payload".into()))?;

        let header = EnvelopeHeader {
            envelope_id: uuid::Uuid::new_v4(),
            resonance_type: ResonanceType::Consequence,
            schema_version: 1,
            timestamp: TemporalAnchor::now(0),
            ttl_ms: self.ttl_ms,
            origin: self.origin,
            continuity_context: None,
            trace_id: uuid::Uuid::new_v4(),
            routing_constraints: self.routing_constraints,
            capability_refs: vec![], // CONSEQUENCE never has capabilities
        };

        let body = TypedPayload::Consequence(payload);

        let mut envelope = MrpEnvelope {
            header,
            body,
            integrity: IntegrityBlock {
                hash: [0u8; 32],
                signature: None,
            },
        };
        envelope.integrity.hash = envelope.compute_hash();

        Ok(envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_mwl_types::{
        CommitmentId, CommitmentScope, ConfidenceProfile, EffectDomain, IdentityMaterial,
    };

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    #[test]
    fn meaning_builder_produces_correct_type() {
        let env = MeaningEnvelopeBuilder::new(test_worldline())
            .payload(MeaningPayload {
                interpretation: "hello".into(),
                confidence: 0.9,
                ambiguity_preserved: true,
                evidence_refs: vec![],
            })
            .build()
            .unwrap();

        assert_eq!(env.header.resonance_type, ResonanceType::Meaning);
        assert!(env.is_type_consistent());
        assert!(env.verify_integrity());
        assert!(env.header.capability_refs.is_empty());
    }

    #[test]
    fn intent_builder_produces_correct_type() {
        let env = IntentEnvelopeBuilder::new(test_worldline())
            .payload(IntentPayload {
                direction: "send message".into(),
                confidence: ConfidenceProfile::new(0.8, 0.8, 0.8, 0.8),
                conditions: vec![],
                derived_from: None,
            })
            .build()
            .unwrap();

        assert_eq!(env.header.resonance_type, ResonanceType::Intent);
        assert!(env.is_type_consistent());
        assert!(env.verify_integrity());
    }

    #[test]
    fn commitment_builder_produces_correct_type() {
        let env = CommitmentEnvelopeBuilder::new(test_worldline())
            .payload(CommitmentPayload {
                commitment_id: CommitmentId::new(),
                scope: CommitmentScope {
                    effect_domain: EffectDomain::Communication,
                    targets: vec![test_worldline()],
                    constraints: vec![],
                },
                affected_parties: vec![],
                evidence: vec![],
            })
            .capability(CapabilityId("CAP-COMM".into()))
            .build()
            .unwrap();

        assert_eq!(env.header.resonance_type, ResonanceType::Commitment);
        assert!(env.is_type_consistent());
        assert!(env.verify_integrity());
        assert_eq!(env.header.capability_refs.len(), 1);
    }

    #[test]
    fn consequence_builder_produces_correct_type() {
        let env = ConsequenceEnvelopeBuilder::new(test_worldline())
            .payload(ConsequencePayload {
                commitment_id: CommitmentId::new(),
                outcome_description: "done".into(),
                state_changes: serde_json::json!({}),
                observed_by: test_worldline(),
            })
            .build()
            .unwrap();

        assert_eq!(env.header.resonance_type, ResonanceType::Consequence);
        assert!(env.is_type_consistent());
        assert!(env.verify_integrity());
    }

    #[test]
    fn builder_fails_without_payload() {
        let result = MeaningEnvelopeBuilder::new(test_worldline()).build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_sets_custom_ttl() {
        let env = MeaningEnvelopeBuilder::new(test_worldline())
            .payload(MeaningPayload {
                interpretation: "test".into(),
                confidence: 0.5,
                ambiguity_preserved: false,
                evidence_refs: vec![],
            })
            .ttl_ms(1000)
            .build()
            .unwrap();

        assert_eq!(env.header.ttl_ms, 1000);
    }
}
