//! Role–Capability–Permit Graph: Role definitions and bindings
//!
//! Roles define abstract responsibilities within a Collective.
//! They are bound to Resonators and grant access to Capabilities.

use crate::CapabilityId;
use chrono::{DateTime, Utc};
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a Role
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoleId(pub String);

impl RoleId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for RoleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A Role defines an abstract responsibility within a Collective
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Role {
    /// Unique role identifier
    pub id: RoleId,
    /// Human-readable role name
    pub name: String,
    /// Description of the role's purpose
    pub description: String,
    /// Capabilities this role grants access to
    pub capabilities: Vec<CapabilityId>,
    /// Budget limits for this role
    pub budgets: RoleBudgets,
    /// Constraints on role behavior
    pub constraints: Vec<RoleConstraint>,
}

impl Role {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: RoleId::generate(),
            name: name.into(),
            description: description.into(),
            capabilities: Vec::new(),
            budgets: RoleBudgets::default(),
            constraints: Vec::new(),
        }
    }

    pub fn with_id(mut self, id: RoleId) -> Self {
        self.id = id;
        self
    }

    pub fn with_capability(mut self, capability: CapabilityId) -> Self {
        self.capabilities.push(capability);
        self
    }

    pub fn with_budgets(mut self, budgets: RoleBudgets) -> Self {
        self.budgets = budgets;
        self
    }

    pub fn with_constraint(mut self, constraint: RoleConstraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Check if this role grants a specific capability
    pub fn has_capability(&self, capability: &CapabilityId) -> bool {
        self.capabilities.contains(capability)
    }
}

/// Budget limits associated with a Role
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RoleBudgets {
    /// Maximum attention units this role can consume
    pub max_attention: u64,
    /// Maximum financial amount this role can spend
    pub max_financial: u64,
    /// Maximum coupling slots this role can use
    pub max_coupling_slots: u32,
    /// Maximum workflows this role can initiate
    pub max_workflow_quota: u32,
}

impl RoleBudgets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_attention(mut self, max: u64) -> Self {
        self.max_attention = max;
        self
    }

    pub fn with_financial(mut self, max: u64) -> Self {
        self.max_financial = max;
        self
    }

    pub fn with_coupling_slots(mut self, max: u32) -> Self {
        self.max_coupling_slots = max;
        self
    }

    pub fn with_workflow_quota(mut self, max: u32) -> Self {
        self.max_workflow_quota = max;
        self
    }
}

/// A constraint on role behavior
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoleConstraint {
    /// Type of constraint
    pub constraint_type: RoleConstraintType,
    /// Human-readable description
    pub description: String,
}

impl RoleConstraint {
    pub fn new(constraint_type: RoleConstraintType, description: impl Into<String>) -> Self {
        Self {
            constraint_type,
            description: description.into(),
        }
    }
}

/// Types of role constraints
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleConstraintType {
    /// Limit concurrent actions
    MaxConcurrentActions(u32),
    /// Restrict to specific effect domains
    DomainRestriction(Vec<String>),
    /// Time window when role is active (hours, 0-23)
    TimeWindow { start_hour: u8, end_hour: u8 },
    /// Custom constraint
    Custom(String),
}

/// A binding between a Resonator and a Role
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoleBinding {
    /// The resonator assigned to this role
    pub resonator_id: ResonatorId,
    /// The role being assigned
    pub role_id: RoleId,
    /// When the binding was created
    pub granted_at: DateTime<Utc>,
    /// Who granted the role
    pub granted_by: ResonatorId,
    /// Optional expiry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl RoleBinding {
    pub fn new(
        resonator_id: ResonatorId,
        role_id: RoleId,
        granted_by: ResonatorId,
    ) -> Self {
        Self {
            resonator_id,
            role_id,
            granted_at: Utc::now(),
            granted_by,
            expires_at: None,
        }
    }

    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Check if the binding is still valid
    pub fn is_active(&self) -> bool {
        match self.expires_at {
            Some(expiry) => Utc::now() < expiry,
            None => true,
        }
    }
}

/// Registry of all roles and their bindings within a Collective
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RoleRegistry {
    /// All defined roles
    pub roles: HashMap<RoleId, Role>,
    /// All role bindings
    pub bindings: Vec<RoleBinding>,
}

impl RoleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new role
    pub fn register_role(&mut self, role: Role) {
        self.roles.insert(role.id.clone(), role);
    }

    /// Get a role by ID
    pub fn get_role(&self, role_id: &RoleId) -> Option<&Role> {
        self.roles.get(role_id)
    }

    /// Bind a resonator to a role
    pub fn bind(&mut self, binding: RoleBinding) {
        self.bindings.push(binding);
    }

    /// Get all active bindings for a resonator
    pub fn bindings_for_resonator(&self, resonator_id: &ResonatorId) -> Vec<&RoleBinding> {
        self.bindings
            .iter()
            .filter(|b| b.resonator_id == *resonator_id && b.is_active())
            .collect()
    }

    /// Get all active bindings for a role
    pub fn bindings_for_role(&self, role_id: &RoleId) -> Vec<&RoleBinding> {
        self.bindings
            .iter()
            .filter(|b| b.role_id == *role_id && b.is_active())
            .collect()
    }

    /// Get all roles held by a resonator (active bindings only)
    pub fn roles_for_resonator(&self, resonator_id: &ResonatorId) -> Vec<&Role> {
        self.bindings_for_resonator(resonator_id)
            .iter()
            .filter_map(|b| self.roles.get(&b.role_id))
            .collect()
    }

    /// Get all resonators in a given role (active bindings only)
    pub fn resonators_in_role(&self, role_id: &RoleId) -> Vec<ResonatorId> {
        self.bindings_for_role(role_id)
            .iter()
            .map(|b| b.resonator_id.clone())
            .collect()
    }

    /// Check if a resonator has a specific role
    pub fn has_role(&self, resonator_id: &ResonatorId, role_id: &RoleId) -> bool {
        self.bindings
            .iter()
            .any(|b| b.resonator_id == *resonator_id && b.role_id == *role_id && b.is_active())
    }

    /// Check if any resonator covers a role
    pub fn has_role_coverage(&self, role_id: &RoleId) -> bool {
        !self.resonators_in_role(role_id).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_id() {
        let id = RoleId::generate();
        assert!(!id.0.is_empty());
        let named = RoleId::new("admin");
        assert_eq!(format!("{}", named), "admin");
    }

    #[test]
    fn test_role_builder() {
        let cap_id = CapabilityId::new("execute-trades");
        let role = Role::new("Trader", "Executes trades")
            .with_capability(cap_id.clone())
            .with_budgets(RoleBudgets::new().with_financial(100_000));

        assert!(role.has_capability(&cap_id));
        assert_eq!(role.budgets.max_financial, 100_000);
    }

    #[test]
    fn test_role_registry() {
        let mut registry = RoleRegistry::new();

        let role = Role::new("Admin", "System administrator")
            .with_id(RoleId::new("admin"));
        registry.register_role(role);

        let resonator = ResonatorId::new("res-1");
        let granter = ResonatorId::new("founder");
        let binding = RoleBinding::new(
            resonator.clone(),
            RoleId::new("admin"),
            granter,
        );
        registry.bind(binding);

        assert!(registry.has_role(&resonator, &RoleId::new("admin")));
        assert!(!registry.has_role(&resonator, &RoleId::new("viewer")));
        assert!(registry.has_role_coverage(&RoleId::new("admin")));
        assert!(!registry.has_role_coverage(&RoleId::new("viewer")));
    }

    #[test]
    fn test_role_binding_active() {
        let binding = RoleBinding::new(
            ResonatorId::new("res-1"),
            RoleId::new("admin"),
            ResonatorId::new("founder"),
        );
        assert!(binding.is_active());

        let expired = RoleBinding::new(
            ResonatorId::new("res-2"),
            RoleId::new("admin"),
            ResonatorId::new("founder"),
        )
        .with_expiry(Utc::now() - chrono::Duration::hours(1));
        assert!(!expired.is_active());
    }
}
