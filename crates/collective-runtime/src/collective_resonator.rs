//! Collective Resonator — the unified coordination entity
//!
//! A Collective Resonator coordinates commitments across multiple resonators.
//! It does NOT "think" — it routes, enforces policy, manages thresholds,
//! allocates resources, and maintains audit trails.
//!
//! This is the runtime counterpart of CollectiveMetadata (the static type).

use crate::{
    attention_allocator::{AttentionConfig, CollectiveAttentionAllocator},
    continuity::ContinuityManager,
    membership_manager::MembershipManager,
    policy_enforcer::{PolicyCheckRequest, PolicyConfig, PolicyDecision, PolicyEnforcer},
    role_router::{ActionRequest, RoleRouter, RouteResult},
    threshold_engine::{SignatureResult, ThresholdEngine},
    treasury_manager::TreasuryManager,
};
use chrono::{DateTime, Utc};
use collective_types::{
    AccountId, Amount, AttentionUnits, AuditJournal, Capability, CollectiveError, CollectiveId,
    CollectiveMetadata, CollectiveReceipt, CollectiveResult, CollectiveSpec, CollectiveStatus,
    CommitmentSignature, MemberBudgets, Permit, PermitId, ReceiptRequirement, ReceiptType, Role,
    RoleId, ThresholdCommitmentId, ThresholdPolicy,
};
use resonator_types::ResonatorId;
use tracing::{info, warn};

/// The Collective Resonator — coordination, not cognition
///
/// This is the main entry point for all collective operations.
/// It composes the specialized managers and enforces invariants
/// across all of them.
pub struct CollectiveResonator {
    /// Static metadata (identity, spec, status)
    metadata: CollectiveMetadata,

    // --- Composed managers ---
    /// Membership lifecycle
    membership: MembershipManager,
    /// Role/capability/permit routing
    router: RoleRouter,
    /// Policy enforcement
    policy: PolicyEnforcer,
    /// Threshold commitment engine
    thresholds: ThresholdEngine,
    /// Treasury management
    treasury: TreasuryManager,
    /// Attention allocation
    attention: CollectiveAttentionAllocator,
    /// Continuity/checkpointing
    continuity: ContinuityManager,
    /// Audit journal (shared across managers)
    journal: AuditJournal,
}

impl CollectiveResonator {
    /// Create a new Collective Resonator
    pub fn new(spec: CollectiveSpec) -> Self {
        let metadata = CollectiveMetadata::new(spec);
        let id = metadata.id.clone();

        info!(collective = %id, name = %metadata.spec.name, "Collective Resonator created");

        let mut journal = AuditJournal::new(id.clone());
        journal.log_receipt(CollectiveReceipt::new(
            id.clone(),
            ReceiptType::Custom("collective_created".into()),
            metadata.spec.created_by.clone(),
            format!("Collective created: {}", metadata.spec.name),
        ));

        Self {
            metadata: metadata.clone(),
            membership: MembershipManager::new(id.clone()),
            router: RoleRouter::new(),
            policy: PolicyEnforcer::with_default_config(),
            thresholds: ThresholdEngine::new(id.clone()),
            treasury: TreasuryManager::new(id.clone()),
            attention: CollectiveAttentionAllocator::with_defaults(id.clone()),
            continuity: ContinuityManager::new(id.clone()),
            journal,
        }
    }

    /// Create with custom configurations
    pub fn with_config(
        spec: CollectiveSpec,
        policy_config: PolicyConfig,
        attention_config: AttentionConfig,
    ) -> Self {
        let metadata = CollectiveMetadata::new(spec);
        let id = metadata.id.clone();

        let mut journal = AuditJournal::new(id.clone());
        journal.log_receipt(CollectiveReceipt::new(
            id.clone(),
            ReceiptType::Custom("collective_created".into()),
            metadata.spec.created_by.clone(),
            format!("Collective created: {}", metadata.spec.name),
        ));

        Self {
            metadata: metadata.clone(),
            membership: MembershipManager::new(id.clone()),
            router: RoleRouter::new(),
            policy: PolicyEnforcer::new(policy_config),
            thresholds: ThresholdEngine::new(id.clone()),
            treasury: TreasuryManager::new(id.clone()),
            attention: CollectiveAttentionAllocator::new(id.clone(), attention_config),
            continuity: ContinuityManager::new(id.clone()),
            journal,
        }
    }

    /// Create with a specific ID (for testing or restoration)
    pub fn with_id(mut self, id: CollectiveId) -> Self {
        self.metadata.id = id;
        self
    }

    // =========================================================================
    // IDENTITY & STATUS
    // =========================================================================

    /// Get the collective's ID
    pub fn id(&self) -> &CollectiveId {
        &self.metadata.id
    }

    /// Get the collective's metadata
    pub fn metadata(&self) -> &CollectiveMetadata {
        &self.metadata
    }

    /// Get current status
    pub fn status(&self) -> CollectiveStatus {
        self.metadata.status
    }

    /// Suspend the collective
    pub fn suspend(&mut self, reason: &str) {
        self.metadata.status = CollectiveStatus::Suspended;
        self.metadata.updated_at = Utc::now();
        self.policy
            .set_collective_status(CollectiveStatus::Suspended);

        warn!(collective = %self.metadata.id, reason = reason, "Collective suspended");

        self.journal.log_receipt(CollectiveReceipt::new(
            self.metadata.id.clone(),
            ReceiptType::Custom("collective_suspended".into()),
            ResonatorId::new("system"),
            format!("Collective suspended: {}", reason),
        ));
    }

    /// Resume the collective
    pub fn resume(&mut self) {
        self.metadata.status = CollectiveStatus::Active;
        self.metadata.updated_at = Utc::now();
        self.policy.set_collective_status(CollectiveStatus::Active);

        info!(collective = %self.metadata.id, "Collective resumed");

        self.journal.log_receipt(CollectiveReceipt::new(
            self.metadata.id.clone(),
            ReceiptType::Custom("collective_resumed".into()),
            ResonatorId::new("system"),
            "Collective resumed".to_string(),
        ));
    }

    /// Dissolve the collective (permanent)
    pub fn dissolve(&mut self, reason: &str) {
        self.metadata.status = CollectiveStatus::Dissolved;
        self.metadata.updated_at = Utc::now();
        self.policy
            .set_collective_status(CollectiveStatus::Dissolved);

        warn!(collective = %self.metadata.id, reason = reason, "Collective dissolved");

        self.journal.log_receipt(CollectiveReceipt::new(
            self.metadata.id.clone(),
            ReceiptType::Custom("collective_dissolved".into()),
            ResonatorId::new("system"),
            format!("Collective dissolved: {}", reason),
        ));

        // Take final checkpoint
        self.checkpoint();
    }

    // =========================================================================
    // MEMBERSHIP OPERATIONS
    // =========================================================================

    /// Add a member to the collective
    pub fn add_member(
        &mut self,
        resonator_id: ResonatorId,
        initial_roles: Vec<RoleId>,
    ) -> CollectiveResult<()> {
        self.ensure_active()?;

        // Add member
        self.membership
            .add_member(resonator_id.clone(), initial_roles, &mut self.journal)?;

        // Allocate default attention
        self.attention
            .allocate_default(resonator_id, &mut self.journal)?;

        Ok(())
    }

    /// Add a member with custom budgets
    pub fn add_member_with_budgets(
        &mut self,
        resonator_id: ResonatorId,
        initial_roles: Vec<RoleId>,
        budgets: MemberBudgets,
        attention: AttentionUnits,
    ) -> CollectiveResult<()> {
        self.ensure_active()?;

        self.membership.add_member_with_budgets(
            resonator_id.clone(),
            initial_roles,
            budgets,
            &mut self.journal,
        )?;

        self.attention
            .allocate_attention(resonator_id, attention, &mut self.journal)?;

        Ok(())
    }

    /// Remove a member (voluntary departure)
    pub fn remove_member(&mut self, resonator_id: &ResonatorId) -> CollectiveResult<()> {
        self.membership
            .remove_member(resonator_id, &mut self.journal)?;

        // Release their attention
        self.attention.release_all_attention(resonator_id);

        Ok(())
    }

    /// Suspend a member
    pub fn suspend_member(
        &mut self,
        resonator_id: &ResonatorId,
        reason: &str,
    ) -> CollectiveResult<()> {
        self.membership
            .suspend_member(resonator_id, reason, &mut self.journal)
    }

    /// Reinstate a suspended member
    pub fn reinstate_member(&mut self, resonator_id: &ResonatorId) -> CollectiveResult<()> {
        self.membership
            .reinstate_member(resonator_id, &mut self.journal)
    }

    /// Expel a member
    pub fn expel_member(
        &mut self,
        resonator_id: &ResonatorId,
        reason: &str,
    ) -> CollectiveResult<()> {
        self.membership
            .expel_member(resonator_id, reason, &mut self.journal)?;

        // Release their attention
        self.attention.release_all_attention(resonator_id);

        Ok(())
    }

    /// Assign a role to a member
    pub fn assign_role(
        &mut self,
        resonator_id: &ResonatorId,
        role_id: RoleId,
    ) -> CollectiveResult<()> {
        self.membership
            .assign_role(resonator_id, role_id, &mut self.journal)
    }

    /// Revoke a role from a member
    pub fn revoke_role(
        &mut self,
        resonator_id: &ResonatorId,
        role_id: &RoleId,
    ) -> CollectiveResult<()> {
        self.membership
            .revoke_role(resonator_id, role_id, &mut self.journal)
    }

    // =========================================================================
    // ROLE & CAPABILITY MANAGEMENT
    // =========================================================================

    /// Register a role
    pub fn register_role(&mut self, role: Role) {
        self.router.register_role(role);
    }

    /// Register a capability
    pub fn register_capability(&mut self, capability: Capability) {
        self.router.register_capability(capability);
    }

    /// Issue a permit
    pub fn issue_permit(&mut self, permit: Permit) -> PermitId {
        self.router.issue_permit(permit)
    }

    /// Route an action to eligible resonators
    pub fn route_action(&self, request: &ActionRequest) -> CollectiveResult<RouteResult> {
        self.router.route_action(request, self.membership.graph())
    }

    /// Route to a specific role
    pub fn route_to_role(&self, role_id: &RoleId) -> CollectiveResult<Vec<ResonatorId>> {
        self.router.route_to_role(role_id, self.membership.graph())
    }

    // =========================================================================
    // POLICY ENFORCEMENT
    // =========================================================================

    /// Check policy for an action
    pub fn check_policy(&self, request: &PolicyCheckRequest) -> CollectiveResult<PolicyDecision> {
        let member = self
            .membership
            .get_member(&request.actor)
            .ok_or_else(|| CollectiveError::MemberNotFound(request.actor.clone()))?;

        // Get role constraints
        let constraints = self
            .router
            .role_registry()
            .get_role(&request.role)
            .map(|r| r.constraints.clone())
            .unwrap_or_default();

        Ok(self.policy.check_policy(request, member, &constraints))
    }

    // =========================================================================
    // THRESHOLD COMMITMENTS
    // =========================================================================

    /// Create a threshold commitment
    pub fn create_threshold_commitment(
        &mut self,
        action_description: impl Into<String>,
        threshold: ThresholdPolicy,
        value: Option<u64>,
        deadline: Option<DateTime<Utc>>,
        receipt_requirements: Vec<ReceiptRequirement>,
    ) -> CollectiveResult<ThresholdCommitmentId> {
        self.ensure_active()?;

        let id = self.thresholds.create_commitment(
            action_description,
            threshold,
            value,
            deadline,
            receipt_requirements,
            &mut self.journal,
        );

        Ok(id)
    }

    /// Add a signature to a threshold commitment
    pub fn sign_threshold_commitment(
        &mut self,
        commitment_id: &ThresholdCommitmentId,
        signature: CommitmentSignature,
    ) -> CollectiveResult<SignatureResult> {
        // Verify signer is an active member
        if !self.membership.is_active_member(&signature.signer) {
            return Err(CollectiveError::MemberNotActive(signature.signer.clone()));
        }

        self.thresholds
            .add_signature(commitment_id, signature, &mut self.journal)
    }

    /// Check if a threshold commitment is satisfied
    pub fn is_threshold_satisfied(&self, commitment_id: &ThresholdCommitmentId) -> bool {
        self.thresholds.is_satisfied(commitment_id)
    }

    /// Expire stale threshold commitments
    pub fn expire_stale_thresholds(&mut self) -> Vec<ThresholdCommitmentId> {
        self.thresholds.expire_stale_commitments(&mut self.journal)
    }

    // =========================================================================
    // TREASURY OPERATIONS
    // =========================================================================

    /// Deposit into the treasury
    pub fn treasury_deposit(
        &mut self,
        account_id: &AccountId,
        amount: Amount,
        depositor: &ResonatorId,
    ) -> CollectiveResult<()> {
        self.treasury
            .deposit(account_id, amount, depositor, &mut self.journal)
    }

    /// Withdraw from the treasury
    pub fn treasury_withdraw(
        &mut self,
        account_id: &AccountId,
        amount: Amount,
        withdrawer: &ResonatorId,
    ) -> CollectiveResult<()> {
        // Verify withdrawer is active member
        if !self.membership.is_active_member(withdrawer) {
            return Err(CollectiveError::MemberNotActive(withdrawer.clone()));
        }

        self.treasury
            .withdraw(account_id, amount, withdrawer, &mut self.journal)
    }

    /// Get total treasury balance
    pub fn treasury_balance(&self) -> Amount {
        self.treasury.total_balance()
    }

    // =========================================================================
    // ATTENTION MANAGEMENT
    // =========================================================================

    /// Get available attention
    pub fn available_attention(&self) -> AttentionUnits {
        self.attention.available_attention()
    }

    /// Get attention utilization ratio
    pub fn attention_utilization(&self) -> f64 {
        self.attention.utilization_ratio()
    }

    /// Rebalance attention across active members
    pub fn rebalance_attention(&mut self) {
        let member_ids = self.membership.active_resonator_ids();
        self.attention.rebalance(&member_ids, &mut self.journal);
    }

    // =========================================================================
    // CONTINUITY & CHECKPOINTING
    // =========================================================================

    /// Take a checkpoint of the collective's state
    pub fn checkpoint(&mut self) -> u64 {
        let cp = self.continuity.checkpoint(
            self.metadata.clone(),
            self.membership.graph().clone(),
            self.router.role_registry().clone(),
            self.treasury.treasury().clone(),
            self.journal.clone(),
        );
        cp.sequence
    }

    /// Verify checkpoint chain integrity
    pub fn verify_continuity(&self) -> bool {
        self.continuity.verify_chain_integrity()
    }

    /// Current checkpoint sequence
    pub fn checkpoint_sequence(&self) -> u64 {
        self.continuity.current_sequence()
    }

    // =========================================================================
    // AGGREGATE QUERIES
    // =========================================================================

    /// Number of active members
    pub fn active_member_count(&self) -> usize {
        self.membership.active_member_count()
    }

    /// Check if a resonator is an active member
    pub fn is_active_member(&self, resonator_id: &ResonatorId) -> bool {
        self.membership.is_active_member(resonator_id)
    }

    /// Get the audit journal
    pub fn journal(&self) -> &AuditJournal {
        &self.journal
    }

    /// Total receipts logged
    pub fn receipt_count(&self) -> usize {
        self.journal.receipt_count()
    }

    /// Get the threshold engine (read-only)
    pub fn thresholds(&self) -> &ThresholdEngine {
        &self.thresholds
    }

    /// Get the attention allocator (read-only)
    pub fn attention(&self) -> &CollectiveAttentionAllocator {
        &self.attention
    }

    /// Get the role router (read-only)
    pub fn router(&self) -> &RoleRouter {
        &self.router
    }

    /// Get the membership manager (read-only)
    pub fn membership(&self) -> &MembershipManager {
        &self.membership
    }

    // =========================================================================
    // INTERNAL HELPERS
    // =========================================================================

    /// Ensure the collective is active
    fn ensure_active(&self) -> CollectiveResult<()> {
        if !self.metadata.status.is_active() {
            Err(CollectiveError::CollectiveNotActive(
                self.metadata.id.clone(),
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collective_types::{ActionType, CapabilityId, RoleBinding};
    use rcf_types::EffectDomain;

    fn make_collective() -> CollectiveResonator {
        let spec = CollectiveSpec::new(
            "Test Corp",
            "A test collective",
            ResonatorId::new("founder"),
        );
        CollectiveResonator::new(spec)
    }

    #[test]
    fn test_create_collective() {
        let cr = make_collective();
        assert_eq!(cr.status(), CollectiveStatus::Active);
        assert_eq!(cr.active_member_count(), 0);
        assert!(cr.receipt_count() > 0); // Creation receipt
    }

    #[test]
    fn test_member_lifecycle() {
        let mut cr = make_collective();

        // Add member
        cr.add_member(ResonatorId::new("res-1"), vec![RoleId::new("worker")])
            .unwrap();
        assert_eq!(cr.active_member_count(), 1);
        assert!(cr.is_active_member(&ResonatorId::new("res-1")));

        // Suspend
        cr.suspend_member(&ResonatorId::new("res-1"), "testing")
            .unwrap();
        assert_eq!(cr.active_member_count(), 0);

        // Reinstate
        cr.reinstate_member(&ResonatorId::new("res-1")).unwrap();
        assert_eq!(cr.active_member_count(), 1);

        // Remove
        cr.remove_member(&ResonatorId::new("res-1")).unwrap();
        assert_eq!(cr.active_member_count(), 0);
    }

    #[test]
    fn test_role_routing() {
        let mut cr = make_collective();

        // Setup role + capability
        let cap = Capability::new("Execute Trade", "Execute trades", ActionType::Execute)
            .with_id(CapabilityId::new("trade"));
        cr.register_capability(cap);

        let role = Role::new("Trader", "Executes trades")
            .with_id(RoleId::new("trader"))
            .with_capability(CapabilityId::new("trade"));
        cr.register_role(role);

        // Add member with role
        cr.add_member(ResonatorId::new("trader-1"), vec![RoleId::new("trader")])
            .unwrap();

        // Bind role
        cr.router.role_registry_mut().bind(RoleBinding::new(
            ResonatorId::new("trader-1"),
            RoleId::new("trader"),
            ResonatorId::new("admin"),
        ));

        // Route action
        let request = ActionRequest::new(ActionType::Execute, EffectDomain::Finance);
        let result = cr.route_action(&request).unwrap();
        assert_eq!(result.eligible_resonators.len(), 1);
        assert_eq!(result.eligible_resonators[0], ResonatorId::new("trader-1"));
    }

    #[test]
    fn test_threshold_commitment() {
        let mut cr = make_collective();

        // Add two members
        cr.add_member(ResonatorId::new("member-1"), vec![]).unwrap();
        cr.add_member(ResonatorId::new("member-2"), vec![]).unwrap();

        // Create 2-of-2 threshold
        let tc_id = cr
            .create_threshold_commitment(
                "Approve budget",
                ThresholdPolicy::m_of_n(2, 2),
                Some(50_000),
                None,
                vec![],
            )
            .unwrap();

        assert!(!cr.is_threshold_satisfied(&tc_id));

        // First signature
        let sig1 = CommitmentSignature::new(ResonatorId::new("member-1"));
        let result = cr.sign_threshold_commitment(&tc_id, sig1).unwrap();
        assert!(matches!(result, SignatureResult::Accepted { .. }));

        // Second signature
        let sig2 = CommitmentSignature::new(ResonatorId::new("member-2"));
        let result = cr.sign_threshold_commitment(&tc_id, sig2).unwrap();
        assert!(matches!(result, SignatureResult::ThresholdMet));
        assert!(cr.is_threshold_satisfied(&tc_id));
    }

    #[test]
    fn test_non_member_cannot_sign() {
        let mut cr = make_collective();

        let tc_id = cr
            .create_threshold_commitment("Test", ThresholdPolicy::SingleSigner, None, None, vec![])
            .unwrap();

        let sig = CommitmentSignature::new(ResonatorId::new("outsider"));
        let result = cr.sign_threshold_commitment(&tc_id, sig);
        assert!(result.is_err());
    }

    #[test]
    fn test_treasury_operations() {
        let mut cr = make_collective();
        let depositor = ResonatorId::new("member-1");
        cr.add_member(depositor.clone(), vec![]).unwrap();

        let op_id = AccountId::new("operating");
        cr.treasury_deposit(&op_id, Amount::new(100_000), &depositor)
            .unwrap();
        assert_eq!(cr.treasury_balance(), Amount::new(100_000));

        cr.treasury_withdraw(&op_id, Amount::new(25_000), &depositor)
            .unwrap();
        assert_eq!(cr.treasury_balance(), Amount::new(75_000));
    }

    #[test]
    fn test_non_member_cannot_withdraw() {
        let mut cr = make_collective();
        let op_id = AccountId::new("operating");

        // Deposit directly via treasury
        cr.treasury_deposit(&op_id, Amount::new(100_000), &ResonatorId::new("system"))
            .unwrap();

        let result =
            cr.treasury_withdraw(&op_id, Amount::new(1_000), &ResonatorId::new("outsider"));
        assert!(result.is_err());
    }

    #[test]
    fn test_suspend_blocks_operations() {
        let mut cr = make_collective();
        cr.suspend("maintenance");

        assert_eq!(cr.status(), CollectiveStatus::Suspended);

        let result = cr.add_member(ResonatorId::new("new"), vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_checkpoint_and_restore() {
        let mut cr = make_collective();

        cr.add_member(ResonatorId::new("res-1"), vec![]).unwrap();
        cr.add_member(ResonatorId::new("res-2"), vec![]).unwrap();

        let seq = cr.checkpoint();
        assert_eq!(seq, 1);
        assert!(cr.verify_continuity());

        // Second checkpoint
        cr.add_member(ResonatorId::new("res-3"), vec![]).unwrap();
        let seq2 = cr.checkpoint();
        assert_eq!(seq2, 2);
        assert!(cr.verify_continuity());
    }

    #[test]
    fn test_attention_management() {
        let mut cr = make_collective();

        cr.add_member(ResonatorId::new("res-1"), vec![]).unwrap();
        cr.add_member(ResonatorId::new("res-2"), vec![]).unwrap();

        // Each gets default allocation, so available should be reduced
        let available = cr.available_attention();
        assert!(available.0 > 0);

        // Rebalance
        cr.rebalance_attention();
        let util = cr.attention_utilization();
        assert!(util > 0.0);
    }

    #[test]
    fn test_dissolve() {
        let mut cr = make_collective();
        cr.add_member(ResonatorId::new("res-1"), vec![]).unwrap();

        cr.dissolve("End of life");
        assert_eq!(cr.status(), CollectiveStatus::Dissolved);

        // Should have taken a final checkpoint
        assert!(cr.checkpoint_sequence() > 0);

        // Operations should fail
        let result = cr.add_member(ResonatorId::new("new"), vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_full_lifecycle() {
        let mut cr = make_collective();

        // Register capability + role
        let cap = Capability::new(
            "Process Order",
            "Process customer orders",
            ActionType::Execute,
        )
        .with_id(CapabilityId::new("process-order"));
        cr.register_capability(cap);

        let role = Role::new("OrderProcessor", "Processes orders")
            .with_id(RoleId::new("processor"))
            .with_capability(CapabilityId::new("process-order"));
        cr.register_role(role);

        // Add members
        cr.add_member(
            ResonatorId::new("processor-1"),
            vec![RoleId::new("processor")],
        )
        .unwrap();
        cr.add_member(
            ResonatorId::new("processor-2"),
            vec![RoleId::new("processor")],
        )
        .unwrap();
        cr.add_member(ResonatorId::new("auditor-1"), vec![RoleId::new("auditor")])
            .unwrap();

        // Fund treasury
        let op_id = AccountId::new("operating");
        cr.treasury_deposit(
            &op_id,
            Amount::new(500_000),
            &ResonatorId::new("processor-1"),
        )
        .unwrap();

        // Create threshold for high-value operations
        let tc_id = cr
            .create_threshold_commitment(
                "Approve large order",
                ThresholdPolicy::m_of_n(2, 3),
                Some(100_000),
                None,
                vec![],
            )
            .unwrap();

        // Collect signatures
        cr.sign_threshold_commitment(
            &tc_id,
            CommitmentSignature::new(ResonatorId::new("processor-1")),
        )
        .unwrap();
        cr.sign_threshold_commitment(
            &tc_id,
            CommitmentSignature::new(ResonatorId::new("processor-2")),
        )
        .unwrap();

        assert!(cr.is_threshold_satisfied(&tc_id));

        // Checkpoint
        cr.checkpoint();
        assert!(cr.verify_continuity());

        // Many receipts should have been generated
        assert!(cr.receipt_count() > 5);

        // Summary
        assert_eq!(cr.active_member_count(), 3);
        assert_eq!(cr.treasury_balance(), Amount::new(500_000));
    }
}
