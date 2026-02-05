//! Policy evaluator service
//!
//! The evaluator provides a high-level interface for evaluating
//! operations against the configured policy stack.

use crate::context::PolicyEvaluationContext;
use crate::decision::{PolicyDecision, PolicyDecisionCard};
use crate::error::Result;
use crate::gate::{ComposedPolicyGate, EvaluationMode, PolicyGate};
use crate::policies::create_platform_policy;
use palm_types::policy::PalmOperation;
use palm_types::PlatformProfile;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Policy evaluator service
#[derive(Debug)]
pub struct PolicyEvaluator {
    /// Platform this evaluator is configured for
    platform: PlatformProfile,

    /// Policy gates to evaluate
    gates: Arc<RwLock<Vec<Arc<dyn PolicyGate>>>>,

    /// Evaluation mode
    evaluation_mode: EvaluationMode,

    /// Whether to emit audit events
    emit_audit_events: bool,
}

impl PolicyEvaluator {
    /// Create a new policy evaluator with default platform policy
    pub fn new(platform: PlatformProfile) -> Self {
        let default_policy = create_platform_policy(platform);
        Self {
            platform,
            gates: Arc::new(RwLock::new(vec![default_policy])),
            evaluation_mode: EvaluationMode::AllMustAllow,
            emit_audit_events: true,
        }
    }

    /// Create with custom gates
    pub fn with_gates(platform: PlatformProfile, gates: Vec<Arc<dyn PolicyGate>>) -> Self {
        Self {
            platform,
            gates: Arc::new(RwLock::new(gates)),
            evaluation_mode: EvaluationMode::AllMustAllow,
            emit_audit_events: true,
        }
    }

    /// Set the evaluation mode
    pub fn with_evaluation_mode(mut self, mode: EvaluationMode) -> Self {
        self.evaluation_mode = mode;
        self
    }

    /// Set whether to emit audit events
    pub fn with_emit_audit_events(mut self, emit: bool) -> Self {
        self.emit_audit_events = emit;
        self
    }

    /// Add a policy gate
    pub async fn add_gate(&self, gate: Arc<dyn PolicyGate>) {
        let mut gates = self.gates.write().await;
        gates.push(gate);
    }

    /// Remove a policy gate by ID
    pub async fn remove_gate(&self, gate_id: &str) -> bool {
        let mut gates = self.gates.write().await;
        let len_before = gates.len();
        gates.retain(|g| g.id() != gate_id);
        gates.len() < len_before
    }

    /// Clear all gates
    pub async fn clear_gates(&self) {
        let mut gates = self.gates.write().await;
        gates.clear();
    }

    /// List all gate IDs
    pub async fn list_gates(&self) -> Vec<String> {
        let gates = self.gates.read().await;
        gates.iter().map(|g| g.id().to_string()).collect()
    }

    /// Evaluate an operation
    pub async fn evaluate(
        &self,
        operation: &PalmOperation,
        context: &PolicyEvaluationContext,
    ) -> Result<PolicyDecision> {
        let card = self.evaluate_with_card(operation, context).await?;
        Ok(card.decision)
    }

    /// Evaluate an operation and return a full decision card
    pub async fn evaluate_with_card(
        &self,
        operation: &PalmOperation,
        context: &PolicyEvaluationContext,
    ) -> Result<PolicyDecisionCard> {
        debug!(
            operation = ?operation,
            actor = %context.actor_id,
            platform = ?context.platform,
            "Evaluating policy"
        );

        let gates = self.gates.read().await;

        if gates.is_empty() {
            // No gates = allow all
            let card = PolicyDecisionCard::new(
                format!("{:?}", operation),
                PolicyDecision::allow(),
                &context.actor_id,
                format!("{:?}", context.platform),
                &context.environment,
                &context.request_id,
            );
            return Ok(card);
        }

        // Create a composed gate from all configured gates
        let mut composed =
            ComposedPolicyGate::new("evaluator", format!("{:?} Policy Evaluator", self.platform))
                .with_evaluation_mode(self.evaluation_mode);

        for gate in gates.iter() {
            composed = composed.add_gate(Arc::clone(gate));
        }

        let card = composed.evaluate_with_card(operation, context).await?;

        // Log the decision
        if self.emit_audit_events {
            self.log_decision(&card);
        }

        Ok(card)
    }

    /// Check if an operation is allowed (convenience method)
    pub async fn is_allowed(
        &self,
        operation: &PalmOperation,
        context: &PolicyEvaluationContext,
    ) -> bool {
        match self.evaluate(operation, context).await {
            Ok(decision) => decision.is_allowed(),
            Err(_) => false,
        }
    }

    /// Get the platform this evaluator is configured for
    pub fn platform(&self) -> PlatformProfile {
        self.platform
    }

    /// Log a policy decision for audit
    fn log_decision(&self, card: &PolicyDecisionCard) {
        match &card.decision {
            PolicyDecision::Allow => {
                info!(
                    request_id = %card.request_id,
                    operation = %card.operation,
                    actor = %card.actor_id,
                    "Policy allowed operation"
                );
            }
            PolicyDecision::Deny { reason, policy_id } => {
                warn!(
                    request_id = %card.request_id,
                    operation = %card.operation,
                    actor = %card.actor_id,
                    policy = %policy_id,
                    reason = %reason,
                    "Policy denied operation"
                );
            }
            PolicyDecision::RequiresApproval {
                reason,
                policy_id,
                approvers,
            } => {
                info!(
                    request_id = %card.request_id,
                    operation = %card.operation,
                    actor = %card.actor_id,
                    policy = %policy_id,
                    reason = %reason,
                    approvers = ?approvers,
                    "Policy requires approval"
                );
            }
            PolicyDecision::Hold {
                reason, policy_id, ..
            } => {
                info!(
                    request_id = %card.request_id,
                    operation = %card.operation,
                    actor = %card.actor_id,
                    policy = %policy_id,
                    reason = %reason,
                    "Policy placed operation on hold"
                );
            }
        }
    }
}

/// Builder for PolicyEvaluator
pub struct PolicyEvaluatorBuilder {
    platform: PlatformProfile,
    gates: Vec<Arc<dyn PolicyGate>>,
    evaluation_mode: EvaluationMode,
    emit_audit_events: bool,
    use_default_policy: bool,
}

impl PolicyEvaluatorBuilder {
    /// Create a new builder for the given platform
    pub fn new(platform: PlatformProfile) -> Self {
        Self {
            platform,
            gates: Vec::new(),
            evaluation_mode: EvaluationMode::AllMustAllow,
            emit_audit_events: true,
            use_default_policy: true,
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

    /// Set whether to emit audit events
    pub fn with_emit_audit_events(mut self, emit: bool) -> Self {
        self.emit_audit_events = emit;
        self
    }

    /// Set whether to use the default platform policy
    pub fn with_default_policy(mut self, use_default: bool) -> Self {
        self.use_default_policy = use_default;
        self
    }

    /// Build the evaluator
    pub fn build(self) -> PolicyEvaluator {
        let mut gates = self.gates;

        if self.use_default_policy {
            let default_policy = create_platform_policy(self.platform);
            gates.insert(0, default_policy);
        }

        PolicyEvaluator {
            platform: self.platform,
            gates: Arc::new(RwLock::new(gates)),
            evaluation_mode: self.evaluation_mode,
            emit_audit_events: self.emit_audit_events,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::{AllowAllPolicyGate, DenyAllPolicyGate};

    #[tokio::test]
    async fn test_evaluator_default_policy() {
        let evaluator = PolicyEvaluator::new(PlatformProfile::Development);
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = evaluator.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_evaluator_custom_gate() {
        let evaluator = PolicyEvaluator::with_gates(
            PlatformProfile::Development,
            vec![Arc::new(DenyAllPolicyGate::new("testing"))],
        );

        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = evaluator.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_denied());
    }

    #[tokio::test]
    async fn test_evaluator_add_remove_gate() {
        let evaluator = PolicyEvaluator::with_gates(
            PlatformProfile::Development,
            vec![Arc::new(AllowAllPolicyGate)],
        );

        assert_eq!(evaluator.list_gates().await.len(), 1);

        evaluator
            .add_gate(Arc::new(DenyAllPolicyGate::new("test")))
            .await;
        assert_eq!(evaluator.list_gates().await.len(), 2);

        evaluator.remove_gate("deny-all").await;
        assert_eq!(evaluator.list_gates().await.len(), 1);
    }

    #[tokio::test]
    async fn test_evaluator_is_allowed() {
        let evaluator = PolicyEvaluator::new(PlatformProfile::Development);
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        assert!(evaluator.is_allowed(&op, &ctx).await);
    }

    #[tokio::test]
    async fn test_evaluator_with_card() {
        let evaluator =
            PolicyEvaluator::new(PlatformProfile::Development).with_emit_audit_events(false);
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let card = evaluator.evaluate_with_card(&op, &ctx).await.unwrap();
        assert!(card.was_allowed());
        assert!(!card.id.is_empty());
    }

    #[tokio::test]
    async fn test_evaluator_builder() {
        let evaluator = PolicyEvaluatorBuilder::new(PlatformProfile::Mapleverse)
            .with_default_policy(true)
            .with_emit_audit_events(false)
            .with_evaluation_mode(EvaluationMode::FirstDenyWins)
            .build();

        assert_eq!(evaluator.platform(), PlatformProfile::Mapleverse);
    }

    #[tokio::test]
    async fn test_evaluator_builder_no_default() {
        let evaluator = PolicyEvaluatorBuilder::new(PlatformProfile::Development)
            .with_default_policy(false)
            .add_gate(Arc::new(AllowAllPolicyGate))
            .build();

        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        // Only the AllowAllPolicyGate should be evaluated
        let decision = evaluator.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_evaluator_empty_gates() {
        let evaluator = PolicyEvaluator::with_gates(PlatformProfile::Development, vec![]);
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        // Empty gates should allow all
        let decision = evaluator.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }
}
