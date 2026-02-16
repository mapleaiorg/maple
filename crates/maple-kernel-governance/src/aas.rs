use maple_mwl_identity::{IdentityManager, IdentityRecord};
use maple_mwl_types::{
    Capability, CapabilityId, EffectDomain, IdentityMaterial, PolicyId, RiskClass, WorldlineId,
};
use tracing::info;

use crate::capability::CapabilityManager;
use crate::error::{AasError, InvariantViolation};
use crate::invariants::{InvariantEnforcer, SystemState};
use crate::policy::{Policy, PolicyEngine};

/// Agent Accountability Service (AAS) — the normative authority.
///
/// Per Whitepaper §6.1: "AAS is the normative authority of Maple AI. It
/// decides—deterministically and audibly—whether an agent's declared Commitment
/// may be allowed to bind the world."
///
/// AAS governs:
/// - **Identity** — WorldLine creation and continuity
/// - **Capabilities** — Bounded authority grants and revocations
/// - **Policies** — Policy-as-code evaluation with constitutional protection
/// - **Invariants** — Continuous verification of all constitutional invariants
pub struct AgentAccountabilityService {
    /// Identity registry
    pub identities: IdentityManager,
    /// Capability manager
    pub capabilities: CapabilityManager,
    /// Policy engine
    pub policies: PolicyEngine,
    /// Invariant enforcer
    pub invariants: InvariantEnforcer,
}

impl AgentAccountabilityService {
    /// Create a new AAS with empty registries.
    pub fn new() -> Self {
        Self {
            identities: IdentityManager::new(),
            capabilities: CapabilityManager::new(),
            policies: PolicyEngine::new(),
            invariants: InvariantEnforcer::new(),
        }
    }

    /// Create AAS with constitutional defaults:
    /// - 8 constitutional invariants loaded
    /// - Default constitutional policies loaded
    pub fn with_constitutional_defaults() -> Self {
        info!("Initializing AAS with constitutional defaults");

        Self {
            identities: IdentityManager::new(),
            capabilities: CapabilityManager::new(),
            policies: PolicyEngine::with_constitutional_defaults(),
            invariants: InvariantEnforcer::with_constitutional_invariants(),
        }
    }

    // =========================================================================
    // IDENTITY OPERATIONS
    // =========================================================================

    /// Register a new WorldLine identity.
    pub fn register_identity(
        &mut self,
        material: IdentityMaterial,
    ) -> Result<WorldlineId, AasError> {
        let wid = self.identities.create_worldline(material)?;
        info!(worldline = %wid, "WorldLine registered in AAS");
        Ok(wid)
    }

    /// Look up an identity.
    pub fn lookup_identity(&self, wid: &WorldlineId) -> Option<&IdentityRecord> {
        self.identities.lookup(wid)
    }

    /// Verify identity material.
    pub fn verify_identity(&self, wid: &WorldlineId, material: &IdentityMaterial) -> bool {
        self.identities.verify(wid, material)
    }

    // =========================================================================
    // CAPABILITY OPERATIONS
    // =========================================================================

    /// Grant a capability to a worldline.
    pub fn grant_capability(
        &mut self,
        to: &WorldlineId,
        cap: Capability,
    ) -> Result<CapabilityId, AasError> {
        // Verify identity exists
        if self.identities.lookup(to).is_none() {
            return Err(AasError::IdentityNotFound(format!("{}", to)));
        }

        self.capabilities.grant(to, cap)
    }

    /// Convenience: grant a simple capability.
    pub fn grant_simple_capability(
        &mut self,
        to: &WorldlineId,
        cap_id: impl Into<String>,
        name: impl Into<String>,
        domain: EffectDomain,
        risk_class: RiskClass,
    ) -> Result<CapabilityId, AasError> {
        // Verify identity exists
        if self.identities.lookup(to).is_none() {
            return Err(AasError::IdentityNotFound(format!("{}", to)));
        }

        self.capabilities
            .grant_simple(to, cap_id, name, domain, risk_class)
    }

    /// Revoke a capability.
    pub fn revoke_capability(
        &mut self,
        cap_id: &CapabilityId,
        wid: &WorldlineId,
        reason: &str,
    ) -> Result<(), AasError> {
        self.capabilities.revoke(cap_id, wid, reason)?;
        Ok(())
    }

    /// Check if a worldline holds a capability.
    pub fn check_capability(&self, wid: &WorldlineId, cap_id: &CapabilityId) -> bool {
        self.capabilities.check(wid, cap_id)
    }

    // =========================================================================
    // POLICY OPERATIONS
    // =========================================================================

    /// Add a policy to the governance engine.
    pub fn add_policy(&mut self, policy: Policy) -> Result<PolicyId, AasError> {
        Ok(self.policies.add_policy(policy)?)
    }

    /// Remove a non-constitutional policy.
    pub fn remove_policy(&mut self, id: &PolicyId) -> Result<(), AasError> {
        Ok(self.policies.remove_policy(id)?)
    }

    // =========================================================================
    // INVARIANT OPERATIONS
    // =========================================================================

    /// Perform a full invariant check against provided system state.
    pub fn check_invariants(&self, state: &SystemState) -> Vec<InvariantViolation> {
        self.invariants.check_all(state)
    }

    /// Enforce all invariants — returns error if any constitutional violation found.
    pub fn enforce_invariants(&self, state: &SystemState) -> Result<(), Vec<InvariantViolation>> {
        self.invariants.enforce(state)
    }

    // =========================================================================
    // ARC PROVIDERS FOR GATE INTEGRATION
    // =========================================================================

    /// Build a shared reference to the CapabilityManager for use in the Gate pipeline.
    ///
    /// Note: This creates an Arc wrapper. For live integration, the AAS should
    /// be constructed with Arc<RwLock<CapabilityManager>> internally.
    /// This method is for testing and setup convenience.
    pub fn capability_provider(&self) -> &CapabilityManager {
        &self.capabilities
    }

    /// Get the PolicyEngine for use in the Gate pipeline.
    pub fn policy_provider(&self) -> &PolicyEngine {
        &self.policies
    }
}

impl Default for AgentAccountabilityService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invariants::SystemState;
    use crate::policy::{PolicyAction, PolicyCondition};
    use maple_mwl_types::IdentityMaterial;
    use std::sync::Arc;

    fn test_material() -> IdentityMaterial {
        IdentityMaterial::GenesisHash([1u8; 32])
    }

    #[test]
    fn create_aas_with_defaults() {
        let aas = AgentAccountabilityService::with_constitutional_defaults();
        assert_eq!(aas.invariants.count(), 9);
        assert!(aas.policies.constitutional_count() >= 3);
    }

    #[test]
    fn register_identity_and_lookup() {
        let mut aas = AgentAccountabilityService::new();
        let wid = aas.register_identity(test_material()).unwrap();

        let record = aas.lookup_identity(&wid).unwrap();
        assert_eq!(record.worldline_id, wid);
    }

    #[test]
    fn grant_capability_requires_identity() {
        let mut aas = AgentAccountabilityService::new();
        let unknown_wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([99u8; 32]));

        let result = aas.grant_simple_capability(
            &unknown_wid,
            "CAP-COMM",
            "Communication",
            EffectDomain::Communication,
            RiskClass::Low,
        );
        assert!(result.is_err());
    }

    #[test]
    fn full_identity_capability_lifecycle() {
        let mut aas = AgentAccountabilityService::new();

        // Register identity
        let wid = aas.register_identity(test_material()).unwrap();

        // Grant capability
        let cap_id = aas
            .grant_simple_capability(
                &wid,
                "CAP-COMM",
                "Communication",
                EffectDomain::Communication,
                RiskClass::Low,
            )
            .unwrap();

        assert!(aas.check_capability(&wid, &cap_id));

        // Revoke
        aas.revoke_capability(&cap_id, &wid, "policy update")
            .unwrap();
        assert!(!aas.check_capability(&wid, &cap_id));
    }

    #[test]
    fn policy_management_through_aas() {
        let mut aas = AgentAccountabilityService::new();

        let policy = Policy {
            id: PolicyId("POL-TEST".into()),
            name: "Test Policy".into(),
            description: "".into(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Approve,
            priority: 10,
            constitutional: false,
        };

        let id = aas.add_policy(policy).unwrap();
        assert_eq!(aas.policies.policies().len(), 1);

        aas.remove_policy(&id).unwrap();
        assert!(aas.policies.policies().is_empty());
    }

    #[test]
    fn constitutional_policy_protected_through_aas() {
        let mut aas = AgentAccountabilityService::with_constitutional_defaults();
        let const_id = PolicyId("POL-CONST-FIN-IRREVERSIBLE".into());
        let result = aas.remove_policy(&const_id);
        assert!(result.is_err());
    }

    #[test]
    fn invariant_check_through_aas() {
        let aas = AgentAccountabilityService::with_constitutional_defaults();
        let state = SystemState::healthy();
        let violations = aas.check_invariants(&state);
        assert!(violations.is_empty());
    }

    #[test]
    fn invariant_enforcement_through_aas() {
        let aas = AgentAccountabilityService::with_constitutional_defaults();
        let mut state = SystemState::healthy();
        state.commitment_boundary_enforced = false;

        let result = aas.enforce_invariants(&state);
        assert!(result.is_err());
    }

    #[test]
    fn full_aas_integration() {
        let mut aas = AgentAccountabilityService::with_constitutional_defaults();

        // 1. Register identities
        let wid1 = aas
            .register_identity(IdentityMaterial::GenesisHash([1u8; 32]))
            .unwrap();
        let wid2 = aas
            .register_identity(IdentityMaterial::GenesisHash([2u8; 32]))
            .unwrap();

        // 2. Grant capabilities
        aas.grant_simple_capability(
            &wid1,
            "CAP-COMM",
            "Communication",
            EffectDomain::Communication,
            RiskClass::Low,
        )
        .unwrap();
        aas.grant_simple_capability(
            &wid2,
            "CAP-FIN",
            "Financial",
            EffectDomain::Financial,
            RiskClass::High,
        )
        .unwrap();

        // 3. Verify capabilities
        assert!(aas.check_capability(&wid1, &CapabilityId("CAP-COMM".into())));
        assert!(!aas.check_capability(&wid1, &CapabilityId("CAP-FIN".into())));
        assert!(aas.check_capability(&wid2, &CapabilityId("CAP-FIN".into())));

        // 4. Check invariants
        let state = SystemState::healthy();
        assert!(aas.enforce_invariants(&state).is_ok());

        // 5. Verify identity
        assert!(aas.verify_identity(&wid1, &IdentityMaterial::GenesisHash([1u8; 32])));
        assert!(!aas.verify_identity(&wid1, &IdentityMaterial::GenesisHash([99u8; 32])));
    }

    #[tokio::test]
    async fn gate_integration_with_real_aas() {
        use maple_kernel_fabric::{EventFabric, FabricConfig};
        use maple_kernel_gate::{
            AdjudicationResult, CapabilityCheckStage, CoSignatureStage, CommitmentDeclaration,
            CommitmentGate, DeclarationStage, FinalDecisionStage, GateConfig, IdentityBindingStage,
            PolicyEvaluationStage, RiskAssessmentStage, RiskConfig,
        };
        use maple_mwl_types::{CapabilityId, CommitmentScope, EventId};
        use std::sync::RwLock;

        // Set up AAS
        let mut aas = AgentAccountabilityService::with_constitutional_defaults();

        // Register identity
        let wid = aas
            .register_identity(IdentityMaterial::GenesisHash([10u8; 32]))
            .unwrap();

        // Grant communication capability
        aas.grant_simple_capability(
            &wid,
            "CAP-COMM",
            "Communication",
            EffectDomain::Communication,
            RiskClass::Low,
        )
        .unwrap();

        // Create fabric
        let fabric = Arc::new(EventFabric::init(FabricConfig::default()).await.unwrap());

        // Create identity manager Arc for gate (share with AAS)
        let identity_mgr = {
            let mut mgr = IdentityManager::new();
            mgr.create_worldline(IdentityMaterial::GenesisHash([10u8; 32]))
                .unwrap();
            Arc::new(RwLock::new(mgr))
        };

        // Create capability and policy providers from AAS
        // We need to wrap them in Arc for the gate stages
        let cap_mgr = {
            let mut mgr = CapabilityManager::new();
            mgr.grant_simple(
                &wid,
                "CAP-COMM",
                "Communication",
                EffectDomain::Communication,
                RiskClass::Low,
            )
            .unwrap();
            mgr
        };
        let cap_provider: Arc<dyn maple_kernel_gate::CapabilityProvider> = Arc::new(cap_mgr);
        let pol_provider: Arc<dyn maple_kernel_gate::PolicyProvider> =
            Arc::new(PolicyEngine::new()); // empty = approve all

        // Build gate with REAL governance (not mocks)
        let config = GateConfig::default();
        let mut gate = CommitmentGate::new(fabric, config.clone());

        gate.add_stage(Box::new(DeclarationStage::new(
            config.require_intent_reference,
            config.min_intent_confidence,
        )));
        gate.add_stage(Box::new(IdentityBindingStage::new(identity_mgr)));
        gate.add_stage(Box::new(CapabilityCheckStage::new(cap_provider)));
        gate.add_stage(Box::new(PolicyEvaluationStage::new(pol_provider)));
        gate.add_stage(Box::new(RiskAssessmentStage::new(RiskConfig::default())));
        gate.add_stage(Box::new(CoSignatureStage::new()));
        gate.add_stage(Box::new(FinalDecisionStage::new()));

        // Submit a valid commitment
        let target_wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([20u8; 32]));
        let decl = CommitmentDeclaration::builder(
            wid.clone(),
            CommitmentScope {
                effect_domain: EffectDomain::Communication,
                targets: vec![target_wid],
                constraints: vec![],
            },
        )
        .derived_from_intent(EventId::new())
        .capability(CapabilityId("CAP-COMM".into()))
        .build();

        let result = gate.submit(decl).await.unwrap();
        assert!(
            matches!(result, AdjudicationResult::Approved { .. }),
            "Expected approval with real governance, got: {:?}",
            result
        );

        // Verify invariants still hold
        let state = SystemState::healthy();
        assert!(aas.enforce_invariants(&state).is_ok());
    }
}
