use async_trait::async_trait;
use maple_mwl_types::{Capability, CapabilityId, PolicyDecisionCard, WorldlineId};

use crate::context::GateContext;
use crate::context::StageResult;
use crate::declaration::CommitmentDeclaration;
use crate::error::GateError;

/// GateStage trait — each stage of the 7-stage pipeline.
///
/// Stages are evaluated sequentially. If a stage returns Deny,
/// the pipeline halts and the denial is recorded.
#[async_trait]
pub trait GateStage: Send + Sync {
    /// Human-readable name of this stage.
    fn stage_name(&self) -> &str;

    /// Stage number (1-7) in the pipeline.
    fn stage_number(&self) -> u8;

    /// Evaluate the commitment declaration in the current context.
    ///
    /// May modify the context (e.g., setting identity_verified, policy_decision).
    async fn evaluate(&self, context: &mut GateContext) -> Result<StageResult, GateError>;
}

/// Capability provider — implemented by AAS (future session).
///
/// For now, use MockCapabilityProvider for testing.
pub trait CapabilityProvider: Send + Sync {
    /// Check if a worldline holds a specific capability.
    fn has_capability(&self, wid: &WorldlineId, cap: &CapabilityId) -> bool;

    /// Get all capabilities held by a worldline.
    fn get_capabilities(&self, wid: &WorldlineId) -> Vec<Capability>;
}

/// Policy provider — implemented by Governance Engine (future session).
///
/// For now, use MockPolicyProvider for testing.
pub trait PolicyProvider: Send + Sync {
    /// Evaluate a commitment declaration against all applicable policies.
    /// Returns a PolicyDecisionCard with the result.
    fn evaluate(&self, declaration: &CommitmentDeclaration) -> PolicyDecisionCard;
}
