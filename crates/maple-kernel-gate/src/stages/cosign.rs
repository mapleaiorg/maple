use async_trait::async_trait;

use crate::context::{GateContext, StageResult};
use crate::error::GateError;
use crate::traits::GateStage;

/// Stage 6: Co-signature Collection
///
/// Checks if previous stages (policy or risk) flagged a co-signature requirement.
/// If required, verifies that sufficient co-signatures have been collected.
///
/// In the current implementation, co-signatures are pre-populated in the GateContext
/// by the caller. In a future implementation, this stage would orchestrate
/// async collection from other WorldLines.
pub struct CoSignatureStage;

impl CoSignatureStage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CoSignatureStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GateStage for CoSignatureStage {
    fn stage_name(&self) -> &str {
        "Co-signature Collection"
    }

    fn stage_number(&self) -> u8 {
        6
    }

    async fn evaluate(&self, context: &mut GateContext) -> Result<StageResult, GateError> {
        // Check if any previous stage requires co-signatures
        let required_signers = context.required_cosigners();

        if required_signers.is_empty() {
            // No co-signatures required
            return Ok(StageResult::Pass);
        }

        // Check if we have all required signatures
        let collected_signers: Vec<_> = context.co_signatures.iter().map(|s| &s.signer).collect();

        let missing: Vec<_> = required_signers
            .iter()
            .filter(|r| !collected_signers.contains(r))
            .collect();

        if missing.is_empty() {
            Ok(StageResult::Pass)
        } else {
            // Return the list of still-required co-signers
            Ok(StageResult::RequireCoSign(
                missing.into_iter().cloned().collect(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{CoSignature, GateContext};
    use crate::declaration::CommitmentDeclaration;
    use maple_mwl_types::{
        CommitmentScope, EffectDomain, IdentityMaterial, TemporalAnchor, WorldlineId,
    };

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn other_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]))
    }

    fn test_scope() -> CommitmentScope {
        CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![test_worldline()],
            constraints: vec![],
        }
    }

    #[tokio::test]
    async fn pass_when_no_cosign_required() {
        let stage = CoSignatureStage::new();
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_pass());
    }

    #[tokio::test]
    async fn require_cosign_when_missing() {
        let stage = CoSignatureStage::new();
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        let mut ctx = GateContext::new(decl);

        // Simulate previous stage requiring co-sign
        let signer = other_worldline();
        ctx.record_stage("policy", StageResult::RequireCoSign(vec![signer.clone()]));

        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(matches!(result, StageResult::RequireCoSign(_)));
    }

    #[tokio::test]
    async fn pass_when_all_cosigns_collected() {
        let stage = CoSignatureStage::new();
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        let mut ctx = GateContext::new(decl);

        let signer = other_worldline();
        ctx.record_stage("policy", StageResult::RequireCoSign(vec![signer.clone()]));

        // Add the required co-signature
        ctx.co_signatures.push(CoSignature {
            signer,
            signed_at: TemporalAnchor::now(0),
            signature_data: vec![1, 2, 3],
        });

        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_pass());
    }
}
