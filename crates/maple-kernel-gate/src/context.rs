use std::time::Duration;

use maple_mwl_types::{
    DenialReason, EventId, PolicyDecisionCard, RiskLevel, TemporalAnchor, WorldlineId,
};
use maple_mwl_identity::ContinuityContext;
use serde::{Deserialize, Serialize};

use crate::declaration::CommitmentDeclaration;

/// Result of a single gate stage evaluation.
#[derive(Clone, Debug)]
pub enum StageResult {
    /// Stage passed — continue to next stage
    Pass,
    /// Stage denied the commitment
    Deny(DenialReason),
    /// Stage requires co-signatures before approval
    RequireCoSign(Vec<WorldlineId>),
    /// Stage requires human-in-the-loop approval
    RequireHumanApproval(String),
    /// Stage defers decision (e.g., waiting for external input)
    Defer(Duration),
}

impl StageResult {
    /// Is this a passing result?
    pub fn is_pass(&self) -> bool {
        matches!(self, StageResult::Pass)
    }

    /// Is this a denial?
    pub fn is_deny(&self) -> bool {
        matches!(self, StageResult::Deny(_))
    }
}

/// A co-signature from another WorldLine (for multi-party approval).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoSignature {
    pub signer: WorldlineId,
    pub signed_at: TemporalAnchor,
    pub signature_data: Vec<u8>,
}

/// Context passed through all 7 stages of the Commitment Gate.
///
/// Accumulates results, identity verification state, policy decisions,
/// and emitted events as the declaration flows through the pipeline.
pub struct GateContext {
    /// The commitment declaration being evaluated
    pub declaration: CommitmentDeclaration,
    /// Whether identity has been verified (set by Stage 2)
    pub identity_verified: bool,
    /// Continuity context from identity verification (set by Stage 2)
    pub continuity_context: Option<ContinuityContext>,
    /// Whether capabilities are valid (set by Stage 3)
    pub capabilities_valid: bool,
    /// Policy decision from governance evaluation (set by Stage 4)
    pub policy_decision: Option<PolicyDecisionCard>,
    /// Risk assessment result (set by Stage 5)
    pub risk_assessment: Option<RiskLevel>,
    /// Collected co-signatures (populated by Stage 6)
    pub co_signatures: Vec<CoSignature>,
    /// Results from each stage (stage_name, result)
    pub stage_results: Vec<(String, StageResult)>,
    /// Events emitted during gate processing
    pub events_emitted: Vec<EventId>,
}

impl GateContext {
    /// Create a new gate context for a declaration.
    pub fn new(declaration: CommitmentDeclaration) -> Self {
        Self {
            declaration,
            identity_verified: false,
            continuity_context: None,
            capabilities_valid: false,
            policy_decision: None,
            risk_assessment: None,
            co_signatures: Vec::new(),
            stage_results: Vec::new(),
            events_emitted: Vec::new(),
        }
    }

    /// Record a stage result.
    pub fn record_stage(&mut self, stage_name: impl Into<String>, result: StageResult) {
        self.stage_results.push((stage_name.into(), result));
    }

    /// Check if any stage has denied the commitment.
    pub fn has_denial(&self) -> bool {
        self.stage_results.iter().any(|(_, r)| r.is_deny())
    }

    /// Get the denial reason if any stage denied.
    pub fn denial_reason(&self) -> Option<&DenialReason> {
        self.stage_results.iter().find_map(|(_, r)| match r {
            StageResult::Deny(reason) => Some(reason),
            _ => None,
        })
    }

    /// Check if co-signatures are required.
    pub fn requires_cosign(&self) -> bool {
        self.stage_results
            .iter()
            .any(|(_, r)| matches!(r, StageResult::RequireCoSign(_)))
    }

    /// Get required co-signers if any.
    pub fn required_cosigners(&self) -> Vec<WorldlineId> {
        self.stage_results
            .iter()
            .filter_map(|(_, r)| match r {
                StageResult::RequireCoSign(signers) => Some(signers.clone()),
                _ => None,
            })
            .flatten()
            .collect()
    }

    /// Check if human approval is required.
    pub fn requires_human_approval(&self) -> bool {
        self.stage_results
            .iter()
            .any(|(_, r)| matches!(r, StageResult::RequireHumanApproval(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declaration::CommitmentDeclaration;
    use maple_mwl_types::{CommitmentScope, EffectDomain, IdentityMaterial};

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

    fn test_context() -> GateContext {
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        GateContext::new(decl)
    }

    #[test]
    fn new_context_has_no_denial() {
        let ctx = test_context();
        assert!(!ctx.has_denial());
        assert!(ctx.denial_reason().is_none());
    }

    #[test]
    fn context_records_stage_results() {
        let mut ctx = test_context();
        ctx.record_stage("stage_1", StageResult::Pass);
        ctx.record_stage(
            "stage_2",
            StageResult::Deny(DenialReason {
                code: "IDENTITY_FAILED".into(),
                message: "Unknown identity".into(),
                policy_refs: vec![],
            }),
        );

        assert_eq!(ctx.stage_results.len(), 2);
        assert!(ctx.has_denial());
        assert!(ctx.denial_reason().is_some());
    }

    #[test]
    fn context_tracks_cosign_requirements() {
        let mut ctx = test_context();
        let signer = WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]));
        ctx.record_stage(
            "stage_6",
            StageResult::RequireCoSign(vec![signer.clone()]),
        );

        assert!(ctx.requires_cosign());
        assert_eq!(ctx.required_cosigners().len(), 1);
    }
}
