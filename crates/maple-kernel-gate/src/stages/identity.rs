use std::sync::Arc;

use async_trait::async_trait;
use maple_mwl_identity::IdentityManager;
use maple_mwl_types::DenialReason;

use crate::context::{GateContext, StageResult};
use crate::error::GateError;
use crate::traits::GateStage;

/// Stage 2: Identity Binding
///
/// Verifies that the declaring WorldlineId exists, has a valid continuity chain,
/// and binds the commitment to a specific continuity context.
pub struct IdentityBindingStage {
    identity_manager: Arc<std::sync::RwLock<IdentityManager>>,
}

impl IdentityBindingStage {
    pub fn new(identity_manager: Arc<std::sync::RwLock<IdentityManager>>) -> Self {
        Self { identity_manager }
    }
}

#[async_trait]
impl GateStage for IdentityBindingStage {
    fn stage_name(&self) -> &str {
        "Identity Binding"
    }

    fn stage_number(&self) -> u8 {
        2
    }

    async fn evaluate(&self, context: &mut GateContext) -> Result<StageResult, GateError> {
        let wid = &context.declaration.declaring_identity;
        let manager = self.identity_manager.read().unwrap();

        // Verify WorldlineId exists
        let record = match manager.lookup(wid) {
            Some(r) => r,
            None => {
                return Ok(StageResult::Deny(DenialReason {
                    code: "IDENTITY_NOT_FOUND".into(),
                    message: format!("WorldlineId not registered: {}", wid),
                    policy_refs: vec!["I.1".into()],
                }));
            }
        };

        // Verify continuity chain integrity
        if let Err(e) = record.continuity.verify_integrity() {
            return Ok(StageResult::Deny(DenialReason {
                code: "CONTINUITY_BROKEN".into(),
                message: format!("Continuity chain integrity check failed: {}", e),
                policy_refs: vec!["I.1".into()],
            }));
        }

        // Bind continuity context
        let continuity_ctx = manager.continuity_context(wid);
        context.identity_verified = true;
        context.continuity_context = continuity_ctx;

        Ok(StageResult::Pass)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declaration::CommitmentDeclaration;
    use crate::context::GateContext;
    use maple_mwl_types::{
        CommitmentScope, EffectDomain, EventId, IdentityMaterial, WorldlineId,
    };

    fn test_scope() -> CommitmentScope {
        CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![],
            constraints: vec!["test".into()],
        }
    }

    #[tokio::test]
    async fn pass_for_registered_identity() {
        let mut manager = IdentityManager::new();
        let material = IdentityMaterial::GenesisHash([1u8; 32]);
        let wid = manager.create_worldline(material).unwrap();

        let stage = IdentityBindingStage::new(Arc::new(std::sync::RwLock::new(manager)));

        let decl = CommitmentDeclaration::builder(wid, test_scope())
            .derived_from_intent(EventId::new())
            .build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_pass());
        assert!(ctx.identity_verified);
    }

    #[tokio::test]
    async fn deny_for_unknown_identity() {
        let manager = IdentityManager::new();
        let stage = IdentityBindingStage::new(Arc::new(std::sync::RwLock::new(manager)));

        let unknown_wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([99u8; 32]));
        let decl = CommitmentDeclaration::builder(unknown_wid, test_scope()).build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_deny());
        assert!(!ctx.identity_verified);
    }
}
