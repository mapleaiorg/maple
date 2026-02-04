//! Membership graph: who belongs to a Collective
//!
//! The membership graph tracks all members, their roles, statuses,
//! and budgets. It is the source of truth for "who's in".

use crate::{CollectiveError, CollectiveId, CollectiveResult, RoleId};
use chrono::{DateTime, Utc};
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A record for a single member of a Collective
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemberRecord {
    /// The resonator's identity
    pub resonator_id: ResonatorId,
    /// Roles assigned to this member
    pub roles: Vec<RoleId>,
    /// When the member joined
    pub joined_at: DateTime<Utc>,
    /// Current membership status
    pub status: MemberStatus,
    /// Resource budgets allocated to this member
    pub budgets: MemberBudgets,
}

impl MemberRecord {
    /// Create a new member record
    pub fn new(resonator_id: ResonatorId) -> Self {
        Self {
            resonator_id,
            roles: Vec::new(),
            joined_at: Utc::now(),
            status: MemberStatus::Active,
            budgets: MemberBudgets::default(),
        }
    }

    pub fn with_role(mut self, role: RoleId) -> Self {
        self.roles.push(role);
        self
    }

    pub fn with_budgets(mut self, budgets: MemberBudgets) -> Self {
        self.budgets = budgets;
        self
    }

    /// Check if the member is currently active
    pub fn is_active(&self) -> bool {
        matches!(self.status, MemberStatus::Active)
    }

    /// Check if the member has a specific role
    pub fn has_role(&self, role_id: &RoleId) -> bool {
        self.roles.contains(role_id)
    }

    /// Add a role to this member
    pub fn add_role(&mut self, role_id: RoleId) {
        if !self.roles.contains(&role_id) {
            self.roles.push(role_id);
        }
    }

    /// Remove a role from this member
    pub fn remove_role(&mut self, role_id: &RoleId) {
        self.roles.retain(|r| r != role_id);
    }
}

/// Status of a member within a Collective
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MemberStatus {
    /// Member is active and participating
    #[default]
    Active,
    /// Member is temporarily suspended
    Suspended,
    /// Member was expelled by the collective
    Expelled,
    /// Member left voluntarily
    Left,
}

/// Budget allocation for a member
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct MemberBudgets {
    /// Attention units allocated
    pub attention: u64,
    /// Financial budget allocated
    pub financial: u64,
    /// Coupling slots allocated
    pub coupling_slots: u32,
    /// Workflow initiation quota
    pub workflow_quota: u32,
}

impl MemberBudgets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_attention(mut self, attention: u64) -> Self {
        self.attention = attention;
        self
    }

    pub fn with_financial(mut self, financial: u64) -> Self {
        self.financial = financial;
        self
    }

    pub fn with_coupling_slots(mut self, slots: u32) -> Self {
        self.coupling_slots = slots;
        self
    }

    pub fn with_workflow_quota(mut self, quota: u32) -> Self {
        self.workflow_quota = quota;
        self
    }
}

/// The membership graph for a Collective
///
/// This is the canonical "who's in" data structure.
/// It does NOT make decisions—it stores membership state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MembershipGraph {
    /// The collective this graph belongs to
    pub collective_id: CollectiveId,
    /// All member records, keyed by resonator ID
    pub members: HashMap<ResonatorId, MemberRecord>,
}

impl MembershipGraph {
    /// Create a new empty membership graph
    pub fn new(collective_id: CollectiveId) -> Self {
        Self {
            collective_id,
            members: HashMap::new(),
        }
    }

    /// Add a new member to the collective
    pub fn add_member(&mut self, record: MemberRecord) -> CollectiveResult<()> {
        if self.members.contains_key(&record.resonator_id) {
            return Err(CollectiveError::MemberAlreadyExists(
                record.resonator_id.clone(),
            ));
        }
        self.members.insert(record.resonator_id.clone(), record);
        Ok(())
    }

    /// Remove a member from the collective (sets status to Left)
    pub fn remove_member(&mut self, resonator_id: &ResonatorId) -> CollectiveResult<()> {
        let member = self
            .members
            .get_mut(resonator_id)
            .ok_or_else(|| CollectiveError::MemberNotFound(resonator_id.clone()))?;
        member.status = MemberStatus::Left;
        Ok(())
    }

    /// Suspend a member
    pub fn suspend_member(&mut self, resonator_id: &ResonatorId) -> CollectiveResult<()> {
        let member = self
            .members
            .get_mut(resonator_id)
            .ok_or_else(|| CollectiveError::MemberNotFound(resonator_id.clone()))?;
        member.status = MemberStatus::Suspended;
        Ok(())
    }

    /// Expel a member
    pub fn expel_member(&mut self, resonator_id: &ResonatorId) -> CollectiveResult<()> {
        let member = self
            .members
            .get_mut(resonator_id)
            .ok_or_else(|| CollectiveError::MemberNotFound(resonator_id.clone()))?;
        member.status = MemberStatus::Expelled;
        Ok(())
    }

    /// Get a member record
    pub fn get_member(&self, resonator_id: &ResonatorId) -> Option<&MemberRecord> {
        self.members.get(resonator_id)
    }

    /// Get a mutable member record
    pub fn get_member_mut(&mut self, resonator_id: &ResonatorId) -> Option<&mut MemberRecord> {
        self.members.get_mut(resonator_id)
    }

    /// Check if a resonator is a member (any status)
    pub fn is_member(&self, resonator_id: &ResonatorId) -> bool {
        self.members.contains_key(resonator_id)
    }

    /// Check if a resonator is an active member
    pub fn is_active_member(&self, resonator_id: &ResonatorId) -> bool {
        self.members
            .get(resonator_id)
            .map(|m| m.is_active())
            .unwrap_or(false)
    }

    /// Get all active members
    pub fn active_members(&self) -> Vec<&MemberRecord> {
        self.members.values().filter(|m| m.is_active()).collect()
    }

    /// Get all members with a specific role (active only)
    pub fn members_with_role(&self, role_id: &RoleId) -> Vec<&MemberRecord> {
        self.members
            .values()
            .filter(|m| m.is_active() && m.has_role(role_id))
            .collect()
    }

    /// Total number of members (all statuses)
    pub fn total_members(&self) -> usize {
        self.members.len()
    }

    /// Number of active members
    pub fn active_member_count(&self) -> usize {
        self.members.values().filter(|m| m.is_active()).count()
    }

    /// Get all resonator IDs of active members
    pub fn active_resonator_ids(&self) -> Vec<ResonatorId> {
        self.members
            .values()
            .filter(|m| m.is_active())
            .map(|m| m.resonator_id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_graph() -> MembershipGraph {
        MembershipGraph::new(CollectiveId::new("test-collective"))
    }

    #[test]
    fn test_add_member() {
        let mut graph = make_graph();
        let record = MemberRecord::new(ResonatorId::new("res-1"));
        graph.add_member(record).unwrap();

        assert!(graph.is_member(&ResonatorId::new("res-1")));
        assert!(graph.is_active_member(&ResonatorId::new("res-1")));
        assert_eq!(graph.total_members(), 1);
    }

    #[test]
    fn test_duplicate_member() {
        let mut graph = make_graph();
        graph
            .add_member(MemberRecord::new(ResonatorId::new("res-1")))
            .unwrap();
        let result = graph.add_member(MemberRecord::new(ResonatorId::new("res-1")));
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_member() {
        let mut graph = make_graph();
        graph
            .add_member(MemberRecord::new(ResonatorId::new("res-1")))
            .unwrap();
        graph.remove_member(&ResonatorId::new("res-1")).unwrap();

        assert!(graph.is_member(&ResonatorId::new("res-1")));
        assert!(!graph.is_active_member(&ResonatorId::new("res-1")));
        assert_eq!(graph.active_member_count(), 0);
    }

    #[test]
    fn test_members_with_role() {
        let mut graph = make_graph();
        let admin_role = RoleId::new("admin");
        let viewer_role = RoleId::new("viewer");

        graph
            .add_member(
                MemberRecord::new(ResonatorId::new("res-1"))
                    .with_role(admin_role.clone())
                    .with_role(viewer_role.clone()),
            )
            .unwrap();
        graph
            .add_member(MemberRecord::new(ResonatorId::new("res-2")).with_role(viewer_role.clone()))
            .unwrap();
        graph
            .add_member(MemberRecord::new(ResonatorId::new("res-3")))
            .unwrap();

        assert_eq!(graph.members_with_role(&admin_role).len(), 1);
        assert_eq!(graph.members_with_role(&viewer_role).len(), 2);
        assert_eq!(graph.active_member_count(), 3);
    }

    #[test]
    fn test_suspend_and_expel() {
        let mut graph = make_graph();
        graph
            .add_member(MemberRecord::new(ResonatorId::new("res-1")))
            .unwrap();
        graph
            .add_member(MemberRecord::new(ResonatorId::new("res-2")))
            .unwrap();

        graph.suspend_member(&ResonatorId::new("res-1")).unwrap();
        assert!(!graph.is_active_member(&ResonatorId::new("res-1")));
        assert_eq!(graph.active_member_count(), 1);

        graph.expel_member(&ResonatorId::new("res-2")).unwrap();
        assert!(!graph.is_active_member(&ResonatorId::new("res-2")));
        assert_eq!(graph.active_member_count(), 0);
    }

    #[test]
    fn test_member_role_management() {
        let mut record = MemberRecord::new(ResonatorId::new("res-1"));
        let role = RoleId::new("admin");

        assert!(!record.has_role(&role));
        record.add_role(role.clone());
        assert!(record.has_role(&role));

        // Adding same role again is idempotent
        record.add_role(role.clone());
        assert_eq!(record.roles.len(), 1);

        record.remove_role(&role);
        assert!(!record.has_role(&role));
    }
}
