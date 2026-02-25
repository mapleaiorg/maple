use maple_mwl_types::{
    CapabilityId, CommitmentId, CommitmentScope, ConfidenceProfile, EventId, Reversibility,
    TemporalAnchor, TemporalBounds, WorldlineId,
};
use serde::{Deserialize, Serialize};

/// A Commitment Declaration — what an agent submits to the Gate.
///
/// Per Whitepaper §2.8, a valid Commitment must specify:
/// - the acting identity
/// - the intended effect domain
/// - scope and targets
/// - temporal bounds
/// - required capabilities
/// - evidence and audit requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentDeclaration {
    pub id: CommitmentId,
    /// Who is declaring this commitment
    pub declaring_identity: WorldlineId,
    /// What intent this commitment derives from (I.3: intent reference required)
    pub derived_from_intent: Option<EventId>,
    /// The confidence profile of the underlying intent
    pub confidence: ConfidenceProfile,
    /// What is being committed to
    pub scope: CommitmentScope,
    /// When this commitment is valid
    pub temporal_bounds: TemporalBounds,
    /// How reversible this commitment is
    pub reversibility: Reversibility,
    /// Capabilities being invoked
    pub capability_refs: Vec<CapabilityId>,
    /// Parties affected or involved
    pub affected_parties: Vec<WorldlineId>,
    /// Additional evidence or context
    pub evidence: Vec<String>,
    /// When declared
    pub declared_at: TemporalAnchor,
}

impl CommitmentDeclaration {
    /// Create a builder for ergonomic construction.
    pub fn builder(
        declaring_identity: WorldlineId,
        scope: CommitmentScope,
    ) -> CommitmentDeclarationBuilder {
        CommitmentDeclarationBuilder {
            declaring_identity,
            scope,
            derived_from_intent: None,
            confidence: ConfidenceProfile::new(0.8, 0.8, 0.8, 0.8),
            temporal_bounds: TemporalBounds {
                starts: TemporalAnchor::now(0),
                expires: None,
                review_at: None,
            },
            reversibility: Reversibility::FullyReversible,
            capability_refs: Vec::new(),
            affected_parties: Vec::new(),
            evidence: Vec::new(),
        }
    }
}

/// Builder for CommitmentDeclaration.
pub struct CommitmentDeclarationBuilder {
    declaring_identity: WorldlineId,
    scope: CommitmentScope,
    derived_from_intent: Option<EventId>,
    confidence: ConfidenceProfile,
    temporal_bounds: TemporalBounds,
    reversibility: Reversibility,
    capability_refs: Vec<CapabilityId>,
    affected_parties: Vec<WorldlineId>,
    evidence: Vec<String>,
}

impl CommitmentDeclarationBuilder {
    pub fn derived_from_intent(mut self, intent: EventId) -> Self {
        self.derived_from_intent = Some(intent);
        self
    }

    pub fn confidence(mut self, confidence: ConfidenceProfile) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn temporal_bounds(mut self, bounds: TemporalBounds) -> Self {
        self.temporal_bounds = bounds;
        self
    }

    pub fn reversibility(mut self, reversibility: Reversibility) -> Self {
        self.reversibility = reversibility;
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

    pub fn affected_party(mut self, party: WorldlineId) -> Self {
        self.affected_parties.push(party);
        self
    }

    pub fn evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence.push(evidence.into());
        self
    }

    pub fn build(self) -> CommitmentDeclaration {
        CommitmentDeclaration {
            id: CommitmentId::new(),
            declaring_identity: self.declaring_identity,
            derived_from_intent: self.derived_from_intent,
            confidence: self.confidence,
            scope: self.scope,
            temporal_bounds: self.temporal_bounds,
            reversibility: self.reversibility,
            capability_refs: self.capability_refs,
            affected_parties: self.affected_parties,
            evidence: self.evidence,
            declared_at: TemporalAnchor::now(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_mwl_types::{EffectDomain, IdentityMaterial};

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn test_scope() -> CommitmentScope {
        CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![test_worldline()],
            constraints: vec!["max_100_messages".into()],
        }
    }

    #[test]
    fn builder_creates_valid_declaration() {
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope())
            .derived_from_intent(EventId::new())
            .capability(CapabilityId("CAP-COMM".into()))
            .evidence("test evidence")
            .build();

        assert!(decl.derived_from_intent.is_some());
        assert_eq!(decl.capability_refs.len(), 1);
        assert_eq!(decl.evidence.len(), 1);
    }

    #[test]
    fn declaration_serialization_roundtrip() {
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope())
            .derived_from_intent(EventId::new())
            .build();

        let json = serde_json::to_string(&decl).unwrap();
        let restored: CommitmentDeclaration = serde_json::from_str(&json).unwrap();
        assert_eq!(decl.id, restored.id);
    }

    #[test]
    fn builder_without_intent_creates_none() {
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        assert!(decl.derived_from_intent.is_none());
    }
}
