use std::sync::Arc;

use maple_kernel_fabric::{EventFabric, EventPayload, ResonanceStage};
use maple_mwl_types::{
    AdjudicationDecision, DenialReason, FailureReason, PolicyDecisionCard, RiskClass, RiskLevel,
    TemporalAnchor,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::context::{GateContext, StageResult};
use crate::declaration::CommitmentDeclaration;
use crate::error::GateError;
use crate::ledger::{CommitmentLedger, LedgerEntry, LedgerFilter, LifecycleEvent};
use crate::traits::GateStage;

/// Configuration for the Commitment Gate.
#[derive(Clone, Debug)]
pub struct GateConfig {
    /// Minimum intent confidence required (default: 0.6)
    pub min_intent_confidence: f64,
    /// Whether a reference to a stabilized intent event is required (default: true)
    pub require_intent_reference: bool,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            min_intent_confidence: 0.6,
            require_intent_reference: true,
        }
    }
}

/// Result of commitment adjudication through the 7-stage pipeline.
#[derive(Clone, Debug)]
pub enum AdjudicationResult {
    /// Commitment approved — ready for execution
    Approved {
        decision: PolicyDecisionCard,
    },
    /// Commitment denied — recorded as first-class entry
    Denied {
        decision: PolicyDecisionCard,
    },
    /// Commitment pending — requires co-signatures
    PendingCoSign {
        required: Vec<maple_mwl_types::WorldlineId>,
    },
    /// Commitment pending — requires human approval
    PendingHumanApproval {
        approver: String,
    },
}

/// Outcome of a committed action (for recording after execution).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CommitmentOutcome {
    Fulfilled,
    Failed(FailureReason),
    PartiallyFulfilled {
        completion: f64,
        remaining: Vec<String>,
    },
    Expired,
}

/// The Commitment Gate — 7-stage pipeline.
///
/// Per Whitepaper §2.10: "The Commitment Boundary is the hard architectural boundary
/// between cognition and action. No data, message, or control flow may cross this
/// boundary unless it is explicitly typed as a Commitment and approved by governance."
///
/// The Gate implements these invariants:
/// - I.3: Only explicit commitments cross into execution
/// - I.5: Accountability established BEFORE execution begins
/// - I.CG-1: PolicyDecisionCards immutable once recorded
/// - I.AAS-3: Commitment ledger is append-only
pub struct CommitmentGate {
    stages: Vec<Box<dyn GateStage>>,
    ledger: CommitmentLedger,
    fabric: Arc<EventFabric>,
    config: GateConfig,
}

impl CommitmentGate {
    /// Create a new Commitment Gate.
    pub fn new(fabric: Arc<EventFabric>, config: GateConfig) -> Self {
        Self {
            stages: Vec::new(),
            ledger: CommitmentLedger::new(),
            fabric,
            config,
        }
    }

    /// Add a stage to the pipeline.
    ///
    /// Stages MUST be added in order (1 through 7).
    pub fn add_stage(&mut self, stage: Box<dyn GateStage>) {
        self.stages.push(stage);
    }

    /// Configuration accessor.
    pub fn config(&self) -> &GateConfig {
        &self.config
    }

    /// Submit a commitment for adjudication through all 7 stages.
    ///
    /// This is THE entry point to the Commitment Boundary.
    /// The pipeline runs all stages sequentially. If any stage denies,
    /// the denial is recorded in the ledger and an event emitted.
    pub async fn submit(
        &mut self,
        declaration: CommitmentDeclaration,
    ) -> Result<AdjudicationResult, GateError> {
        let commitment_id = declaration.id.clone();
        let worldline_id = declaration.declaring_identity.clone();

        info!(
            commitment_id = %commitment_id,
            worldline = %worldline_id,
            "Commitment submitted to Gate"
        );

        // Emit declaration event to fabric
        let decl_event = self
            .fabric
            .emit(
                worldline_id.clone(),
                ResonanceStage::Commitment,
                EventPayload::CommitmentDeclared {
                    commitment_id: commitment_id.clone(),
                    scope: serde_json::to_value(&declaration.scope).unwrap_or_default(),
                    parties: declaration.affected_parties.clone(),
                },
                vec![],
            )
            .await?;

        // Create gate context
        let mut context = GateContext::new(declaration.clone());
        context.events_emitted.push(decl_event.id.clone());

        // Run each stage sequentially
        for stage in &self.stages {
            debug!(
                stage = stage.stage_name(),
                number = stage.stage_number(),
                "Evaluating stage"
            );

            let result = stage.evaluate(&mut context).await?;

            // Record the stage result
            context.record_stage(stage.stage_name(), result.clone());

            match &result {
                StageResult::Pass => {
                    debug!(stage = stage.stage_name(), "Stage passed");
                }
                StageResult::Deny(reason) => {
                    warn!(
                        stage = stage.stage_name(),
                        code = %reason.code,
                        message = %reason.message,
                        "Stage denied commitment"
                    );

                    // Build denial decision card (I.5: before any execution)
                    let decision = self.build_denial_card(&context, reason);

                    // Emit denial event
                    let deny_event = self
                        .fabric
                        .emit(
                            worldline_id.clone(),
                            ResonanceStage::Governance,
                            EventPayload::CommitmentDenied {
                                commitment_id: commitment_id.clone(),
                                rationale: reason.message.clone(),
                            },
                            vec![decl_event.id.clone()],
                        )
                        .await?;

                    context.events_emitted.push(deny_event.id.clone());

                    // Record in ledger — denied commitments are FIRST-CLASS records
                    let entry = LedgerEntry {
                        commitment_id: commitment_id.clone(),
                        declaration,
                        decision: decision.clone(),
                        lifecycle: vec![
                            LifecycleEvent::Declared(TemporalAnchor::now(0)),
                            LifecycleEvent::Denied {
                                at: TemporalAnchor::now(0),
                                reason: reason.clone(),
                            },
                        ],
                        created_at: TemporalAnchor::now(0),
                    };
                    self.ledger.append(entry)?;

                    return Ok(AdjudicationResult::Denied { decision });
                }
                StageResult::RequireCoSign(signers) => {
                    // Don't stop pipeline yet — let co-sign stage handle it
                    debug!(
                        stage = stage.stage_name(),
                        signers = signers.len(),
                        "Co-signatures required"
                    );
                }
                StageResult::RequireHumanApproval(reason) => {
                    debug!(
                        stage = stage.stage_name(),
                        reason = %reason,
                        "Human approval required"
                    );
                }
                StageResult::Defer(_duration) => {
                    debug!(stage = stage.stage_name(), "Stage deferred");
                }
            }
        }

        // All stages evaluated — check final result
        // If the last stage was a co-sign requirement
        if let Some((_, last_result)) = context.stage_results.last() {
            match last_result {
                StageResult::RequireCoSign(signers) => {
                    return Ok(AdjudicationResult::PendingCoSign {
                        required: signers.clone(),
                    });
                }
                StageResult::RequireHumanApproval(reason) => {
                    return Ok(AdjudicationResult::PendingHumanApproval {
                        approver: reason.clone(),
                    });
                }
                _ => {}
            }
        }

        // All stages passed — approval!
        let decision = context
            .policy_decision
            .clone()
            .unwrap_or_else(|| self.build_approval_card(&context));

        // Emit approval event (I.5: PolicyDecisionCard BEFORE execution)
        let approve_event = self
            .fabric
            .emit(
                worldline_id,
                ResonanceStage::Commitment,
                EventPayload::CommitmentApproved {
                    commitment_id: commitment_id.clone(),
                    decision_card: serde_json::to_value(&decision).unwrap_or_default(),
                },
                vec![decl_event.id.clone()],
            )
            .await?;

        context.events_emitted.push(approve_event.id.clone());

        // Record in ledger
        let entry = LedgerEntry {
            commitment_id: commitment_id.clone(),
            declaration,
            decision: decision.clone(),
            lifecycle: vec![
                LifecycleEvent::Declared(TemporalAnchor::now(0)),
                LifecycleEvent::Approved(TemporalAnchor::now(0)),
            ],
            created_at: TemporalAnchor::now(0),
        };
        self.ledger.append(entry)?;

        info!(
            commitment_id = %commitment_id,
            "Commitment approved through Gate"
        );

        Ok(AdjudicationResult::Approved { decision })
    }

    /// Record outcome for an approved commitment.
    pub async fn record_outcome(
        &mut self,
        cid: &maple_mwl_types::CommitmentId,
        outcome: CommitmentOutcome,
    ) -> Result<(), GateError> {
        let lifecycle_event = match &outcome {
            CommitmentOutcome::Fulfilled => LifecycleEvent::Fulfilled(TemporalAnchor::now(0)),
            CommitmentOutcome::Failed(reason) => LifecycleEvent::Failed {
                at: TemporalAnchor::now(0),
                reason: reason.clone(),
            },
            CommitmentOutcome::PartiallyFulfilled { .. } => {
                LifecycleEvent::Fulfilled(TemporalAnchor::now(0))
            }
            CommitmentOutcome::Expired => LifecycleEvent::Expired(TemporalAnchor::now(0)),
        };

        self.ledger.record_lifecycle(cid, lifecycle_event)?;

        // Emit consequence event
        let entry = self
            .ledger
            .history(cid)
            .ok_or_else(|| GateError::CommitmentNotFound(cid.clone()))?;

        let worldline_id = entry.declaration.declaring_identity.clone();

        match &outcome {
            CommitmentOutcome::Fulfilled => {
                self.fabric
                    .emit(
                        worldline_id,
                        ResonanceStage::Consequence,
                        EventPayload::CommitmentFulfilled {
                            commitment_id: cid.clone(),
                        },
                        vec![],
                    )
                    .await?;
            }
            CommitmentOutcome::Failed(reason) => {
                self.fabric
                    .emit(
                        worldline_id,
                        ResonanceStage::Consequence,
                        EventPayload::CommitmentFailed {
                            commitment_id: cid.clone(),
                            reason: reason.message.clone(),
                        },
                        vec![],
                    )
                    .await?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Query the commitment ledger.
    pub fn query_ledger(&self, filter: &LedgerFilter) -> Vec<&LedgerEntry> {
        self.ledger.query(filter)
    }

    /// Get the ledger for direct access.
    pub fn ledger(&self) -> &CommitmentLedger {
        &self.ledger
    }

    /// Build a denial PolicyDecisionCard.
    fn build_denial_card(
        &self,
        context: &GateContext,
        reason: &DenialReason,
    ) -> PolicyDecisionCard {
        PolicyDecisionCard {
            decision_id: uuid::Uuid::new_v4().to_string(),
            decision: AdjudicationDecision::Deny,
            rationale: reason.message.clone(),
            risk: context.risk_assessment.clone().unwrap_or(RiskLevel {
                class: RiskClass::Low,
                score: None,
                factors: vec![],
            }),
            conditions: vec![],
            policy_refs: reason.policy_refs.clone(),
            decided_at: TemporalAnchor::now(0),
            version: 1,
        }
    }

    /// Build an approval PolicyDecisionCard.
    fn build_approval_card(&self, context: &GateContext) -> PolicyDecisionCard {
        PolicyDecisionCard {
            decision_id: uuid::Uuid::new_v4().to_string(),
            decision: AdjudicationDecision::Approve,
            rationale: "All gate stages passed".into(),
            risk: context.risk_assessment.clone().unwrap_or(RiskLevel {
                class: RiskClass::Low,
                score: Some(0.0),
                factors: vec![],
            }),
            conditions: vec![],
            policy_refs: vec![],
            decided_at: TemporalAnchor::now(0),
            version: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mocks::{MockCapabilityProvider, MockPolicyProvider};
    use crate::stages::{
        CapabilityCheckStage, CoSignatureStage, DeclarationStage, FinalDecisionStage,
        IdentityBindingStage, PolicyEvaluationStage, RiskAssessmentStage,
    };
    use crate::stages::risk::RiskConfig;
    use maple_mwl_identity::IdentityManager;
    use maple_mwl_types::{
        CapabilityId, CommitmentScope, EffectDomain, EventId, IdentityMaterial,
    };

    async fn setup_gate(
        approve_policy: bool,
    ) -> (
        CommitmentGate,
        maple_mwl_types::WorldlineId,
        Arc<std::sync::RwLock<IdentityManager>>,
    ) {
        let fabric = Arc::new(
            EventFabric::init(maple_kernel_fabric::FabricConfig::default())
                .await
                .unwrap(),
        );

        let mut identity_mgr = IdentityManager::new();
        let material = IdentityMaterial::GenesisHash([1u8; 32]);
        let wid = identity_mgr.create_worldline(material).unwrap();
        let identity_mgr = Arc::new(std::sync::RwLock::new(identity_mgr));

        let mut cap_provider = MockCapabilityProvider::new();
        cap_provider.grant(wid.clone(), "CAP-COMM", EffectDomain::Communication);
        let cap_provider = Arc::new(cap_provider);

        let policy_provider: Arc<dyn crate::traits::PolicyProvider> = if approve_policy {
            Arc::new(MockPolicyProvider::approve_all())
        } else {
            Arc::new(MockPolicyProvider::deny_all())
        };

        let config = GateConfig {
            min_intent_confidence: 0.6,
            require_intent_reference: true,
        };

        let mut gate = CommitmentGate::new(fabric, config.clone());

        // Add all 7 stages
        gate.add_stage(Box::new(DeclarationStage::new(
            config.require_intent_reference,
            config.min_intent_confidence,
        )));
        gate.add_stage(Box::new(IdentityBindingStage::new(identity_mgr.clone())));
        gate.add_stage(Box::new(CapabilityCheckStage::new(cap_provider)));
        gate.add_stage(Box::new(PolicyEvaluationStage::new(policy_provider)));
        gate.add_stage(Box::new(RiskAssessmentStage::new(RiskConfig::default())));
        gate.add_stage(Box::new(CoSignatureStage::new()));
        gate.add_stage(Box::new(FinalDecisionStage::new()));

        (gate, wid, identity_mgr)
    }

    fn valid_declaration(wid: maple_mwl_types::WorldlineId) -> CommitmentDeclaration {
        CommitmentDeclaration::builder(
            wid,
            CommitmentScope {
                effect_domain: EffectDomain::Communication,
                targets: vec![maple_mwl_types::WorldlineId::derive(
                    &IdentityMaterial::GenesisHash([2u8; 32]),
                )],
                constraints: vec![],
            },
        )
        .derived_from_intent(EventId::new())
        .capability(CapabilityId("CAP-COMM".into()))
        .build()
    }

    #[tokio::test]
    async fn full_pipeline_approval() {
        let (mut gate, wid, _) = setup_gate(true).await;
        let decl = valid_declaration(wid);
        let cid = decl.id.clone();

        let result = gate.submit(decl).await.unwrap();

        assert!(matches!(result, AdjudicationResult::Approved { .. }));
        assert_eq!(gate.ledger().len(), 1);

        // Verify ledger entry
        let entry = gate.ledger().history(&cid).unwrap();
        assert_eq!(
            entry.decision.decision,
            AdjudicationDecision::Approve
        );
    }

    #[tokio::test]
    async fn full_pipeline_denial_from_policy() {
        let (mut gate, wid, _) = setup_gate(false).await;
        let decl = valid_declaration(wid);
        let cid = decl.id.clone();

        let result = gate.submit(decl).await.unwrap();

        assert!(matches!(result, AdjudicationResult::Denied { .. }));
        assert_eq!(gate.ledger().len(), 1);

        // Denied commitments are first-class records
        let entry = gate.ledger().history(&cid).unwrap();
        assert_eq!(entry.decision.decision, AdjudicationDecision::Deny);
    }

    #[tokio::test]
    async fn stage_1_rejects_without_intent_reference() {
        let (mut gate, wid, _) = setup_gate(true).await;

        // Declaration without intent reference
        let decl = CommitmentDeclaration::builder(
            wid,
            CommitmentScope {
                effect_domain: EffectDomain::Communication,
                targets: vec![maple_mwl_types::WorldlineId::derive(
                    &IdentityMaterial::GenesisHash([2u8; 32]),
                )],
                constraints: vec![],
            },
        )
        .build(); // NO derived_from_intent

        let result = gate.submit(decl).await.unwrap();
        assert!(matches!(result, AdjudicationResult::Denied { .. }));

        if let AdjudicationResult::Denied { decision } = result {
            assert!(decision.rationale.contains("I.3"));
        }
    }

    #[tokio::test]
    async fn stage_2_rejects_unknown_identity() {
        let (mut gate, _, _) = setup_gate(true).await;

        let unknown_wid = maple_mwl_types::WorldlineId::derive(
            &IdentityMaterial::GenesisHash([99u8; 32]),
        );
        let decl = CommitmentDeclaration::builder(
            unknown_wid,
            CommitmentScope {
                effect_domain: EffectDomain::Communication,
                targets: vec![maple_mwl_types::WorldlineId::derive(
                    &IdentityMaterial::GenesisHash([2u8; 32]),
                )],
                constraints: vec![],
            },
        )
        .derived_from_intent(EventId::new())
        .build();

        let result = gate.submit(decl).await.unwrap();
        assert!(matches!(result, AdjudicationResult::Denied { .. }));
    }

    #[tokio::test]
    async fn stage_3_rejects_insufficient_capabilities() {
        let (mut gate, wid, _) = setup_gate(true).await;

        // Request a capability we don't have
        let decl = CommitmentDeclaration::builder(
            wid,
            CommitmentScope {
                effect_domain: EffectDomain::Financial, // don't have financial capability
                targets: vec![maple_mwl_types::WorldlineId::derive(
                    &IdentityMaterial::GenesisHash([2u8; 32]),
                )],
                constraints: vec![],
            },
        )
        .derived_from_intent(EventId::new())
        .capability(CapabilityId("CAP-FIN".into()))
        .build();

        let result = gate.submit(decl).await.unwrap();
        assert!(matches!(result, AdjudicationResult::Denied { .. }));
    }

    #[tokio::test]
    async fn record_outcome_after_approval() {
        let (mut gate, wid, _) = setup_gate(true).await;
        let decl = valid_declaration(wid);
        let cid = decl.id.clone();

        gate.submit(decl).await.unwrap();

        // Record fulfillment
        gate.record_outcome(&cid, CommitmentOutcome::Fulfilled)
            .await
            .unwrap();

        let entry = gate.ledger().history(&cid).unwrap();
        assert!(matches!(
            entry.lifecycle.last().unwrap(),
            LifecycleEvent::Fulfilled(_)
        ));
    }

    #[tokio::test]
    async fn decision_immutable_after_creation() {
        // I.CG-1: PolicyDecisionCards are immutable once recorded
        let (mut gate, wid, _) = setup_gate(true).await;
        let decl = valid_declaration(wid);
        let cid = decl.id.clone();

        gate.submit(decl).await.unwrap();

        let decision_id_before = gate
            .ledger()
            .history(&cid)
            .unwrap()
            .decision
            .decision_id
            .clone();

        // Recording an outcome does NOT change the decision
        gate.record_outcome(&cid, CommitmentOutcome::Fulfilled)
            .await
            .unwrap();

        let decision_id_after = gate
            .ledger()
            .history(&cid)
            .unwrap()
            .decision
            .decision_id
            .clone();

        assert_eq!(decision_id_before, decision_id_after);
    }

    #[tokio::test]
    async fn ledger_query_works() {
        let (mut gate, wid, _) = setup_gate(true).await;

        // Submit two declarations
        let decl1 = valid_declaration(wid.clone());
        let decl2 = valid_declaration(wid.clone());

        gate.submit(decl1).await.unwrap();
        gate.submit(decl2).await.unwrap();

        let filter = LedgerFilter::new().with_worldline(wid);
        let results = gate.query_ledger(&filter);
        assert_eq!(results.len(), 2);
    }
}
