use std::sync::Arc;

use async_trait::async_trait;
use maple_mwl_types::{AdjudicationDecision, DenialReason};

use crate::context::{GateContext, StageResult};
use crate::error::GateError;
use crate::traits::{GateStage, PolicyProvider};

/// Stage 4: Policy Evaluation
///
/// Evaluates all applicable governance policies against the commitment.
/// Produces a PolicyDecisionCard and may deny based on policy violations.
pub struct PolicyEvaluationStage {
    policies: Arc<dyn PolicyProvider>,
}

impl PolicyEvaluationStage {
    pub fn new(policies: Arc<dyn PolicyProvider>) -> Self {
        Self { policies }
    }
}

#[async_trait]
impl GateStage for PolicyEvaluationStage {
    fn stage_name(&self) -> &str {
        "Policy Evaluation"
    }

    fn stage_number(&self) -> u8 {
        4
    }

    async fn evaluate(&self, context: &mut GateContext) -> Result<StageResult, GateError> {
        let decision_card = self.policies.evaluate(&context.declaration);

        let result = match decision_card.decision {
            AdjudicationDecision::Approve => StageResult::Pass,
            AdjudicationDecision::Deny => StageResult::Deny(DenialReason {
                code: "POLICY_DENIED".into(),
                message: decision_card.rationale.clone(),
                policy_refs: decision_card.policy_refs.clone(),
            }),
            AdjudicationDecision::RequireCoSignature => {
                // Policy requires co-signatures — defer to Stage 6
                StageResult::RequireCoSign(context.declaration.affected_parties.clone())
            }
            AdjudicationDecision::RequireHumanReview => {
                StageResult::RequireHumanApproval(decision_card.rationale.clone())
            }
        };

        // Store the policy decision card regardless of outcome
        context.policy_decision = Some(decision_card);

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::GateContext;
    use crate::declaration::CommitmentDeclaration;
    use crate::mocks::MockPolicyProvider;
    use maple_mwl_types::{CommitmentScope, EffectDomain, IdentityMaterial, WorldlineId};

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn test_scope() -> CommitmentScope {
        CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![test_worldline()],
            constraints: vec![],
        }
    }

    #[tokio::test]
    async fn pass_when_policy_approves() {
        let provider = MockPolicyProvider::approve_all();
        let stage = PolicyEvaluationStage::new(Arc::new(provider));

        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_pass());
        assert!(ctx.policy_decision.is_some());
    }

    #[tokio::test]
    async fn deny_when_policy_denies() {
        let provider = MockPolicyProvider::deny_all();
        let stage = PolicyEvaluationStage::new(Arc::new(provider));

        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_deny());
        // Decision card should still be recorded even on denial
        assert!(ctx.policy_decision.is_some());
    }

    #[tokio::test]
    async fn require_cosign_from_policy() {
        let provider = MockPolicyProvider::with_decision(
            AdjudicationDecision::RequireCoSignature,
            "Multi-party approval required",
        );
        let stage = PolicyEvaluationStage::new(Arc::new(provider));

        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(matches!(result, StageResult::RequireCoSign(_)));
    }
}
