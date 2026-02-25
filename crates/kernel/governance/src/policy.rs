use maple_mwl_types::{
    AdjudicationDecision, EffectDomain, PolicyDecisionCard, PolicyId, Reversibility, RiskClass,
    RiskLevel, TemporalAnchor,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use maple_kernel_gate::CommitmentDeclaration;

use crate::error::PolicyError;

/// Policy Engine — policy-as-code evaluation.
///
/// Per Whitepaper §6.1: "AAS is the normative authority of Maple AI. It
/// decides—deterministically and audibly—whether an agent's declared Commitment
/// may be allowed to bind the world."
///
/// Policies are evaluated in priority order (highest first). Constitutional
/// policies cannot be removed (I.GCP-2).
pub struct PolicyEngine {
    policies: Vec<Policy>,
}

/// A governance policy.
///
/// Policies are evaluated against commitment declarations. Constitutional
/// policies cannot be removed even by operators.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Policy {
    pub id: PolicyId,
    pub name: String,
    pub description: String,
    pub condition: PolicyCondition,
    pub action: PolicyAction,
    pub priority: u32,
    /// If true, this policy is constitutional and cannot be removed
    /// per I.GCP-2 (Constitutional Immutability).
    pub constitutional: bool,
}

/// Condition under which a policy applies.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PolicyCondition {
    /// Always applies
    Always,
    /// Applies to specific effect domains
    DomainMatch(Vec<EffectDomain>),
    /// Applies when risk class meets or exceeds threshold
    RiskThreshold(RiskClass),
    /// Applies to irreversible commitments
    IrreversibleOnly,
    /// Applies when target count exceeds threshold
    TargetCountExceeds(usize),
    /// Compound: all conditions must match
    All(Vec<PolicyCondition>),
    /// Compound: any condition must match
    Any(Vec<PolicyCondition>),
}

/// Action a policy takes when its condition matches.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PolicyAction {
    /// Approve the commitment
    Approve,
    /// Deny with reason
    Deny(String),
    /// Require human review
    RequireHumanReview(String),
    /// Require co-signatures
    RequireCoSignature,
    /// Set a risk class on the decision
    SetRiskClass(RiskClass),
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    /// Create a PolicyEngine with the 8 constitutional invariant policies
    /// pre-loaded. These cannot be removed.
    pub fn with_constitutional_defaults() -> Self {
        let mut engine = Self::new();

        // Constitutional policy: irreversible actions in financial domain require human review
        engine
            .add_policy(Policy {
                id: PolicyId("POL-CONST-FIN-IRREVERSIBLE".into()),
                name: "Financial Irreversible Review".into(),
                description: "Irreversible financial commitments require human review".into(),
                condition: PolicyCondition::All(vec![
                    PolicyCondition::DomainMatch(vec![EffectDomain::Financial]),
                    PolicyCondition::IrreversibleOnly,
                ]),
                action: PolicyAction::RequireHumanReview(
                    "Irreversible financial commitment requires human approval".into(),
                ),
                priority: 1000,
                constitutional: true,
            })
            .unwrap();

        // Constitutional policy: high-risk commitments require co-signatures
        engine
            .add_policy(Policy {
                id: PolicyId("POL-CONST-HIGH-RISK-COSIGN".into()),
                name: "High Risk Co-Signature".into(),
                description: "High-risk commitments require co-signatures".into(),
                condition: PolicyCondition::RiskThreshold(RiskClass::High),
                action: PolicyAction::RequireCoSignature,
                priority: 900,
                constitutional: true,
            })
            .unwrap();

        // Constitutional policy: governance domain requires human review
        engine
            .add_policy(Policy {
                id: PolicyId("POL-CONST-GOV-REVIEW".into()),
                name: "Governance Domain Review".into(),
                description: "Governance changes always require human review".into(),
                condition: PolicyCondition::DomainMatch(vec![EffectDomain::Governance]),
                action: PolicyAction::RequireHumanReview(
                    "Governance domain changes require human approval".into(),
                ),
                priority: 950,
                constitutional: true,
            })
            .unwrap();

        engine
    }

    /// Add a policy to the engine.
    pub fn add_policy(&mut self, policy: Policy) -> Result<PolicyId, PolicyError> {
        // Check for duplicate
        if self.policies.iter().any(|p| p.id == policy.id) {
            return Err(PolicyError::DuplicatePolicy(policy.id));
        }

        let id = policy.id.clone();
        info!(policy_id = %id.0, name = %policy.name, "Policy added");
        self.policies.push(policy);

        // Sort by priority (highest first)
        self.policies.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(id)
    }

    /// Remove a non-constitutional policy.
    ///
    /// Per I.GCP-2 (Constitutional Immutability): Constitutional policies
    /// cannot be removed.
    pub fn remove_policy(&mut self, id: &PolicyId) -> Result<(), PolicyError> {
        let idx = self
            .policies
            .iter()
            .position(|p| p.id == *id)
            .ok_or_else(|| PolicyError::PolicyNotFound(id.clone()))?;

        if self.policies[idx].constitutional {
            return Err(PolicyError::ConstitutionalPolicyRemoval(id.clone()));
        }

        warn!(policy_id = %id.0, "Policy removed");
        self.policies.remove(idx);
        Ok(())
    }

    /// Evaluate a commitment declaration against all applicable policies.
    ///
    /// Returns a PolicyDecisionCard with the result. Policies are evaluated
    /// in priority order. The first policy whose condition matches determines
    /// the decision.
    pub fn evaluate_declaration(&self, declaration: &CommitmentDeclaration) -> PolicyDecisionCard {
        let mut matching_policies = Vec::new();
        let mut decision = AdjudicationDecision::Approve;
        let mut rationale = "All policies passed".to_string();
        let mut risk_class = RiskClass::Low;
        let mut policy_refs = Vec::new();

        for policy in &self.policies {
            if self.condition_matches(&policy.condition, declaration) {
                matching_policies.push(policy);
                policy_refs.push(policy.id.0.clone());

                debug!(
                    policy_id = %policy.id.0,
                    name = %policy.name,
                    "Policy matched"
                );

                match &policy.action {
                    PolicyAction::Deny(reason) => {
                        decision = AdjudicationDecision::Deny;
                        rationale = reason.clone();
                        break; // Deny is terminal
                    }
                    PolicyAction::RequireHumanReview(reason) => {
                        // Only escalate, never de-escalate
                        if decision != AdjudicationDecision::Deny {
                            decision = AdjudicationDecision::RequireHumanReview;
                            rationale = reason.clone();
                        }
                    }
                    PolicyAction::RequireCoSignature => {
                        if decision == AdjudicationDecision::Approve {
                            decision = AdjudicationDecision::RequireCoSignature;
                            rationale = format!("Policy {} requires co-signature", policy.name);
                        }
                    }
                    PolicyAction::SetRiskClass(class) => {
                        if *class > risk_class {
                            risk_class = *class;
                        }
                    }
                    PolicyAction::Approve => {
                        // No change needed
                    }
                }
            }
        }

        PolicyDecisionCard {
            decision_id: uuid::Uuid::new_v4().to_string(),
            decision,
            rationale,
            risk: RiskLevel {
                class: risk_class,
                score: Some(risk_class_to_score(risk_class)),
                factors: matching_policies
                    .iter()
                    .map(|p| format!("{}: {}", p.id.0, p.name))
                    .collect(),
            },
            conditions: vec![],
            policy_refs,
            decided_at: TemporalAnchor::now(0),
            version: 1,
        }
    }

    /// Check if a condition matches a declaration.
    fn condition_matches(
        &self,
        condition: &PolicyCondition,
        declaration: &CommitmentDeclaration,
    ) -> bool {
        match condition {
            PolicyCondition::Always => true,
            PolicyCondition::DomainMatch(domains) => {
                domains.contains(&declaration.scope.effect_domain)
            }
            PolicyCondition::RiskThreshold(threshold) => {
                // Infer risk class from reversibility
                let inferred = infer_risk_class(declaration);
                inferred >= *threshold
            }
            PolicyCondition::IrreversibleOnly => {
                matches!(declaration.reversibility, Reversibility::Irreversible)
            }
            PolicyCondition::TargetCountExceeds(threshold) => {
                declaration.scope.targets.len() > *threshold
            }
            PolicyCondition::All(conditions) => conditions
                .iter()
                .all(|c| self.condition_matches(c, declaration)),
            PolicyCondition::Any(conditions) => conditions
                .iter()
                .any(|c| self.condition_matches(c, declaration)),
        }
    }

    /// Get all policies.
    pub fn policies(&self) -> &[Policy] {
        &self.policies
    }

    /// Count of constitutional policies.
    pub fn constitutional_count(&self) -> usize {
        self.policies.iter().filter(|p| p.constitutional).count()
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement the Gate's PolicyProvider trait so the real PolicyEngine
/// can be used in the Commitment Gate pipeline (replacing mocks).
impl maple_kernel_gate::PolicyProvider for PolicyEngine {
    fn evaluate(&self, declaration: &CommitmentDeclaration) -> PolicyDecisionCard {
        self.evaluate_declaration(declaration)
    }
}

/// Infer a risk class from commitment declaration properties.
fn infer_risk_class(declaration: &CommitmentDeclaration) -> RiskClass {
    match &declaration.reversibility {
        Reversibility::Irreversible => RiskClass::Critical,
        Reversibility::TimeWindow { window_ms } => {
            if *window_ms < 60_000 {
                RiskClass::High
            } else {
                RiskClass::Medium
            }
        }
        Reversibility::Conditional { .. } => RiskClass::Medium,
        Reversibility::FullyReversible => RiskClass::Low,
    }
}

/// Convert risk class to a numeric score.
fn risk_class_to_score(class: RiskClass) -> f64 {
    match class {
        RiskClass::Low => 0.1,
        RiskClass::Medium => 0.4,
        RiskClass::High => 0.7,
        RiskClass::Critical => 0.95,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_mwl_types::{CommitmentScope, IdentityMaterial, WorldlineId};

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn test_scope(domain: EffectDomain) -> CommitmentScope {
        CommitmentScope {
            effect_domain: domain,
            targets: vec![test_worldline()],
            constraints: vec![],
        }
    }

    fn simple_declaration(domain: EffectDomain) -> CommitmentDeclaration {
        CommitmentDeclaration::builder(test_worldline(), test_scope(domain)).build()
    }

    #[test]
    fn empty_engine_approves_all() {
        let engine = PolicyEngine::new();
        let decl = simple_declaration(EffectDomain::Communication);
        let card = engine.evaluate_declaration(&decl);
        assert_eq!(card.decision, AdjudicationDecision::Approve);
    }

    #[test]
    fn deny_policy_denies() {
        let mut engine = PolicyEngine::new();
        engine
            .add_policy(Policy {
                id: PolicyId("POL-DENY-ALL".into()),
                name: "Deny All".into(),
                description: "Denies everything".into(),
                condition: PolicyCondition::Always,
                action: PolicyAction::Deny("Denied by policy".into()),
                priority: 100,
                constitutional: false,
            })
            .unwrap();

        let decl = simple_declaration(EffectDomain::Communication);
        let card = engine.evaluate_declaration(&decl);
        assert_eq!(card.decision, AdjudicationDecision::Deny);
    }

    #[test]
    fn domain_match_policy() {
        let mut engine = PolicyEngine::new();
        engine
            .add_policy(Policy {
                id: PolicyId("POL-FIN-REVIEW".into()),
                name: "Financial Review".into(),
                description: "Financial commitments need review".into(),
                condition: PolicyCondition::DomainMatch(vec![EffectDomain::Financial]),
                action: PolicyAction::RequireHumanReview("Financial review required".into()),
                priority: 100,
                constitutional: false,
            })
            .unwrap();

        // Communication should pass
        let comm_decl = simple_declaration(EffectDomain::Communication);
        let card = engine.evaluate_declaration(&comm_decl);
        assert_eq!(card.decision, AdjudicationDecision::Approve);

        // Financial should require review
        let fin_decl = simple_declaration(EffectDomain::Financial);
        let card = engine.evaluate_declaration(&fin_decl);
        assert_eq!(card.decision, AdjudicationDecision::RequireHumanReview);
    }

    #[test]
    fn constitutional_policy_cannot_be_removed() {
        let mut engine = PolicyEngine::new();
        engine
            .add_policy(Policy {
                id: PolicyId("POL-CONST-1".into()),
                name: "Constitutional".into(),
                description: "Cannot be removed".into(),
                condition: PolicyCondition::Always,
                action: PolicyAction::Approve,
                priority: 1000,
                constitutional: true,
            })
            .unwrap();

        let result = engine.remove_policy(&PolicyId("POL-CONST-1".into()));
        assert!(matches!(
            result,
            Err(PolicyError::ConstitutionalPolicyRemoval(_))
        ));
    }

    #[test]
    fn non_constitutional_policy_can_be_removed() {
        let mut engine = PolicyEngine::new();
        engine
            .add_policy(Policy {
                id: PolicyId("POL-TEMP".into()),
                name: "Temporary".into(),
                description: "Can be removed".into(),
                condition: PolicyCondition::Always,
                action: PolicyAction::Approve,
                priority: 10,
                constitutional: false,
            })
            .unwrap();

        assert!(engine.remove_policy(&PolicyId("POL-TEMP".into())).is_ok());
        assert!(engine.policies().is_empty());
    }

    #[test]
    fn priority_ordering() {
        let mut engine = PolicyEngine::new();

        // Low priority: approve
        engine
            .add_policy(Policy {
                id: PolicyId("POL-LOW".into()),
                name: "Low Priority Approve".into(),
                description: "".into(),
                condition: PolicyCondition::Always,
                action: PolicyAction::Approve,
                priority: 10,
                constitutional: false,
            })
            .unwrap();

        // High priority: deny
        engine
            .add_policy(Policy {
                id: PolicyId("POL-HIGH".into()),
                name: "High Priority Deny".into(),
                description: "".into(),
                condition: PolicyCondition::Always,
                action: PolicyAction::Deny("High priority denial".into()),
                priority: 100,
                constitutional: false,
            })
            .unwrap();

        let decl = simple_declaration(EffectDomain::Communication);
        let card = engine.evaluate_declaration(&decl);
        // High priority deny should win
        assert_eq!(card.decision, AdjudicationDecision::Deny);
    }

    #[test]
    fn duplicate_policy_rejected() {
        let mut engine = PolicyEngine::new();
        let policy = Policy {
            id: PolicyId("POL-1".into()),
            name: "Test".into(),
            description: "".into(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Approve,
            priority: 10,
            constitutional: false,
        };

        assert!(engine.add_policy(policy.clone()).is_ok());
        assert!(matches!(
            engine.add_policy(policy),
            Err(PolicyError::DuplicatePolicy(_))
        ));
    }

    #[test]
    fn irreversible_detection() {
        let mut engine = PolicyEngine::new();
        engine
            .add_policy(Policy {
                id: PolicyId("POL-IRREV".into()),
                name: "Irreversible Deny".into(),
                description: "".into(),
                condition: PolicyCondition::IrreversibleOnly,
                action: PolicyAction::Deny("Irreversible not allowed".into()),
                priority: 100,
                constitutional: false,
            })
            .unwrap();

        // Reversible passes
        let decl = simple_declaration(EffectDomain::Communication);
        let card = engine.evaluate_declaration(&decl);
        assert_eq!(card.decision, AdjudicationDecision::Approve);

        // Irreversible denied
        let irrev_decl = CommitmentDeclaration::builder(
            test_worldline(),
            test_scope(EffectDomain::Communication),
        )
        .reversibility(Reversibility::Irreversible)
        .build();
        let card = engine.evaluate_declaration(&irrev_decl);
        assert_eq!(card.decision, AdjudicationDecision::Deny);
    }

    #[test]
    fn constitutional_defaults_loaded() {
        let engine = PolicyEngine::with_constitutional_defaults();
        assert!(engine.constitutional_count() >= 3);
    }

    #[test]
    fn implements_gate_policy_provider() {
        use maple_kernel_gate::PolicyProvider;

        let engine = PolicyEngine::new();
        let decl = simple_declaration(EffectDomain::Communication);
        let card = PolicyProvider::evaluate(&engine, &decl);
        assert_eq!(card.decision, AdjudicationDecision::Approve);
    }

    #[test]
    fn compound_all_condition() {
        let mut engine = PolicyEngine::new();
        engine
            .add_policy(Policy {
                id: PolicyId("POL-COMPOUND".into()),
                name: "Compound".into(),
                description: "".into(),
                condition: PolicyCondition::All(vec![
                    PolicyCondition::DomainMatch(vec![EffectDomain::Financial]),
                    PolicyCondition::IrreversibleOnly,
                ]),
                action: PolicyAction::Deny("Financial + irreversible denied".into()),
                priority: 100,
                constitutional: false,
            })
            .unwrap();

        // Financial but reversible: passes
        let fin_rev = simple_declaration(EffectDomain::Financial);
        assert_eq!(
            engine.evaluate_declaration(&fin_rev).decision,
            AdjudicationDecision::Approve
        );

        // Financial AND irreversible: denied
        let fin_irrev =
            CommitmentDeclaration::builder(test_worldline(), test_scope(EffectDomain::Financial))
                .reversibility(Reversibility::Irreversible)
                .build();
        assert_eq!(
            engine.evaluate_declaration(&fin_irrev).decision,
            AdjudicationDecision::Deny
        );
    }
}
