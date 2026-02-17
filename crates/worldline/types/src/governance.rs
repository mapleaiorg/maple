use serde::{Deserialize, Serialize};

use crate::commitment::{EffectDomain, TemporalBounds};
use crate::confidence::RiskClass;
use crate::confidence::RiskLevel;
use crate::temporal::TemporalAnchor;
use crate::worldline_id::WorldlineId;

/// Policy Decision Card — output of commitment adjudication by AAS.
/// Per Whitepaper §6.5: "This decision is authoritative, versioned, and permanently recorded."
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyDecisionCard {
    /// Unique decision identifier
    pub decision_id: String,
    /// Approve or Deny
    pub decision: AdjudicationDecision,
    /// Human-readable rationale
    pub rationale: String,
    /// Risk assessment
    pub risk: RiskLevel,
    /// Conditions that must hold for approval to remain valid
    pub conditions: Vec<String>,
    /// Policy rules that were evaluated
    pub policy_refs: Vec<String>,
    /// When this decision was made
    pub decided_at: TemporalAnchor,
    /// Version for auditability
    pub version: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdjudicationDecision {
    Approve,
    Deny,
    RequireHumanReview,
    RequireCoSignature,
}

/// Capability — bounded authority grant.
/// Per Whitepaper §6.3: "A capability defines the maximum scope of Commitments
/// an identity may attempt."
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub name: String,
    pub effect_domain: EffectDomain,
    pub scope: CapabilityScope,
    pub temporal_validity: TemporalBounds,
    pub risk_class: RiskClass,
    pub issuer: WorldlineId,
    pub revocation_conditions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityScope {
    pub max_targets: Option<u32>,
    pub allowed_targets: Option<Vec<WorldlineId>>,
    pub max_consequence_value: Option<f64>,
    pub constraints: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjudication_decision_serialization() {
        let decisions = [
            AdjudicationDecision::Approve,
            AdjudicationDecision::Deny,
            AdjudicationDecision::RequireHumanReview,
            AdjudicationDecision::RequireCoSignature,
        ];
        for d in &decisions {
            let json = serde_json::to_string(d).unwrap();
            let restored: AdjudicationDecision = serde_json::from_str(&json).unwrap();
            assert_eq!(*d, restored);
        }
    }

    #[test]
    fn policy_decision_card_serialization() {
        let card = PolicyDecisionCard {
            decision_id: "PDC-001".into(),
            decision: AdjudicationDecision::Approve,
            rationale: "Low risk, within capability bounds".into(),
            risk: RiskLevel {
                class: RiskClass::Low,
                score: Some(0.1),
                factors: vec![],
            },
            conditions: vec!["must complete within 1h".into()],
            policy_refs: vec!["POL-COMM-001".into()],
            decided_at: TemporalAnchor::now(1),
            version: 1,
        };
        let json = serde_json::to_string(&card).unwrap();
        let restored: PolicyDecisionCard = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.decision_id, "PDC-001");
        assert_eq!(restored.decision, AdjudicationDecision::Approve);
    }

    #[test]
    fn capability_scope_serialization() {
        let scope = CapabilityScope {
            max_targets: Some(10),
            allowed_targets: None,
            max_consequence_value: Some(1000.0),
            constraints: vec!["read-only".into()],
        };
        let json = serde_json::to_string(&scope).unwrap();
        let restored: CapabilityScope = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.max_targets, Some(10));
    }
}
