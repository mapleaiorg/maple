//! Membership Manager — lifecycle management for collective members
//!
//! Handles adding, removing, suspending, and expelling members.
//! Enforces membership invariants and emits receipts for all changes.

use collective_types::{
    AuditJournal, CollectiveError, CollectiveId, CollectiveReceipt, CollectiveResult,
    MemberBudgets, MemberRecord, MemberStatus, MembershipGraph, ReceiptType, RoleId,
};
use resonator_types::ResonatorId;
use tracing::{info, warn};

/// Manages membership lifecycle with audit trail
pub struct MembershipManager {
    /// The membership graph (source of truth)
    graph: MembershipGraph,
    /// Default budgets for new members
    default_budgets: MemberBudgets,
}

impl MembershipManager {
    /// Create a new membership manager
    pub fn new(collective_id: CollectiveId) -> Self {
        Self {
            graph: MembershipGraph::new(collective_id),
            default_budgets: MemberBudgets::default(),
        }
    }

    /// Create from an existing membership graph
    pub fn from_graph(graph: MembershipGraph) -> Self {
        Self {
            graph,
            default_budgets: MemberBudgets::default(),
        }
    }

    /// Set default budgets for new members
    pub fn set_default_budgets(&mut self, budgets: MemberBudgets) {
        self.default_budgets = budgets;
    }

    /// Add a new member with default budgets
    pub fn add_member(
        &mut self,
        resonator_id: ResonatorId,
        initial_roles: Vec<RoleId>,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        let record = MemberRecord::new(resonator_id.clone()).with_budgets(self.default_budgets);

        // Add roles
        let mut record = record;
        for role in &initial_roles {
            record.add_role(role.clone());
        }

        self.graph.add_member(record)?;

        info!(
            resonator = %resonator_id,
            roles = ?initial_roles,
            "Member added to collective"
        );

        journal.log_receipt(CollectiveReceipt::new(
            self.graph.collective_id.clone(),
            ReceiptType::Custom("member_added".into()),
            resonator_id,
            format!("Member added with roles: {:?}", initial_roles),
        ));

        Ok(())
    }

    /// Add a member with custom budgets
    pub fn add_member_with_budgets(
        &mut self,
        resonator_id: ResonatorId,
        initial_roles: Vec<RoleId>,
        budgets: MemberBudgets,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        let mut record = MemberRecord::new(resonator_id.clone()).with_budgets(budgets);

        for role in &initial_roles {
            record.add_role(role.clone());
        }

        self.graph.add_member(record)?;

        journal.log_receipt(CollectiveReceipt::new(
            self.graph.collective_id.clone(),
            ReceiptType::Custom("member_added".into()),
            resonator_id,
            "Member added with custom budgets".to_string(),
        ));

        Ok(())
    }

    /// Remove a member (voluntary departure)
    pub fn remove_member(
        &mut self,
        resonator_id: &ResonatorId,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        // Verify member exists and is active
        if !self.graph.is_active_member(resonator_id) {
            return Err(CollectiveError::MemberNotActive(resonator_id.clone()));
        }

        self.graph.remove_member(resonator_id)?;

        info!(resonator = %resonator_id, "Member left collective");

        journal.log_receipt(CollectiveReceipt::new(
            self.graph.collective_id.clone(),
            ReceiptType::Custom("member_left".into()),
            resonator_id.clone(),
            "Member left voluntarily".to_string(),
        ));

        Ok(())
    }

    /// Suspend a member (temporary, reversible)
    pub fn suspend_member(
        &mut self,
        resonator_id: &ResonatorId,
        reason: &str,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        if !self.graph.is_active_member(resonator_id) {
            return Err(CollectiveError::MemberNotActive(resonator_id.clone()));
        }

        self.graph.suspend_member(resonator_id)?;

        warn!(resonator = %resonator_id, reason = reason, "Member suspended");

        journal.log_receipt(CollectiveReceipt::new(
            self.graph.collective_id.clone(),
            ReceiptType::Custom("member_suspended".into()),
            resonator_id.clone(),
            format!("Member suspended: {}", reason),
        ));

        Ok(())
    }

    /// Reinstate a suspended member
    pub fn reinstate_member(
        &mut self,
        resonator_id: &ResonatorId,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        let member = self
            .graph
            .get_member_mut(resonator_id)
            .ok_or_else(|| CollectiveError::MemberNotFound(resonator_id.clone()))?;

        if member.status != MemberStatus::Suspended {
            return Err(CollectiveError::InvalidMembership(format!(
                "Cannot reinstate member with status {:?}",
                member.status
            )));
        }

        member.status = MemberStatus::Active;

        info!(resonator = %resonator_id, "Member reinstated");

        journal.log_receipt(CollectiveReceipt::new(
            self.graph.collective_id.clone(),
            ReceiptType::Custom("member_reinstated".into()),
            resonator_id.clone(),
            "Member reinstated".to_string(),
        ));

        Ok(())
    }

    /// Expel a member (permanent)
    pub fn expel_member(
        &mut self,
        resonator_id: &ResonatorId,
        reason: &str,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        let member = self
            .graph
            .get_member(resonator_id)
            .ok_or_else(|| CollectiveError::MemberNotFound(resonator_id.clone()))?;

        if member.status == MemberStatus::Expelled {
            return Err(CollectiveError::InvalidMembership(
                "Member already expelled".into(),
            ));
        }

        self.graph.expel_member(resonator_id)?;

        warn!(resonator = %resonator_id, reason = reason, "Member expelled");

        journal.log_receipt(CollectiveReceipt::new(
            self.graph.collective_id.clone(),
            ReceiptType::Custom("member_expelled".into()),
            resonator_id.clone(),
            format!("Member expelled: {}", reason),
        ));

        Ok(())
    }

    /// Assign a role to a member
    pub fn assign_role(
        &mut self,
        resonator_id: &ResonatorId,
        role_id: RoleId,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        let member = self
            .graph
            .get_member_mut(resonator_id)
            .ok_or_else(|| CollectiveError::MemberNotFound(resonator_id.clone()))?;

        if !member.is_active() {
            return Err(CollectiveError::MemberNotActive(resonator_id.clone()));
        }

        member.add_role(role_id.clone());

        journal.log_receipt(CollectiveReceipt::new(
            self.graph.collective_id.clone(),
            ReceiptType::Custom("role_assigned".into()),
            resonator_id.clone(),
            format!("Role assigned: {}", role_id),
        ));

        Ok(())
    }

    /// Revoke a role from a member
    pub fn revoke_role(
        &mut self,
        resonator_id: &ResonatorId,
        role_id: &RoleId,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        let member = self
            .graph
            .get_member_mut(resonator_id)
            .ok_or_else(|| CollectiveError::MemberNotFound(resonator_id.clone()))?;

        member.remove_role(role_id);

        journal.log_receipt(CollectiveReceipt::new(
            self.graph.collective_id.clone(),
            ReceiptType::Custom("role_revoked".into()),
            resonator_id.clone(),
            format!("Role revoked: {}", role_id),
        ));

        Ok(())
    }

    /// Update a member's budgets
    pub fn update_budgets(
        &mut self,
        resonator_id: &ResonatorId,
        budgets: MemberBudgets,
    ) -> CollectiveResult<()> {
        let member = self
            .graph
            .get_member_mut(resonator_id)
            .ok_or_else(|| CollectiveError::MemberNotFound(resonator_id.clone()))?;

        member.budgets = budgets;
        Ok(())
    }

    // --- Query methods (delegate to graph) ---

    pub fn graph(&self) -> &MembershipGraph {
        &self.graph
    }

    pub fn is_member(&self, resonator_id: &ResonatorId) -> bool {
        self.graph.is_member(resonator_id)
    }

    pub fn is_active_member(&self, resonator_id: &ResonatorId) -> bool {
        self.graph.is_active_member(resonator_id)
    }

    pub fn get_member(&self, resonator_id: &ResonatorId) -> Option<&MemberRecord> {
        self.graph.get_member(resonator_id)
    }

    pub fn active_member_count(&self) -> usize {
        self.graph.active_member_count()
    }

    pub fn members_with_role(&self, role_id: &RoleId) -> Vec<&MemberRecord> {
        self.graph.members_with_role(role_id)
    }

    pub fn active_resonator_ids(&self) -> Vec<ResonatorId> {
        self.graph.active_resonator_ids()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (MembershipManager, AuditJournal) {
        let id = CollectiveId::new("test-coll");
        let mgr = MembershipManager::new(id.clone());
        let journal = AuditJournal::new(id);
        (mgr, journal)
    }

    #[test]
    fn test_add_and_query_member() {
        let (mut mgr, mut journal) = setup();
        let res = ResonatorId::new("res-1");

        mgr.add_member(res.clone(), vec![RoleId::new("admin")], &mut journal)
            .unwrap();

        assert!(mgr.is_active_member(&res));
        assert_eq!(mgr.active_member_count(), 1);
        assert_eq!(mgr.members_with_role(&RoleId::new("admin")).len(), 1);
        assert_eq!(journal.receipt_count(), 1);
    }

    #[test]
    fn test_suspend_and_reinstate() {
        let (mut mgr, mut journal) = setup();
        let res = ResonatorId::new("res-1");

        mgr.add_member(res.clone(), vec![], &mut journal).unwrap();
        mgr.suspend_member(&res, "bad behavior", &mut journal)
            .unwrap();
        assert!(!mgr.is_active_member(&res));

        mgr.reinstate_member(&res, &mut journal).unwrap();
        assert!(mgr.is_active_member(&res));
        assert_eq!(journal.receipt_count(), 3); // add, suspend, reinstate
    }

    #[test]
    fn test_expel_member() {
        let (mut mgr, mut journal) = setup();
        let res = ResonatorId::new("res-1");

        mgr.add_member(res.clone(), vec![], &mut journal).unwrap();
        mgr.expel_member(&res, "violation", &mut journal).unwrap();
        assert!(!mgr.is_active_member(&res));
        assert!(mgr.is_member(&res)); // Still a member record, just expelled
    }

    #[test]
    fn test_role_assignment() {
        let (mut mgr, mut journal) = setup();
        let res = ResonatorId::new("res-1");

        mgr.add_member(res.clone(), vec![], &mut journal).unwrap();
        mgr.assign_role(&res, RoleId::new("trader"), &mut journal)
            .unwrap();

        assert_eq!(mgr.members_with_role(&RoleId::new("trader")).len(), 1);

        mgr.revoke_role(&res, &RoleId::new("trader"), &mut journal)
            .unwrap();
        assert_eq!(mgr.members_with_role(&RoleId::new("trader")).len(), 0);
    }

    #[test]
    fn test_cannot_suspend_inactive() {
        let (mut mgr, mut journal) = setup();
        let res = ResonatorId::new("res-1");

        mgr.add_member(res.clone(), vec![], &mut journal).unwrap();
        mgr.remove_member(&res, &mut journal).unwrap();

        let result = mgr.suspend_member(&res, "test", &mut journal);
        assert!(result.is_err());
    }
}
