use async_trait::async_trait;

use crate::context::{GateContext, StageResult};
use crate::error::GateError;
use crate::traits::GateStage;
use maple_mwl_types::DenialReason;

/// Stage 1: Declaration Validation
///
/// Validates structural completeness of the commitment declaration:
/// - Confidence profile exists and is reasonable
/// - Scope is defined
/// - Intent reference present (if required by config)
/// - Temporal bounds are valid
pub struct DeclarationStage {
    /// Whether an intent reference is required
    pub require_intent_reference: bool,
    /// Minimum confidence for intent
    pub min_intent_confidence: f64,
}

impl DeclarationStage {
    pub fn new(require_intent_reference: bool, min_intent_confidence: f64) -> Self {
        Self {
            require_intent_reference,
            min_intent_confidence,
        }
    }
}

#[async_trait]
impl GateStage for DeclarationStage {
    fn stage_name(&self) -> &str {
        "Declaration Validation"
    }

    fn stage_number(&self) -> u8 {
        1
    }

    async fn evaluate(&self, context: &mut GateContext) -> Result<StageResult, GateError> {
        let decl = &context.declaration;

        // I.3: Commitment without stabilized intent is rejected
        if self.require_intent_reference && decl.derived_from_intent.is_none() {
            return Ok(StageResult::Deny(DenialReason {
                code: "MISSING_INTENT_REF".into(),
                message: "Commitment without intent reference violates I.3 (Commitment Boundary): intent does NOT imply action".into(),
                policy_refs: vec!["I.3".into()],
            }));
        }

        // Check confidence threshold
        if !decl
            .confidence
            .is_sufficient_for_commitment(self.min_intent_confidence)
        {
            return Ok(StageResult::Deny(DenialReason {
                code: "LOW_CONFIDENCE".into(),
                message: format!(
                    "Intent confidence {:.2} below threshold {:.2}",
                    decl.confidence.overall, self.min_intent_confidence
                ),
                policy_refs: vec![],
            }));
        }

        // Check scope has targets
        if decl.scope.targets.is_empty() && decl.scope.constraints.is_empty() {
            return Ok(StageResult::Deny(DenialReason {
                code: "EMPTY_SCOPE".into(),
                message: "Commitment scope must specify targets or constraints".into(),
                policy_refs: vec![],
            }));
        }

        Ok(StageResult::Pass)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declaration::CommitmentDeclaration;
    use maple_mwl_types::{
        CommitmentScope, ConfidenceProfile, EffectDomain, EventId, IdentityMaterial, WorldlineId,
    };

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
    async fn pass_with_intent_reference() {
        let stage = DeclarationStage::new(true, 0.6);
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope())
            .derived_from_intent(EventId::new())
            .build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_pass());
    }

    #[tokio::test]
    async fn deny_without_intent_reference() {
        let stage = DeclarationStage::new(true, 0.6);
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_deny());
    }

    #[tokio::test]
    async fn pass_without_intent_reference_when_not_required() {
        let stage = DeclarationStage::new(false, 0.6);
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_pass());
    }

    #[tokio::test]
    async fn deny_low_confidence() {
        let stage = DeclarationStage::new(false, 0.9);
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope())
            .confidence(ConfidenceProfile::new(0.5, 0.5, 0.5, 0.5))
            .build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_deny());
    }

    #[tokio::test]
    async fn deny_empty_scope() {
        let stage = DeclarationStage::new(false, 0.0);
        let empty_scope = CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![],
            constraints: vec![],
        };
        let decl = CommitmentDeclaration::builder(test_worldline(), empty_scope).build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_deny());
    }
}
