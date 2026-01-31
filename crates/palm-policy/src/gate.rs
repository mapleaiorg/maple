//! Policy gate trait and implementations
//!
//! The PolicyGate trait defines the interface for policy evaluation.
//! Gates can be composed to create complex policy chains.

use crate::context::PolicyEvaluationContext;
use crate::decision::{PolicyDecision, PolicyDecisionCard, PolicyEvaluationRecord};
use crate::error::Result;
use async_trait::async_trait;
use palm_types::policy::PalmOperation;
use std::sync::Arc;
use std::time::Instant;

/// Policy gate for evaluating operations
#[async_trait]
pub trait PolicyGate: Send + Sync + std::fmt::Debug {
    /// Unique identifier for this policy gate
    fn id(&self) -> &str;

    /// Human-readable name for this policy gate
    fn name(&self) -> &str;

    /// Evaluate an operation against this policy
    async fn evaluate(
        &self,
        operation: &PalmOperation,
        context: &PolicyEvaluationContext,
    ) -> Result<PolicyDecision>;

    /// Get policy description
    fn description(&self) -> &str {
        "Policy gate"
    }

    /// Check if this policy applies to the given operation
    fn applies_to(&self, _operation: &PalmOperation) -> bool {
        true
    }

    /// Priority of this policy (higher = evaluated first)
    fn priority(&self) -> u32 {
        100
    }
}

/// Composed policy gate that chains multiple gates
#[derive(Debug)]
pub struct ComposedPolicyGate {
    id: String,
    name: String,
    gates: Vec<Arc<dyn PolicyGate>>,
    evaluation_mode: EvaluationMode,
}

/// Mode for composed policy evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationMode {
    /// All policies must allow (AND logic)
    AllMustAllow,

    /// First policy to deny wins
    FirstDenyWins,

    /// Most restrictive decision wins
    MostRestrictive,
}

impl ComposedPolicyGate {
    /// Create a new composed policy gate
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            gates: Vec::new(),
            evaluation_mode: EvaluationMode::AllMustAllow,
        }
    }

    /// Add a policy gate
    pub fn add_gate(mut self, gate: Arc<dyn PolicyGate>) -> Self {
        self.gates.push(gate);
        self
    }

    /// Set the evaluation mode
    pub fn with_evaluation_mode(mut self, mode: EvaluationMode) -> Self {
        self.evaluation_mode = mode;
        self
    }

    /// Evaluate all policies and create a decision card
    pub async fn evaluate_with_card(
        &self,
        operation: &PalmOperation,
        context: &PolicyEvaluationContext,
    ) -> Result<PolicyDecisionCard> {
        let mut card = PolicyDecisionCard::new(
            format!("{:?}", operation),
            PolicyDecision::allow(),
            &context.actor_id,
            format!("{:?}", context.platform),
            &context.environment,
            &context.request_id,
        );

        // Sort gates by priority (higher first)
        let mut sorted_gates = self.gates.clone();
        sorted_gates.sort_by(|a, b| b.priority().cmp(&a.priority()));

        let mut final_decision = PolicyDecision::allow();
        let mut most_restrictive_decision: Option<PolicyDecision> = None;

        for gate in &sorted_gates {
            if !gate.applies_to(operation) {
                continue;
            }

            let start = Instant::now();
            let decision = gate.evaluate(operation, context).await?;
            let duration_us = start.elapsed().as_micros() as u64;

            let record = PolicyEvaluationRecord::new(
                gate.id(),
                gate.name(),
                decision.clone(),
                duration_us,
            );
            card.add_evaluation(record);

            match self.evaluation_mode {
                EvaluationMode::AllMustAllow => {
                    if !decision.is_allowed() {
                        final_decision = decision;
                        break;
                    }
                }
                EvaluationMode::FirstDenyWins => {
                    if decision.is_denied() {
                        final_decision = decision;
                        break;
                    } else if !decision.is_allowed() && final_decision.is_allowed() {
                        final_decision = decision;
                    }
                }
                EvaluationMode::MostRestrictive => {
                    most_restrictive_decision = Some(Self::more_restrictive(
                        most_restrictive_decision,
                        decision,
                    ));
                }
            }
        }

        if self.evaluation_mode == EvaluationMode::MostRestrictive {
            if let Some(decision) = most_restrictive_decision {
                final_decision = decision;
            }
        }

        // Update the card's final decision
        card.decision = final_decision;
        Ok(card)
    }

    /// Compare two decisions and return the more restrictive one
    fn more_restrictive(
        current: Option<PolicyDecision>,
        new: PolicyDecision,
    ) -> PolicyDecision {
        match current {
            None => new,
            Some(current) => {
                // Order: Deny > Hold > RequiresApproval > Allow
                match (&current, &new) {
                    (PolicyDecision::Deny { .. }, _) => current,
                    (_, PolicyDecision::Deny { .. }) => new,
                    (PolicyDecision::Hold { .. }, _) => current,
                    (_, PolicyDecision::Hold { .. }) => new,
                    (PolicyDecision::RequiresApproval { .. }, _) => current,
                    (_, PolicyDecision::RequiresApproval { .. }) => new,
                    _ => current,
                }
            }
        }
    }
}

#[async_trait]
impl PolicyGate for ComposedPolicyGate {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn evaluate(
        &self,
        operation: &PalmOperation,
        context: &PolicyEvaluationContext,
    ) -> Result<PolicyDecision> {
        let card = self.evaluate_with_card(operation, context).await?;
        Ok(card.decision)
    }

    fn description(&self) -> &str {
        "Composed policy gate that chains multiple policies"
    }
}

/// Allow-all policy gate for testing
#[derive(Debug)]
pub struct AllowAllPolicyGate;

#[async_trait]
impl PolicyGate for AllowAllPolicyGate {
    fn id(&self) -> &str {
        "allow-all"
    }

    fn name(&self) -> &str {
        "Allow All Policy"
    }

    async fn evaluate(
        &self,
        _operation: &PalmOperation,
        _context: &PolicyEvaluationContext,
    ) -> Result<PolicyDecision> {
        Ok(PolicyDecision::allow())
    }

    fn description(&self) -> &str {
        "Allows all operations (for testing/development only)"
    }

    fn priority(&self) -> u32 {
        0 // Lowest priority
    }
}

/// Deny-all policy gate for testing
#[derive(Debug)]
pub struct DenyAllPolicyGate {
    reason: String,
}

impl DenyAllPolicyGate {
    /// Create a new deny-all policy gate
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl PolicyGate for DenyAllPolicyGate {
    fn id(&self) -> &str {
        "deny-all"
    }

    fn name(&self) -> &str {
        "Deny All Policy"
    }

    async fn evaluate(
        &self,
        _operation: &PalmOperation,
        _context: &PolicyEvaluationContext,
    ) -> Result<PolicyDecision> {
        Ok(PolicyDecision::deny(&self.reason, self.id()))
    }

    fn description(&self) -> &str {
        "Denies all operations"
    }

    fn priority(&self) -> u32 {
        1000 // Highest priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use palm_types::PlatformProfile;

    #[tokio::test]
    async fn test_allow_all_policy() {
        let gate = AllowAllPolicyGate;
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = gate.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_deny_all_policy() {
        let gate = DenyAllPolicyGate::new("system maintenance");
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = gate.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_denied());
        assert_eq!(decision.reason(), Some("system maintenance"));
    }

    #[tokio::test]
    async fn test_composed_policy_all_allow() {
        let composed = ComposedPolicyGate::new("composed", "Composed Policy")
            .add_gate(Arc::new(AllowAllPolicyGate))
            .add_gate(Arc::new(AllowAllPolicyGate))
            .with_evaluation_mode(EvaluationMode::AllMustAllow);

        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = composed.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_composed_policy_first_deny_wins() {
        let composed = ComposedPolicyGate::new("composed", "Composed Policy")
            .add_gate(Arc::new(AllowAllPolicyGate))
            .add_gate(Arc::new(DenyAllPolicyGate::new("denied")))
            .with_evaluation_mode(EvaluationMode::FirstDenyWins);

        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = composed.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_denied());
    }

    #[tokio::test]
    async fn test_composed_policy_with_card() {
        let composed = ComposedPolicyGate::new("composed", "Composed Policy")
            .add_gate(Arc::new(AllowAllPolicyGate))
            .add_gate(Arc::new(AllowAllPolicyGate));

        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let card = composed.evaluate_with_card(&op, &ctx).await.unwrap();
        assert!(card.was_allowed());
        assert_eq!(card.policies_evaluated.len(), 2);
    }
}
