//! Shared test helpers for MWL integration tests.
//!
//! Provides kernel setup, worldline creation, and common assertion utilities.

use std::sync::Arc;

use maple_kernel_fabric::{EventFabric, EventPayload, FabricConfig, KernelEvent, ResonanceStage};
use maple_kernel_gate::{
    CapabilityCheckStage, CoSignatureStage, CommitmentDeclaration,
    CommitmentGate, DeclarationStage, FinalDecisionStage, GateConfig, IdentityBindingStage,
    MockCapabilityProvider, MockPolicyProvider, PolicyEvaluationStage, RiskAssessmentStage,
    RiskConfig,
};
use maple_kernel_mrp::MrpRouter;
use maple_kernel_provenance::ProvenanceIndex;
use maple_mwl_identity::IdentityManager;
use maple_mwl_types::{
    CapabilityId, CommitmentScope, EffectDomain, EventId, IdentityMaterial, WorldlineId,
};

/// A fully initialized MWL kernel for integration testing.
pub struct TestKernel {
    pub fabric: Arc<EventFabric>,
    pub gate: CommitmentGate,
    pub mrp_router: MrpRouter,
    pub provenance: ProvenanceIndex,
    pub identity_mgr: Arc<std::sync::RwLock<IdentityManager>>,
    pub cap_provider: Arc<MockCapabilityProvider>,
}

/// Options for kernel construction.
pub struct KernelOptions {
    pub approve_policies: bool,
    pub require_intent_reference: bool,
}

impl Default for KernelOptions {
    fn default() -> Self {
        Self {
            approve_policies: true,
            require_intent_reference: true,
        }
    }
}

impl TestKernel {
    /// Build a fully configured test kernel.
    pub async fn new(opts: KernelOptions) -> Self {
        let fabric = Arc::new(
            EventFabric::init(FabricConfig::default())
                .await
                .expect("fabric init"),
        );

        let identity_mgr = Arc::new(std::sync::RwLock::new(IdentityManager::new()));
        let cap_provider = Arc::new(MockCapabilityProvider::new());

        let policy_provider: Arc<dyn maple_kernel_gate::PolicyProvider> = if opts.approve_policies {
            Arc::new(MockPolicyProvider::approve_all())
        } else {
            Arc::new(MockPolicyProvider::deny_all())
        };

        let gate_config = GateConfig {
            min_intent_confidence: 0.6,
            require_intent_reference: opts.require_intent_reference,
        };

        let mut gate = CommitmentGate::new(fabric.clone(), gate_config.clone());
        gate.add_stage(Box::new(DeclarationStage::new(
            gate_config.require_intent_reference,
            gate_config.min_intent_confidence,
        )));
        gate.add_stage(Box::new(IdentityBindingStage::new(identity_mgr.clone())));
        gate.add_stage(Box::new(CapabilityCheckStage::new(cap_provider.clone())));
        gate.add_stage(Box::new(PolicyEvaluationStage::new(policy_provider)));
        gate.add_stage(Box::new(RiskAssessmentStage::new(RiskConfig::default())));
        gate.add_stage(Box::new(CoSignatureStage::new()));
        gate.add_stage(Box::new(FinalDecisionStage::new()));

        let mrp_router = MrpRouter::new();
        let provenance = ProvenanceIndex::new();

        Self {
            fabric,
            gate,
            mrp_router,
            provenance,
            identity_mgr,
            cap_provider,
        }
    }

    /// Create a worldline and register it with the identity manager.
    pub fn create_worldline(&self, seed: u8) -> WorldlineId {
        let material = IdentityMaterial::GenesisHash([seed; 32]);
        let mut mgr = self.identity_mgr.write().unwrap();
        mgr.create_worldline(material).expect("create worldline")
    }

    /// Create a worldline with a label.
    pub fn create_worldline_with_label(&self, seed: u8, _label: &str) -> WorldlineId {
        let material = IdentityMaterial::GenesisHash([seed; 32]);
        let mut mgr = self.identity_mgr.write().unwrap();
        let wid = mgr.create_worldline(material).expect("create worldline");
        // WorldlineId doesn't support post-creation labeling easily,
        // but we use the seed for identification in tests.
        wid
    }

    /// Grant a capability to a worldline.
    pub fn grant_capability(&self, wid: &WorldlineId, cap_id: &str, domain: EffectDomain) {
        // We need mutable access through Arc — use interior mutability pattern
        // Since MockCapabilityProvider uses RwLock internally for grants
        // Actually, we keep a clone and rebuild. For tests, re-create with grants.
        // The Arc<MockCapabilityProvider> was created before adding grants.
        // Instead, grant before building the gate. Use a helper pattern.
        //
        // NOTE: The mock provider stores grants in a HashMap behind no lock.
        // For integration tests, we grant capabilities BEFORE gate submission.
        // This is safe because we control the sequence.
        unsafe {
            let provider = Arc::as_ptr(&self.cap_provider) as *mut MockCapabilityProvider;
            (*provider).grant(wid.clone(), cap_id, domain);
        }
    }

    /// Emit a genesis (System/WorldlineCreated) event for a worldline.
    pub async fn emit_genesis(&mut self, wid: &WorldlineId) -> KernelEvent {
        let event = self
            .fabric
            .emit(
                wid.clone(),
                ResonanceStage::System,
                EventPayload::WorldlineCreated {
                    profile: "test".into(),
                },
                vec![],
            )
            .await
            .expect("emit genesis");
        self.provenance.add_event(&event).expect("index genesis");
        event
    }

    /// Emit a meaning event.
    pub async fn emit_meaning(
        &mut self,
        wid: &WorldlineId,
        parents: Vec<EventId>,
    ) -> KernelEvent {
        let event = self
            .fabric
            .emit(
                wid.clone(),
                ResonanceStage::Meaning,
                EventPayload::MeaningFormed {
                    interpretation_count: 1,
                    confidence: 0.85,
                    ambiguity_preserved: true,
                },
                parents,
            )
            .await
            .expect("emit meaning");
        self.provenance.add_event(&event).expect("index meaning");
        event
    }

    /// Emit an intent event.
    pub async fn emit_intent(
        &mut self,
        wid: &WorldlineId,
        parents: Vec<EventId>,
    ) -> KernelEvent {
        let event = self
            .fabric
            .emit(
                wid.clone(),
                ResonanceStage::Intent,
                EventPayload::IntentStabilized {
                    direction: "forward".into(),
                    confidence: 0.9,
                    conditions: vec![],
                },
                parents,
            )
            .await
            .expect("emit intent");
        self.provenance.add_event(&event).expect("index intent");
        event
    }

    /// Build a valid commitment declaration referencing an intent event.
    pub fn build_declaration(
        &self,
        wid: WorldlineId,
        intent_event_id: EventId,
        domain: EffectDomain,
        cap_id: &str,
        targets: Vec<WorldlineId>,
    ) -> CommitmentDeclaration {
        CommitmentDeclaration::builder(
            wid,
            CommitmentScope {
                effect_domain: domain,
                targets,
                constraints: vec![],
            },
        )
        .derived_from_intent(intent_event_id)
        .capability(CapabilityId(cap_id.into()))
        .build()
    }
}
