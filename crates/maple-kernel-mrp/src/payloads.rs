use maple_mwl_types::{CommitmentId, CommitmentScope, ConfidenceProfile, EventId, WorldlineId};
use serde::{Deserialize, Serialize};

/// Meaning payload — interpretation, beliefs, uncertainty.
/// Non-executable. Lives entirely within cognition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeaningPayload {
    pub interpretation: String,
    pub confidence: f64,
    pub ambiguity_preserved: bool,
    pub evidence_refs: Vec<EventId>,
}

/// Intent payload — goals, plans, constraints.
/// Non-executable. May be shared for negotiation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentPayload {
    pub direction: String,
    pub confidence: ConfidenceProfile,
    pub conditions: Vec<String>,
    pub derived_from: Option<EventId>,
}

/// Commitment payload — explicit obligation.
/// The ONLY executable type (after Gate approval).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentPayload {
    pub commitment_id: CommitmentId,
    pub scope: CommitmentScope,
    pub affected_parties: Vec<WorldlineId>,
    pub evidence: Vec<String>,
}

/// Consequence payload — observable outcome.
/// Emitted ONLY by the execution layer, never by agents.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsequencePayload {
    pub commitment_id: CommitmentId,
    pub outcome_description: String,
    pub state_changes: serde_json::Value,
    pub observed_by: WorldlineId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_mwl_types::IdentityMaterial;

    #[test]
    fn meaning_payload_serialization() {
        let p = MeaningPayload {
            interpretation: "test".into(),
            confidence: 0.9,
            ambiguity_preserved: true,
            evidence_refs: vec![EventId::new()],
        };
        let json = serde_json::to_string(&p).unwrap();
        let restored: MeaningPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.interpretation, "test");
    }

    #[test]
    fn intent_payload_serialization() {
        let p = IntentPayload {
            direction: "send message".into(),
            confidence: ConfidenceProfile::new(0.8, 0.8, 0.8, 0.8),
            conditions: vec!["user confirmed".into()],
            derived_from: Some(EventId::new()),
        };
        let json = serde_json::to_string(&p).unwrap();
        let restored: IntentPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.direction, "send message");
    }

    #[test]
    fn commitment_payload_serialization() {
        let wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]));
        let p = CommitmentPayload {
            commitment_id: CommitmentId::new(),
            scope: CommitmentScope {
                effect_domain: maple_mwl_types::EffectDomain::Communication,
                targets: vec![wid],
                constraints: vec![],
            },
            affected_parties: vec![],
            evidence: vec!["approved by user".into()],
        };
        let json = serde_json::to_string(&p).unwrap();
        let _: CommitmentPayload = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn consequence_payload_serialization() {
        let wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]));
        let p = ConsequencePayload {
            commitment_id: CommitmentId::new(),
            outcome_description: "message sent".into(),
            state_changes: serde_json::json!({"status": "delivered"}),
            observed_by: wid,
        };
        let json = serde_json::to_string(&p).unwrap();
        let restored: ConsequencePayload = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.outcome_description, "message sent");
    }
}
