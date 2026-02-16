use std::sync::Arc;

use async_trait::async_trait;
use maple_mwl_types::DenialReason;

use crate::context::{GateContext, StageResult};
use crate::error::GateError;
use crate::traits::{CapabilityProvider, GateStage};

/// Stage 3: Capability Check
///
/// Verifies that the declaring WorldLine holds all capabilities
/// required by the commitment scope.
pub struct CapabilityCheckStage {
    capabilities: Arc<dyn CapabilityProvider>,
}

impl CapabilityCheckStage {
    pub fn new(capabilities: Arc<dyn CapabilityProvider>) -> Self {
        Self { capabilities }
    }
}

#[async_trait]
impl GateStage for CapabilityCheckStage {
    fn stage_name(&self) -> &str {
        "Capability Check"
    }

    fn stage_number(&self) -> u8 {
        3
    }

    async fn evaluate(&self, context: &mut GateContext) -> Result<StageResult, GateError> {
        let wid = &context.declaration.declaring_identity;
        let required_caps = &context.declaration.capability_refs;

        if required_caps.is_empty() {
            // No capabilities required — pass
            context.capabilities_valid = true;
            return Ok(StageResult::Pass);
        }

        let mut missing = Vec::new();

        for cap_id in required_caps {
            if !self.capabilities.has_capability(wid, cap_id) {
                missing.push(cap_id.to_string());
            }
        }

        if !missing.is_empty() {
            return Ok(StageResult::Deny(DenialReason {
                code: "INSUFFICIENT_CAPABILITIES".into(),
                message: format!(
                    "WorldLine {} lacks required capabilities: {}",
                    wid,
                    missing.join(", ")
                ),
                policy_refs: vec![],
            }));
        }

        // Verify capabilities cover the declared effect domain
        let held_caps = self.capabilities.get_capabilities(wid);
        let domain = &context.declaration.scope.effect_domain;
        let covers_domain = held_caps.iter().any(|c| c.effect_domain == *domain);

        if !covers_domain {
            return Ok(StageResult::Deny(DenialReason {
                code: "DOMAIN_NOT_COVERED".into(),
                message: format!("No held capability covers effect domain {:?}", domain),
                policy_refs: vec![],
            }));
        }

        context.capabilities_valid = true;
        Ok(StageResult::Pass)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::GateContext;
    use crate::declaration::CommitmentDeclaration;
    use crate::mocks::MockCapabilityProvider;
    use maple_mwl_types::{
        CapabilityId, CommitmentScope, EffectDomain, EventId, IdentityMaterial, WorldlineId,
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
    async fn pass_with_matching_capabilities() {
        let mut provider = MockCapabilityProvider::new();
        let wid = test_worldline();
        provider.grant(wid.clone(), "CAP-COMM", EffectDomain::Communication);

        let stage = CapabilityCheckStage::new(Arc::new(provider));

        let decl = CommitmentDeclaration::builder(wid, test_scope())
            .derived_from_intent(EventId::new())
            .capability(CapabilityId("CAP-COMM".into()))
            .build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_pass());
        assert!(ctx.capabilities_valid);
    }

    #[tokio::test]
    async fn deny_missing_capability() {
        let provider = MockCapabilityProvider::new(); // empty
        let stage = CapabilityCheckStage::new(Arc::new(provider));

        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope())
            .capability(CapabilityId("CAP-MISSING".into()))
            .build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_deny());
    }

    #[tokio::test]
    async fn pass_no_capabilities_required() {
        let provider = MockCapabilityProvider::new();
        let stage = CapabilityCheckStage::new(Arc::new(provider));

        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_pass());
    }
}
