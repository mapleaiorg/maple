use async_trait::async_trait;
use maple_mwl_types::{
    AdjudicationDecision, PolicyDecisionCard, RiskClass, RiskLevel, TemporalAnchor,
};

use crate::context::{GateContext, StageResult};
use crate::error::GateError;
use crate::traits::GateStage;

/// Stage 7: Final Decision
///
/// Aggregates all stage results and produces the final PolicyDecisionCard.
/// This is the authoritative decision that gets recorded in the Commitment Ledger.
///
/// Per I.5 (Pre-Execution Accountability): The PolicyDecisionCard is emitted
/// BEFORE any execution begins. Post-hoc attribution is forbidden.
pub struct FinalDecisionStage;

impl FinalDecisionStage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FinalDecisionStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GateStage for FinalDecisionStage {
    fn stage_name(&self) -> &str {
        "Final Decision"
    }

    fn stage_number(&self) -> u8 {
        7
    }

    async fn evaluate(&self, context: &mut GateContext) -> Result<StageResult, GateError> {
        // Check if any previous stage denied
        // Clone the reason first to release borrow on context
        let denial = context.denial_reason().cloned();
        if let Some(reason) = denial {
            // Build a final denial PolicyDecisionCard
            let card = PolicyDecisionCard {
                decision_id: uuid::Uuid::new_v4().to_string(),
                decision: AdjudicationDecision::Deny,
                rationale: reason.message.clone(),
                risk: context
                    .risk_assessment
                    .clone()
                    .unwrap_or(RiskLevel {
                        class: RiskClass::Low,
                        score: None,
                        factors: vec![],
                    }),
                conditions: vec![],
                policy_refs: reason.policy_refs.clone(),
                decided_at: TemporalAnchor::now(0),
                version: 1,
            };
            context.policy_decision = Some(card);
            return Ok(StageResult::Deny(reason));
        }

        // Check if co-signatures are still pending
        if context.requires_cosign() {
            // Check the last co-sign stage result
            let last_cosign = context.stage_results.iter().rev().find_map(|(name, r)| {
                if name == "Co-signature Collection" {
                    Some(r)
                } else {
                    None
                }
            });

            if let Some(StageResult::RequireCoSign(missing)) = last_cosign {
                return Ok(StageResult::RequireCoSign(missing.clone()));
            }
        }

        // Check if human approval is still pending
        if context.requires_human_approval() {
            let reason = context
                .stage_results
                .iter()
                .find_map(|(_, r)| match r {
                    StageResult::RequireHumanApproval(reason) => Some(reason.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "Human approval required".into());
            return Ok(StageResult::RequireHumanApproval(reason));
        }

        // All stages passed — build approval PolicyDecisionCard
        // Use the policy stage's card if available, or build one
        if context.policy_decision.is_none() {
            let card = PolicyDecisionCard {
                decision_id: uuid::Uuid::new_v4().to_string(),
                decision: AdjudicationDecision::Approve,
                rationale: "All 7 stages passed".into(),
                risk: context
                    .risk_assessment
                    .clone()
                    .unwrap_or(RiskLevel {
                        class: RiskClass::Low,
                        score: Some(0.0),
                        factors: vec![],
                    }),
                conditions: vec![],
                policy_refs: vec![],
                decided_at: TemporalAnchor::now(0),
                version: 1,
            };
            context.policy_decision = Some(card);
        }

        Ok(StageResult::Pass)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::GateContext;
    use crate::declaration::CommitmentDeclaration;
    use maple_mwl_types::{CommitmentScope, DenialReason, EffectDomain, IdentityMaterial, WorldlineId};

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
    async fn approve_when_all_pass() {
        let stage = FinalDecisionStage::new();
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        let mut ctx = GateContext::new(decl);
        ctx.record_stage("stage_1", StageResult::Pass);
        ctx.record_stage("stage_2", StageResult::Pass);
        ctx.record_stage("stage_3", StageResult::Pass);

        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_pass());
        assert!(ctx.policy_decision.is_some());
        assert_eq!(
            ctx.policy_decision.unwrap().decision,
            AdjudicationDecision::Approve
        );
    }

    #[tokio::test]
    async fn deny_when_previous_denied() {
        let stage = FinalDecisionStage::new();
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        let mut ctx = GateContext::new(decl);
        ctx.record_stage("stage_1", StageResult::Pass);
        ctx.record_stage(
            "stage_3",
            StageResult::Deny(DenialReason {
                code: "CAP_FAIL".into(),
                message: "Missing capability".into(),
                policy_refs: vec![],
            }),
        );

        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_deny());
        assert!(ctx.policy_decision.is_some());
        assert_eq!(
            ctx.policy_decision.unwrap().decision,
            AdjudicationDecision::Deny
        );
    }
}
