//! AAS Policy - Policy engine for commitment adjudication
//!
//! The policy engine evaluates commitments against defined rules.
//! This is where governance policies are enforced.

#![deny(unsafe_code)]

use aas_types::{
    AgentId, Decision, RiskAssessment, RiskFactor, RiskLevel, Rationale, RuleReference,
};
use rcf_commitment::RcfCommitment;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;

/// Policy engine for evaluating commitments
pub struct PolicyEngine {
    policies: RwLock<Vec<Policy>>,
}

impl PolicyEngine {
    /// Create a new policy engine
    pub fn new() -> Self {
        Self {
            policies: RwLock::new(vec![]),
        }
    }

    /// Create a policy engine with default policies
    pub fn with_defaults() -> Self {
        let engine = Self::new();
        engine.add_default_policies();
        engine
    }

    /// Add default safety policies
    fn add_default_policies(&self) {
        let mut policies = self.policies.write().unwrap();

        // Critical domain policy - always require human approval
        policies.push(Policy {
            policy_id: "critical-domain-approval".to_string(),
            name: "Critical Domain Approval".to_string(),
            description: "Requires human approval for critical domains".to_string(),
            priority: 100,
            rules: vec![
                Rule {
                    rule_id: "critical-domain-check".to_string(),
                    description: "Check if domain is critical".to_string(),
                    condition: RuleCondition::DomainIsCritical,
                    action: RuleAction::RequireHumanApproval,
                },
            ],
            enabled: true,
        });

        // High-impact scope policy
        policies.push(Policy {
            policy_id: "high-impact-scope".to_string(),
            name: "High Impact Scope".to_string(),
            description: "Requires review for high-impact scopes".to_string(),
            priority: 90,
            rules: vec![
                Rule {
                    rule_id: "global-scope-check".to_string(),
                    description: "Check for global scope".to_string(),
                    condition: RuleCondition::ScopeIsGlobal,
                    action: RuleAction::RequireHumanApproval,
                },
            ],
            enabled: true,
        });

        // Irreversible action policy
        policies.push(Policy {
            policy_id: "irreversible-actions".to_string(),
            name: "Irreversible Actions".to_string(),
            description: "Special handling for irreversible actions".to_string(),
            priority: 95,
            rules: vec![
                Rule {
                    rule_id: "irreversible-check".to_string(),
                    description: "Check for irreversible effects".to_string(),
                    condition: RuleCondition::IsIrreversible,
                    action: RuleAction::RequireHumanApproval,
                },
            ],
            enabled: true,
        });
    }

    /// Add a policy to the engine
    pub fn add_policy(&self, policy: Policy) -> Result<(), PolicyError> {
        let mut policies = self.policies.write().map_err(|_| PolicyError::LockError)?;
        policies.push(policy);
        policies.sort_by(|a, b| b.priority.cmp(&a.priority));
        Ok(())
    }

    /// Evaluate a commitment against all policies
    pub fn evaluate(&self, commitment: &RcfCommitment, context: &EvaluationContext) -> Result<PolicyEvaluation, PolicyError> {
        let policies = self.policies.read().map_err(|_| PolicyError::LockError)?;

        let mut rule_results = vec![];
        let mut decision = Decision::Approved;
        let mut risk_factors = vec![];

        for policy in policies.iter().filter(|p| p.enabled) {
            for rule in &policy.rules {
                let result = self.evaluate_rule(rule, commitment, context)?;
                rule_results.push(result.clone());

                if result.triggered {
                    // Track risk factor
                    risk_factors.push(RiskFactor {
                        name: rule.rule_id.clone(),
                        description: rule.description.clone(),
                        severity: match &rule.action {
                            RuleAction::Deny => RiskLevel::Critical,
                            RuleAction::RequireHumanApproval => RiskLevel::High,
                            RuleAction::RequireAdditionalInfo => RiskLevel::Medium,
                            RuleAction::AddCondition(_) => RiskLevel::Low,
                            RuleAction::Allow => RiskLevel::Low,
                        },
                    });

                    // Update decision based on action
                    match &rule.action {
                        RuleAction::Deny => {
                            decision = Decision::Denied;
                        }
                        RuleAction::RequireHumanApproval if decision != Decision::Denied => {
                            decision = Decision::PendingHumanReview;
                        }
                        RuleAction::RequireAdditionalInfo
                            if decision == Decision::Approved =>
                        {
                            decision = Decision::PendingAdditionalInfo;
                        }
                        _ => {}
                    }
                }
            }
        }

        let overall_risk = if risk_factors.iter().any(|r| r.severity == RiskLevel::Critical) {
            RiskLevel::Critical
        } else if risk_factors.iter().any(|r| r.severity == RiskLevel::High) {
            RiskLevel::High
        } else if risk_factors.iter().any(|r| r.severity == RiskLevel::Medium) {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        Ok(PolicyEvaluation {
            decision,
            rationale: Rationale {
                summary: format!(
                    "Evaluated {} rules from {} policies",
                    rule_results.len(),
                    policies.len()
                ),
                rule_references: rule_results
                    .iter()
                    .map(|r| RuleReference {
                        rule_id: r.rule_id.clone(),
                        rule_description: r.description.clone(),
                        evaluation_result: !r.triggered,
                    })
                    .collect(),
            },
            risk_assessment: RiskAssessment {
                overall_risk,
                risk_factors,
                mitigations: vec![],
            },
            rule_results,
        })
    }

    /// Evaluate a single rule
    fn evaluate_rule(
        &self,
        rule: &Rule,
        commitment: &RcfCommitment,
        _context: &EvaluationContext,
    ) -> Result<RuleResult, PolicyError> {
        let triggered = match &rule.condition {
            RuleCondition::Always => true,
            RuleCondition::Never => false,
            RuleCondition::DomainIsCritical => commitment.effect_domain.is_critical(),
            RuleCondition::ScopeIsGlobal => commitment.scope.is_global(),
            RuleCondition::IsIrreversible => commitment.reversibility.is_irreversible(),
            RuleCondition::Custom(_expr) => false, // TODO: Implement custom expression evaluation
        };

        Ok(RuleResult {
            rule_id: rule.rule_id.clone(),
            description: rule.description.clone(),
            triggered,
            action: if triggered {
                Some(rule.action.clone())
            } else {
                None
            },
        })
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// A policy definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Policy {
    pub policy_id: String,
    pub name: String,
    pub description: String,
    pub priority: u32,
    pub rules: Vec<Rule>,
    pub enabled: bool,
}

/// A rule within a policy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub rule_id: String,
    pub description: String,
    pub condition: RuleCondition,
    pub action: RuleAction,
}

/// Conditions for rule evaluation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RuleCondition {
    Always,
    Never,
    DomainIsCritical,
    ScopeIsGlobal,
    IsIrreversible,
    Custom(String),
}

/// Actions taken when a rule triggers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RuleAction {
    Allow,
    Deny,
    RequireHumanApproval,
    RequireAdditionalInfo,
    AddCondition(String),
}

/// Context for policy evaluation
#[derive(Clone, Debug)]
pub struct EvaluationContext {
    pub agent_id: AgentId,
    pub capabilities: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Result of policy evaluation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyEvaluation {
    pub decision: Decision,
    pub rationale: Rationale,
    pub risk_assessment: RiskAssessment,
    pub rule_results: Vec<RuleResult>,
}

/// Result of evaluating a single rule
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleResult {
    pub rule_id: String,
    pub description: String,
    pub triggered: bool,
    pub action: Option<RuleAction>,
}

/// Policy-related errors
#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("Policy not found: {0}")]
    NotFound(String),

    #[error("Invalid rule expression: {0}")]
    InvalidExpression(String),

    #[error("Evaluation failed: {0}")]
    EvaluationFailed(String),

    #[error("Lock error")]
    LockError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcf_commitment::{CommitmentBuilder, Reversibility};
    use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint};

    #[test]
    fn test_policy_evaluation() {
        let engine = PolicyEngine::with_defaults();

        let commitment = CommitmentBuilder::new(
            IdentityRef::new("test-agent"),
            EffectDomain::Finance,
        )
        .with_scope(ScopeConstraint::default())
        .with_reversibility(Reversibility::Irreversible)
        .build()
        .unwrap();

        let context = EvaluationContext {
            agent_id: AgentId::new("test-agent"),
            capabilities: vec![],
            metadata: HashMap::new(),
        };

        let result = engine.evaluate(&commitment, &context).unwrap();

        // Financial domain should require human approval
        assert_eq!(result.decision, Decision::PendingHumanReview);
    }
}
