//! AAS Policy - Policy engine for commitment adjudication
//!
//! The policy engine evaluates commitments against defined rules.
//! This is where governance policies are enforced.

#![deny(unsafe_code)]

use aas_types::{
    AgentId, Decision, Rationale, RiskAssessment, RiskFactor, RiskLevel, RuleReference,
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
            rules: vec![Rule {
                rule_id: "critical-domain-check".to_string(),
                description: "Check if domain is critical".to_string(),
                condition: RuleCondition::DomainIsCritical,
                action: RuleAction::RequireHumanApproval,
            }],
            enabled: true,
        });

        // High-impact scope policy
        policies.push(Policy {
            policy_id: "high-impact-scope".to_string(),
            name: "High Impact Scope".to_string(),
            description: "Requires review for high-impact scopes".to_string(),
            priority: 90,
            rules: vec![Rule {
                rule_id: "global-scope-check".to_string(),
                description: "Check for global scope".to_string(),
                condition: RuleCondition::ScopeIsGlobal,
                action: RuleAction::RequireHumanApproval,
            }],
            enabled: true,
        });

        // Irreversible action policy
        policies.push(Policy {
            policy_id: "irreversible-actions".to_string(),
            name: "Irreversible Actions".to_string(),
            description: "Special handling for irreversible actions".to_string(),
            priority: 95,
            rules: vec![Rule {
                rule_id: "irreversible-check".to_string(),
                description: "Check for irreversible effects".to_string(),
                condition: RuleCondition::IsIrreversible,
                action: RuleAction::RequireHumanApproval,
            }],
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
    pub fn evaluate(
        &self,
        commitment: &RcfCommitment,
        context: &EvaluationContext,
    ) -> Result<PolicyEvaluation, PolicyError> {
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
                        RuleAction::RequireAdditionalInfo if decision == Decision::Approved => {
                            decision = Decision::PendingAdditionalInfo;
                        }
                        _ => {}
                    }
                }
            }
        }

        self.apply_runtime_guardrails(commitment, context, &mut decision, &mut risk_factors);

        let overall_risk = if risk_factors
            .iter()
            .any(|r| r.severity == RiskLevel::Critical)
        {
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

    fn apply_runtime_guardrails(
        &self,
        commitment: &RcfCommitment,
        context: &EvaluationContext,
        decision: &mut Decision,
        risk_factors: &mut Vec<RiskFactor>,
    ) {
        let tier = context
            .metadata
            .get("profile_tier")
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_else(|| "mapleverse".to_string());

        let attention_available =
            parse_u64_meta(&context.metadata, "attention_available").unwrap_or(u64::MAX);
        let attention_required =
            parse_u64_meta(&context.metadata, "attention_required").unwrap_or(0);
        if attention_required > attention_available {
            risk_factors.push(RiskFactor {
                name: "attention_bound_exceeded".to_string(),
                description: format!(
                    "attention required {} exceeds available {}",
                    attention_required, attention_available
                ),
                severity: RiskLevel::High,
            });
            *decision = Decision::Denied;
        }

        let capability_risk = context
            .metadata
            .get("capability_risk")
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_else(|| "safe".to_string());
        let capability_mode = context
            .metadata
            .get("capability_mode")
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_else(|| "simulation".to_string());

        let requested_value = parse_f64_meta(&context.metadata, "requested_value").unwrap_or(0.0);
        let autonomous_limit = autonomous_limit_for_tier(&tier);

        if capability_risk == "dangerous" {
            risk_factors.push(RiskFactor {
                name: "dangerous_capability".to_string(),
                description: "capability marked as dangerous requires stronger controls"
                    .to_string(),
                severity: RiskLevel::High,
            });

            if requested_value <= 0.0 {
                if *decision != Decision::Denied {
                    *decision = Decision::PendingAdditionalInfo;
                }
            } else if requested_value > autonomous_limit && *decision != Decision::Denied {
                *decision = Decision::PendingHumanReview;
            } else if tier == "finalverse" && *decision == Decision::Approved {
                *decision = Decision::PendingHumanReview;
            }
        }

        if capability_mode == "real" {
            risk_factors.push(RiskFactor {
                name: "real_tool_mode".to_string(),
                description: "capability is configured for real external side effects".to_string(),
                severity: RiskLevel::High,
            });
            if *decision == Decision::Approved {
                *decision = Decision::PendingHumanReview;
            }
        }

        if requested_value > autonomous_limit {
            risk_factors.push(RiskFactor {
                name: "autonomous_limit_exceeded".to_string(),
                description: format!(
                    "requested value {} exceeds autonomous limit {} for tier {}",
                    requested_value, autonomous_limit, tier
                ),
                severity: RiskLevel::High,
            });
            if *decision != Decision::Denied {
                *decision = Decision::PendingHumanReview;
            }
        }

        // iBank pure-AI lane: allow low-risk financial commitments under autonomous limit.
        // This keeps high-risk/ambiguous cases in hybrid review while enabling bounded autonomy.
        if tier == "ibank"
            && commitment.effect_domain == rcf_types::EffectDomain::Finance
            && requested_value > 0.0
            && requested_value <= autonomous_limit
            && capability_risk == "dangerous"
            && attention_required <= attention_available
            && *decision == Decision::PendingHumanReview
        {
            risk_factors.push(RiskFactor {
                name: "ibank_autonomous_lane".to_string(),
                description: format!(
                    "requested value {} is within ibank autonomous limit {}",
                    requested_value, autonomous_limit
                ),
                severity: RiskLevel::Low,
            });
            *decision = Decision::Approved;
        }

        // iBank baseline: finance commitments above autonomous limit require approval.
        if tier == "ibank"
            && commitment.effect_domain == rcf_types::EffectDomain::Finance
            && requested_value > autonomous_limit
            && *decision != Decision::Denied
        {
            *decision = Decision::PendingHumanReview;
        }
    }

    /// Evaluate a single rule
    fn evaluate_rule(
        &self,
        rule: &Rule,
        commitment: &RcfCommitment,
        context: &EvaluationContext,
    ) -> Result<RuleResult, PolicyError> {
        let triggered = match &rule.condition {
            RuleCondition::Always => true,
            RuleCondition::Never => false,
            RuleCondition::DomainIsCritical => commitment.effect_domain.is_critical(),
            RuleCondition::ScopeIsGlobal => commitment.scope.is_global(),
            RuleCondition::IsIrreversible => commitment.reversibility.is_irreversible(),
            RuleCondition::Custom(expr) => evaluate_custom_condition(expr, commitment, context)?,
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

fn evaluate_custom_condition(
    expression: &str,
    commitment: &RcfCommitment,
    context: &EvaluationContext,
) -> Result<bool, PolicyError> {
    let expr = expression.trim();
    if expr.is_empty() {
        return Err(PolicyError::InvalidExpression(
            "custom expression cannot be empty".to_string(),
        ));
    }

    if expr.contains("||") {
        for part in expr.split("||") {
            if evaluate_custom_condition(part, commitment, context)? {
                return Ok(true);
            }
        }
        return Ok(false);
    }

    if expr.contains("&&") {
        for part in expr.split("&&") {
            if !evaluate_custom_condition(part, commitment, context)? {
                return Ok(false);
            }
        }
        return Ok(true);
    }

    if let Some(capability) = parse_has_capability(expr) {
        return Ok(context.capabilities.iter().any(|cap| cap == &capability));
    }

    if let Some((lhs, rhs, equals)) = parse_comparison(expr) {
        let lhs_value = resolve_custom_lhs(lhs, commitment, context).ok_or_else(|| {
            PolicyError::InvalidExpression(format!("unsupported expression field '{}'", lhs))
        })?;
        let rhs_value = strip_quotes(rhs.trim());

        let matches = lhs_value == rhs_value;
        return Ok(if equals { matches } else { !matches });
    }

    if let Some(metadata_key) = expr.strip_prefix("metadata.") {
        if metadata_key.is_empty() {
            return Err(PolicyError::InvalidExpression(
                "metadata key cannot be empty".to_string(),
            ));
        }
        return Ok(context
            .metadata
            .get(metadata_key)
            .map(|value| is_truthy(value))
            .unwrap_or(false));
    }

    match expr {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(PolicyError::InvalidExpression(format!(
            "unsupported custom expression '{}'",
            expr
        ))),
    }
}

fn parse_comparison(expr: &str) -> Option<(&str, &str, bool)> {
    if let Some((lhs, rhs)) = expr.split_once("==") {
        return Some((lhs.trim(), rhs.trim(), true));
    }
    if let Some((lhs, rhs)) = expr.split_once("!=") {
        return Some((lhs.trim(), rhs.trim(), false));
    }
    None
}

fn parse_has_capability(expr: &str) -> Option<String> {
    if let Some(rest) = expr.strip_prefix("capability:") {
        return Some(strip_quotes(rest.trim()));
    }

    let call = expr.strip_prefix("has_capability")?;
    let args = call.trim();
    let args = args.strip_prefix('(')?.strip_suffix(')')?;
    Some(strip_quotes(args.trim()))
}

fn resolve_custom_lhs(
    lhs: &str,
    commitment: &RcfCommitment,
    context: &EvaluationContext,
) -> Option<String> {
    match lhs {
        "agent_id" => Some(context.agent_id.to_string()),
        "effect_domain" => Some(commitment.effect_domain.name().to_string()),
        "reversibility" => Some(match commitment.reversibility {
            rcf_commitment::Reversibility::Reversible => "reversible".to_string(),
            rcf_commitment::Reversibility::PartiallyReversible(_) => {
                "partially_reversible".to_string()
            }
            rcf_commitment::Reversibility::Irreversible => "irreversible".to_string(),
        }),
        "scope.global" => Some(commitment.scope.is_global().to_string()),
        _ => lhs
            .strip_prefix("metadata.")
            .and_then(|key| context.metadata.get(key).cloned()),
    }
}

fn strip_quotes(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let double = trimmed.starts_with('"') && trimmed.ends_with('"');
        let single = trimmed.starts_with('\'') && trimmed.ends_with('\'');
        if double || single {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

fn is_truthy(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "0" | "false" | "no" | "off"
    )
}

fn parse_u64_meta(metadata: &HashMap<String, String>, key: &str) -> Option<u64> {
    metadata
        .get(key)
        .and_then(|value| value.parse::<u64>().ok())
}

fn parse_f64_meta(metadata: &HashMap<String, String>, key: &str) -> Option<f64> {
    metadata
        .get(key)
        .and_then(|value| value.parse::<f64>().ok())
}

fn autonomous_limit_for_tier(tier: &str) -> f64 {
    match tier {
        "ibank" => 10_000.0,
        "finalverse" => 1_000.0,
        "mapleverse" => 25_000.0,
        _ => 5_000.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcf_commitment::{CommitmentBuilder, Reversibility};
    use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint};

    #[test]
    fn test_policy_evaluation() {
        let engine = PolicyEngine::with_defaults();

        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Finance)
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

    #[test]
    fn denies_when_attention_budget_is_exceeded() {
        let engine = PolicyEngine::with_defaults();
        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Computation)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("profile_tier".to_string(), "mapleverse".to_string());
        metadata.insert("attention_available".to_string(), "2".to_string());
        metadata.insert("attention_required".to_string(), "10".to_string());
        metadata.insert("capability_risk".to_string(), "safe".to_string());
        let context = EvaluationContext {
            agent_id: AgentId::new("test-agent"),
            capabilities: vec![],
            metadata,
        };

        let result = engine.evaluate(&commitment, &context).unwrap();
        assert_eq!(result.decision, Decision::Denied);
    }

    #[test]
    fn ibank_limit_enforces_human_review_for_large_amounts() {
        let engine = PolicyEngine::with_defaults();
        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Finance)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("profile_tier".to_string(), "ibank".to_string());
        metadata.insert("attention_available".to_string(), "100".to_string());
        metadata.insert("attention_required".to_string(), "10".to_string());
        metadata.insert("capability_risk".to_string(), "dangerous".to_string());
        metadata.insert("requested_value".to_string(), "25000".to_string());
        let context = EvaluationContext {
            agent_id: AgentId::new("test-agent"),
            capabilities: vec![],
            metadata,
        };

        let result = engine.evaluate(&commitment, &context).unwrap();
        assert_eq!(result.decision, Decision::PendingHumanReview);
    }

    #[test]
    fn ibank_low_risk_within_limit_can_autonomously_execute() {
        let engine = PolicyEngine::with_defaults();
        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Finance)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("profile_tier".to_string(), "ibank".to_string());
        metadata.insert("attention_available".to_string(), "100".to_string());
        metadata.insert("attention_required".to_string(), "10".to_string());
        metadata.insert("capability_risk".to_string(), "dangerous".to_string());
        metadata.insert("requested_value".to_string(), "100".to_string());
        let context = EvaluationContext {
            agent_id: AgentId::new("test-agent"),
            capabilities: vec![],
            metadata,
        };

        let result = engine.evaluate(&commitment, &context).unwrap();
        assert_eq!(result.decision, Decision::Approved);
    }

    #[test]
    fn real_capability_mode_requires_human_review() {
        let engine = PolicyEngine::with_defaults();
        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Computation)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("profile_tier".to_string(), "mapleverse".to_string());
        metadata.insert("attention_available".to_string(), "100".to_string());
        metadata.insert("attention_required".to_string(), "1".to_string());
        metadata.insert("capability_risk".to_string(), "safe".to_string());
        metadata.insert("capability_mode".to_string(), "real".to_string());
        let context = EvaluationContext {
            agent_id: AgentId::new("test-agent"),
            capabilities: vec![],
            metadata,
        };

        let result = engine.evaluate(&commitment, &context).unwrap();
        assert_eq!(result.decision, Decision::PendingHumanReview);
    }

    #[test]
    fn custom_metadata_rule_can_trigger() {
        let engine = PolicyEngine::with_defaults();
        engine
            .add_policy(Policy {
                policy_id: "custom-meta".to_string(),
                name: "Custom metadata rule".to_string(),
                description: "deny on production env".to_string(),
                priority: 200,
                rules: vec![Rule {
                    rule_id: "custom-meta-env".to_string(),
                    description: "check metadata env".to_string(),
                    condition: RuleCondition::Custom("metadata.env == production".to_string()),
                    action: RuleAction::Deny,
                }],
                enabled: true,
            })
            .unwrap();

        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Computation)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("env".to_string(), "production".to_string());
        let context = EvaluationContext {
            agent_id: AgentId::new("test-agent"),
            capabilities: vec![],
            metadata,
        };

        let result = engine.evaluate(&commitment, &context).unwrap();
        assert_eq!(result.decision, Decision::Denied);
        assert!(result
            .rule_results
            .iter()
            .any(|rule| rule.rule_id == "custom-meta-env" && rule.triggered));
    }

    #[test]
    fn custom_capability_rule_can_trigger() {
        let engine = PolicyEngine::new();
        engine
            .add_policy(Policy {
                policy_id: "custom-cap".to_string(),
                name: "Custom capability rule".to_string(),
                description: "require review when privileged capability exists".to_string(),
                priority: 200,
                rules: vec![Rule {
                    rule_id: "custom-capability".to_string(),
                    description: "checks capability expression".to_string(),
                    condition: RuleCondition::Custom(
                        "has_capability(\"cap-financial-settle\")".to_string(),
                    ),
                    action: RuleAction::RequireHumanApproval,
                }],
                enabled: true,
            })
            .unwrap();

        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Computation)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();

        let context = EvaluationContext {
            agent_id: AgentId::new("test-agent"),
            capabilities: vec!["cap-financial-settle".to_string()],
            metadata: HashMap::new(),
        };

        let result = engine.evaluate(&commitment, &context).unwrap();
        assert_eq!(result.decision, Decision::PendingHumanReview);
    }

    #[test]
    fn invalid_custom_expression_returns_error() {
        let engine = PolicyEngine::new();
        engine
            .add_policy(Policy {
                policy_id: "custom-invalid".to_string(),
                name: "Invalid custom rule".to_string(),
                description: "invalid expression should fail evaluation".to_string(),
                priority: 200,
                rules: vec![Rule {
                    rule_id: "invalid-expr".to_string(),
                    description: "invalid lhs".to_string(),
                    condition: RuleCondition::Custom("unknown_field == 1".to_string()),
                    action: RuleAction::Deny,
                }],
                enabled: true,
            })
            .unwrap();

        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Computation)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();

        let context = EvaluationContext {
            agent_id: AgentId::new("test-agent"),
            capabilities: vec![],
            metadata: HashMap::new(),
        };

        let result = engine.evaluate(&commitment, &context);
        assert!(matches!(result, Err(PolicyError::InvalidExpression(_))));
    }
}
