use std::collections::HashMap;

use maple_mwl_types::{
    AdjudicationDecision, Capability, CapabilityId, CapabilityScope, EffectDomain,
    PolicyDecisionCard, RiskClass, RiskLevel, TemporalAnchor, TemporalBounds, WorldlineId,
};

use crate::declaration::CommitmentDeclaration;
use crate::traits::{CapabilityProvider, PolicyProvider};

/// Mock capability provider for testing.
///
/// Stores a map of WorldlineId → Vec<Capability>.
pub struct MockCapabilityProvider {
    capabilities: HashMap<WorldlineId, Vec<Capability>>,
}

impl MockCapabilityProvider {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }

    /// Grant a capability to a worldline.
    pub fn grant(
        &mut self,
        wid: WorldlineId,
        cap_id: impl Into<String>,
        domain: EffectDomain,
    ) {
        let cap = Capability {
            id: cap_id.into(),
            name: "mock-capability".into(),
            effect_domain: domain,
            scope: CapabilityScope {
                max_targets: None,
                allowed_targets: None,
                max_consequence_value: None,
                constraints: vec![],
            },
            temporal_validity: TemporalBounds {
                starts: TemporalAnchor::genesis(),
                expires: None,
                review_at: None,
            },
            risk_class: RiskClass::Low,
            issuer: wid.clone(),
            revocation_conditions: vec![],
        };

        self.capabilities.entry(wid).or_default().push(cap);
    }
}

impl Default for MockCapabilityProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityProvider for MockCapabilityProvider {
    fn has_capability(&self, wid: &WorldlineId, cap: &CapabilityId) -> bool {
        self.capabilities
            .get(wid)
            .map(|caps| caps.iter().any(|c| c.id == cap.0))
            .unwrap_or(false)
    }

    fn get_capabilities(&self, wid: &WorldlineId) -> Vec<Capability> {
        self.capabilities.get(wid).cloned().unwrap_or_default()
    }
}

/// Mock policy provider for testing.
///
/// Can be configured to approve or deny all commitments.
pub struct MockPolicyProvider {
    default_decision: AdjudicationDecision,
    rationale: String,
}

impl MockPolicyProvider {
    /// Create a provider that approves everything.
    pub fn approve_all() -> Self {
        Self {
            default_decision: AdjudicationDecision::Approve,
            rationale: "Mock policy: approved".into(),
        }
    }

    /// Create a provider that denies everything.
    pub fn deny_all() -> Self {
        Self {
            default_decision: AdjudicationDecision::Deny,
            rationale: "Mock policy: denied".into(),
        }
    }

    /// Create a provider with a specific decision.
    pub fn with_decision(decision: AdjudicationDecision, rationale: impl Into<String>) -> Self {
        Self {
            default_decision: decision,
            rationale: rationale.into(),
        }
    }
}

impl PolicyProvider for MockPolicyProvider {
    fn evaluate(&self, _declaration: &CommitmentDeclaration) -> PolicyDecisionCard {
        PolicyDecisionCard {
            decision_id: uuid::Uuid::new_v4().to_string(),
            decision: self.default_decision,
            rationale: self.rationale.clone(),
            risk: RiskLevel {
                class: RiskClass::Low,
                score: Some(0.1),
                factors: vec![],
            },
            conditions: vec![],
            policy_refs: vec!["mock-policy-v1".into()],
            decided_at: TemporalAnchor::now(0),
            version: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_mwl_types::IdentityMaterial;

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    #[test]
    fn mock_capability_provider_grants_and_checks() {
        let mut provider = MockCapabilityProvider::new();
        let wid = test_worldline();
        let cap_id = CapabilityId("CAP-COMM".into());

        assert!(!provider.has_capability(&wid, &cap_id));

        provider.grant(wid.clone(), "CAP-COMM", EffectDomain::Communication);
        assert!(provider.has_capability(&wid, &cap_id));
    }

    #[test]
    fn mock_policy_approve_all() {
        let provider = MockPolicyProvider::approve_all();
        let scope = maple_mwl_types::CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![],
            constraints: vec![],
        };
        let decl =
            crate::declaration::CommitmentDeclaration::builder(test_worldline(), scope).build();
        let card = provider.evaluate(&decl);
        assert_eq!(card.decision, AdjudicationDecision::Approve);
    }

    #[test]
    fn mock_policy_deny_all() {
        let provider = MockPolicyProvider::deny_all();
        let scope = maple_mwl_types::CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![],
            constraints: vec![],
        };
        let decl =
            crate::declaration::CommitmentDeclaration::builder(test_worldline(), scope).build();
        let card = provider.evaluate(&decl);
        assert_eq!(card.decision, AdjudicationDecision::Deny);
    }
}
