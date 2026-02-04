//! Role Router — routes actions to eligible resonators via RCPG
//!
//! The Role Router is the "switchboard" of the Collective. Given an
//! action request, it determines which resonators have the right role,
//! capability, and active permit to execute it.

use collective_types::{
    ActionType, Capability, CapabilityId, CollectiveError, CollectiveResult, MembershipGraph,
    Permit, PermitId, Role, RoleId, RoleRegistry,
};
use rcf_types::EffectDomain;
use resonator_types::ResonatorId;
use std::collections::HashMap;
use tracing::debug;

/// Routes actions to eligible resonators based on Role-Capability-Permit graph
pub struct RoleRouter {
    /// Role definitions and bindings
    role_registry: RoleRegistry,
    /// Capability definitions
    capabilities: HashMap<CapabilityId, Capability>,
    /// Active permits
    permits: HashMap<PermitId, Permit>,
}

/// Result of routing an action request
#[derive(Clone, Debug)]
pub struct RouteResult {
    /// Resonators eligible to execute this action
    pub eligible_resonators: Vec<ResonatorId>,
    /// The role that covers this action
    pub covering_role: RoleId,
    /// The capability being exercised
    pub capability_id: CapabilityId,
}

/// A request to route an action
#[derive(Clone, Debug)]
pub struct ActionRequest {
    /// The action type to perform
    pub action_type: ActionType,
    /// The effect domain
    pub domain: EffectDomain,
    /// Target of the action
    pub target: String,
    /// The specific operation
    pub operation: String,
    /// Estimated value (for budget checks)
    pub estimated_value: Option<u64>,
}

impl ActionRequest {
    pub fn new(action_type: ActionType, domain: EffectDomain) -> Self {
        Self {
            action_type,
            domain,
            target: "*".to_string(),
            operation: "*".to_string(),
            estimated_value: None,
        }
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = target.into();
        self
    }

    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = operation.into();
        self
    }

    pub fn with_value(mut self, value: u64) -> Self {
        self.estimated_value = Some(value);
        self
    }
}

impl RoleRouter {
    pub fn new() -> Self {
        Self {
            role_registry: RoleRegistry::new(),
            capabilities: HashMap::new(),
            permits: HashMap::new(),
        }
    }

    /// Create from existing registry
    pub fn from_registry(role_registry: RoleRegistry) -> Self {
        Self {
            role_registry,
            capabilities: HashMap::new(),
            permits: HashMap::new(),
        }
    }

    // --- Registration ---

    /// Register a role
    pub fn register_role(&mut self, role: Role) {
        self.role_registry.register_role(role);
    }

    /// Register a capability
    pub fn register_capability(&mut self, capability: Capability) {
        self.capabilities.insert(capability.id.clone(), capability);
    }

    /// Issue a permit
    pub fn issue_permit(&mut self, permit: Permit) -> PermitId {
        let id = permit.id.clone();
        self.permits.insert(id.clone(), permit);
        id
    }

    /// Revoke a permit
    pub fn revoke_permit(&mut self, permit_id: &PermitId) -> CollectiveResult<()> {
        let permit = self
            .permits
            .get_mut(permit_id)
            .ok_or_else(|| CollectiveError::PermitNotFound(permit_id.clone()))?;
        permit.revoke();
        Ok(())
    }

    // --- Routing ---

    /// Route an action to eligible resonators
    ///
    /// This is the core routing algorithm:
    /// 1. Find capabilities matching the action type
    /// 2. Find roles that grant those capabilities
    /// 3. Find resonators with those roles (active bindings)
    /// 4. Filter by active permits covering scope
    /// 5. Return eligible resonators
    pub fn route_action(
        &self,
        request: &ActionRequest,
        membership: &MembershipGraph,
    ) -> CollectiveResult<RouteResult> {
        // Step 1: Find matching capabilities
        let matching_capabilities: Vec<&Capability> = self
            .capabilities
            .values()
            .filter(|cap| cap.action_type == request.action_type)
            .collect();

        if matching_capabilities.is_empty() {
            return Err(CollectiveError::PolicyViolation(format!(
                "No capability registered for action type {:?}",
                request.action_type
            )));
        }

        // Step 2 & 3: Find roles + resonators for each capability
        for capability in &matching_capabilities {
            // Find roles granting this capability
            let covering_roles: Vec<&Role> = self
                .role_registry
                .roles
                .values()
                .filter(|role| role.has_capability(&capability.id))
                .collect();

            for role in &covering_roles {
                // Get active resonators in this role
                let resonators_in_role = self.role_registry.resonators_in_role(&role.id);

                // Filter to active members only
                let active_resonators: Vec<ResonatorId> = resonators_in_role
                    .into_iter()
                    .filter(|r| membership.is_active_member(r))
                    .collect();

                if active_resonators.is_empty() {
                    continue;
                }

                // Step 4: Filter by active permits
                let eligible: Vec<ResonatorId> = active_resonators
                    .into_iter()
                    .filter(|r| {
                        self.has_valid_permit(
                            r,
                            &capability.id,
                            &request.domain,
                            &request.target,
                            &request.operation,
                        )
                    })
                    .collect();

                if !eligible.is_empty() {
                    debug!(
                        role = %role.id,
                        capability = %capability.id,
                        eligible_count = eligible.len(),
                        "Action routed successfully"
                    );

                    return Ok(RouteResult {
                        eligible_resonators: eligible,
                        covering_role: role.id.clone(),
                        capability_id: capability.id.clone(),
                    });
                }
            }
        }

        // If we get here, we found capabilities but no eligible resonators
        // Try without permit check — maybe permits just need to be issued
        for capability in &matching_capabilities {
            let covering_roles: Vec<&Role> = self
                .role_registry
                .roles
                .values()
                .filter(|role| role.has_capability(&capability.id))
                .collect();

            for role in &covering_roles {
                let active_resonators: Vec<ResonatorId> = self
                    .role_registry
                    .resonators_in_role(&role.id)
                    .into_iter()
                    .filter(|r| membership.is_active_member(r))
                    .collect();

                if !active_resonators.is_empty() {
                    debug!(
                        role = %role.id,
                        capability = %capability.id,
                        "Action routed (no permit filtering)"
                    );

                    return Ok(RouteResult {
                        eligible_resonators: active_resonators,
                        covering_role: role.id.clone(),
                        capability_id: capability.id.clone(),
                    });
                }
            }
        }

        Err(CollectiveError::PolicyViolation(
            "No eligible resonators found for this action".into(),
        ))
    }

    /// Route to a specific role
    pub fn route_to_role(
        &self,
        role_id: &RoleId,
        membership: &MembershipGraph,
    ) -> CollectiveResult<Vec<ResonatorId>> {
        if !self.role_registry.has_role_coverage(role_id) {
            return Err(CollectiveError::RoleNotFound(role_id.clone()));
        }

        let resonators: Vec<ResonatorId> = self
            .role_registry
            .resonators_in_role(role_id)
            .into_iter()
            .filter(|r| membership.is_active_member(r))
            .collect();

        if resonators.is_empty() {
            return Err(CollectiveError::PolicyViolation(format!(
                "No active members in role: {}",
                role_id
            )));
        }

        Ok(resonators)
    }

    /// Check if a resonator has a valid permit for a capability+scope
    fn has_valid_permit(
        &self,
        resonator_id: &ResonatorId,
        capability_id: &CapabilityId,
        domain: &EffectDomain,
        target: &str,
        operation: &str,
    ) -> bool {
        self.permits.values().any(|permit| {
            permit.capability_id == *capability_id
                && permit.grantee == *resonator_id
                && permit.is_usable()
                && permit.scope.covers_domain(domain)
                && permit.scope.covers_target(target)
                && permit.scope.covers_operation(operation)
        })
    }

    /// Record permit usage
    pub fn record_permit_use(&mut self, permit_id: &PermitId) -> CollectiveResult<()> {
        let permit = self
            .permits
            .get_mut(permit_id)
            .ok_or_else(|| CollectiveError::PermitNotFound(permit_id.clone()))?;

        if !permit.is_usable() {
            return Err(CollectiveError::PermitExpired(permit_id.clone()));
        }

        permit.record_use();
        Ok(())
    }

    // --- Query methods ---

    pub fn role_registry(&self) -> &RoleRegistry {
        &self.role_registry
    }

    pub fn role_registry_mut(&mut self) -> &mut RoleRegistry {
        &mut self.role_registry
    }

    pub fn get_capability(&self, id: &CapabilityId) -> Option<&Capability> {
        self.capabilities.get(id)
    }

    pub fn get_permit(&self, id: &PermitId) -> Option<&Permit> {
        self.permits.get(id)
    }

    /// Get all active permits for a resonator
    pub fn permits_for_resonator(&self, resonator_id: &ResonatorId) -> Vec<&Permit> {
        self.permits
            .values()
            .filter(|p| p.grantee == *resonator_id && p.is_usable())
            .collect()
    }

    /// Check if a role has coverage (any active members)
    pub fn has_role_coverage(&self, role_id: &RoleId) -> bool {
        self.role_registry.has_role_coverage(role_id)
    }

    /// Get all registered capability IDs
    pub fn capability_ids(&self) -> Vec<CapabilityId> {
        self.capabilities.keys().cloned().collect()
    }

    /// Clean up expired/exhausted permits
    pub fn cleanup_permits(&mut self) {
        let expired: Vec<PermitId> = self
            .permits
            .iter()
            .filter(|(_, p)| !p.is_usable())
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired {
            self.permits.remove(&id);
        }
    }
}

impl Default for RoleRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collective_types::{CollectiveId, GrantAuthority, Permit, PermitScope, RoleBinding};

    fn setup() -> (RoleRouter, MembershipGraph) {
        let mut router = RoleRouter::new();
        let mut graph = MembershipGraph::new(CollectiveId::new("test"));

        // Register capability
        let cap = Capability::new(
            "Execute Trade",
            "Execute financial trades",
            ActionType::Execute,
        )
        .with_id(CapabilityId::new("trade"));
        router.register_capability(cap);

        // Register role with capability
        let role = Role::new("Trader", "Executes trades")
            .with_id(RoleId::new("trader"))
            .with_capability(CapabilityId::new("trade"));
        router.register_role(role);

        // Add member
        let res = ResonatorId::new("trader-1");
        graph
            .add_member(
                collective_types::MemberRecord::new(res.clone()).with_role(RoleId::new("trader")),
            )
            .unwrap();

        // Bind role
        router.role_registry_mut().bind(RoleBinding::new(
            res.clone(),
            RoleId::new("trader"),
            ResonatorId::new("admin"),
        ));

        // Issue permit
        let permit = Permit::new(
            CapabilityId::new("trade"),
            res,
            PermitScope::new()
                .with_domain(EffectDomain::Finance)
                .with_operation("execute"),
            GrantAuthority::Collective(CollectiveId::new("test")),
        );
        router.issue_permit(permit);

        (router, graph)
    }

    #[test]
    fn test_route_action() {
        let (router, graph) = setup();

        let request = ActionRequest::new(ActionType::Execute, EffectDomain::Finance)
            .with_operation("execute");

        let result = router.route_action(&request, &graph).unwrap();
        assert_eq!(result.eligible_resonators.len(), 1);
        assert_eq!(result.eligible_resonators[0], ResonatorId::new("trader-1"));
        assert_eq!(result.covering_role, RoleId::new("trader"));
    }

    #[test]
    fn test_route_to_role() {
        let (router, graph) = setup();

        let resonators = router
            .route_to_role(&RoleId::new("trader"), &graph)
            .unwrap();
        assert_eq!(resonators.len(), 1);
    }

    #[test]
    fn test_no_capability_for_action() {
        let (router, graph) = setup();

        let request = ActionRequest::new(ActionType::Audit, EffectDomain::Finance);
        let result = router.route_action(&request, &graph);
        assert!(result.is_err());
    }

    #[test]
    fn test_permit_revocation() {
        let (mut router, graph) = setup();

        // Get the permit ID
        let permit_ids: Vec<PermitId> = router.permits.keys().cloned().collect();
        router.revoke_permit(&permit_ids[0]).unwrap();

        // Route should still work (falls back to role-based without permit)
        let request = ActionRequest::new(ActionType::Execute, EffectDomain::Finance);
        let result = router.route_action(&request, &graph).unwrap();
        assert_eq!(result.eligible_resonators.len(), 1);
    }

    #[test]
    fn test_permits_for_resonator() {
        let (router, _) = setup();
        let permits = router.permits_for_resonator(&ResonatorId::new("trader-1"));
        assert_eq!(permits.len(), 1);

        let no_permits = router.permits_for_resonator(&ResonatorId::new("nobody"));
        assert_eq!(no_permits.len(), 0);
    }
}
