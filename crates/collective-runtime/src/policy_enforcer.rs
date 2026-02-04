//! Policy Enforcer — validates actions against collective policies
//!
//! The policy enforcer is the "guardian" of the Collective. Before any
//! action proceeds, it checks membership, budget, role constraints,
//! and collective status. Safety always overrides optimization (Invariant 6).

use collective_types::{CollectiveStatus, MemberBudgets, MemberRecord, RoleConstraintType, RoleId};
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};
use tracing::warn;

/// Policy check result
#[derive(Clone, Debug)]
pub enum PolicyDecision {
    /// Action is allowed
    Allow,
    /// Action is denied with reason
    Deny(String),
    /// Action requires additional approval (threshold commitment)
    RequiresApproval {
        reason: String,
        required_role: Option<RoleId>,
    },
}

impl PolicyDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, PolicyDecision::Allow)
    }

    pub fn is_denied(&self) -> bool {
        matches!(self, PolicyDecision::Deny(_))
    }
}

/// Configuration for policy enforcement
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Maximum value an action can have before requiring approval
    pub approval_threshold: u64,
    /// Whether to enforce budget limits
    pub enforce_budgets: bool,
    /// Whether to enforce role constraints (time windows, etc.)
    pub enforce_role_constraints: bool,
    /// Whether to enforce domain restrictions
    pub enforce_domain_restrictions: bool,
    /// Maximum concurrent actions per member
    pub max_concurrent_actions: u32,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            approval_threshold: 10_000,
            enforce_budgets: true,
            enforce_role_constraints: true,
            enforce_domain_restrictions: true,
            max_concurrent_actions: 10,
        }
    }
}

/// A request to check policy
#[derive(Clone, Debug)]
pub struct PolicyCheckRequest {
    /// Who is requesting the action
    pub actor: ResonatorId,
    /// What role they're acting under
    pub role: RoleId,
    /// Estimated value of the action
    pub estimated_value: Option<u64>,
    /// Estimated attention cost
    pub attention_cost: Option<u64>,
    /// Effect domain
    pub domain: Option<String>,
    /// Current hour (for time-window checks)
    pub current_hour: Option<u8>,
}

/// Policy Enforcer — the guardian of collective rules
pub struct PolicyEnforcer {
    /// Policy configuration
    config: PolicyConfig,
    /// Collective status (must be active for most operations)
    collective_status: CollectiveStatus,
}

impl PolicyEnforcer {
    pub fn new(config: PolicyConfig) -> Self {
        Self {
            config,
            collective_status: CollectiveStatus::Active,
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(PolicyConfig::default())
    }

    /// Update collective status
    pub fn set_collective_status(&mut self, status: CollectiveStatus) {
        self.collective_status = status;
    }

    /// Full policy check for an action
    pub fn check_policy(
        &self,
        request: &PolicyCheckRequest,
        member: &MemberRecord,
        role_constraints: &[collective_types::RoleConstraint],
    ) -> PolicyDecision {
        // Check 1: Collective must be active (Invariant: failure must be explicit)
        if !self.collective_status.is_active() {
            return PolicyDecision::Deny("Collective is not active".into());
        }

        // Check 2: Member must be active
        if !member.is_active() {
            return PolicyDecision::Deny(format!(
                "Member {:?} is not active (status: {:?})",
                request.actor, member.status
            ));
        }

        // Check 3: Member must have the claimed role
        if !member.has_role(&request.role) {
            return PolicyDecision::Deny(format!(
                "Member {} does not have role {}",
                request.actor, request.role
            ));
        }

        // Check 4: Budget limits
        if self.config.enforce_budgets {
            if let Some(decision) = self.check_budget_limits(request, &member.budgets) {
                return decision;
            }
        }

        // Check 5: Role constraints
        if self.config.enforce_role_constraints {
            if let Some(decision) = self.check_role_constraints(request, role_constraints) {
                return decision;
            }
        }

        // Check 6: Value threshold (may require additional approval)
        if let Some(value) = request.estimated_value {
            if value > self.config.approval_threshold {
                return PolicyDecision::RequiresApproval {
                    reason: format!(
                        "Action value {} exceeds approval threshold {}",
                        value, self.config.approval_threshold
                    ),
                    required_role: None,
                };
            }
        }

        PolicyDecision::Allow
    }

    /// Check budget limits
    fn check_budget_limits(
        &self,
        request: &PolicyCheckRequest,
        budgets: &MemberBudgets,
    ) -> Option<PolicyDecision> {
        // Check attention budget
        if let Some(attention_cost) = request.attention_cost {
            if attention_cost > budgets.attention {
                warn!(
                    actor = %request.actor,
                    required = attention_cost,
                    available = budgets.attention,
                    "Attention budget exceeded"
                );
                return Some(PolicyDecision::Deny(format!(
                    "Attention budget exceeded: need {}, have {}",
                    attention_cost, budgets.attention
                )));
            }
        }

        // Check financial budget
        if let Some(value) = request.estimated_value {
            if value > budgets.financial {
                warn!(
                    actor = %request.actor,
                    required = value,
                    available = budgets.financial,
                    "Financial budget exceeded"
                );
                return Some(PolicyDecision::Deny(format!(
                    "Financial budget exceeded: need {}, have {}",
                    value, budgets.financial
                )));
            }
        }

        None
    }

    /// Check role constraints
    fn check_role_constraints(
        &self,
        request: &PolicyCheckRequest,
        constraints: &[collective_types::RoleConstraint],
    ) -> Option<PolicyDecision> {
        for constraint in constraints {
            match &constraint.constraint_type {
                RoleConstraintType::TimeWindow {
                    start_hour,
                    end_hour,
                } => {
                    if let Some(current_hour) = request.current_hour {
                        let in_window = if start_hour <= end_hour {
                            current_hour >= *start_hour && current_hour < *end_hour
                        } else {
                            // Wraps midnight (e.g., 22-06)
                            current_hour >= *start_hour || current_hour < *end_hour
                        };

                        if !in_window {
                            return Some(PolicyDecision::Deny(format!(
                                "Outside time window: {} (allowed {}:00-{}:00)",
                                constraint.description, start_hour, end_hour
                            )));
                        }
                    }
                }

                RoleConstraintType::DomainRestriction(allowed_domains) => {
                    if self.config.enforce_domain_restrictions {
                        if let Some(domain) = &request.domain {
                            if !allowed_domains.contains(domain) {
                                return Some(PolicyDecision::Deny(format!(
                                    "Domain '{}' not allowed for role (allowed: {:?})",
                                    domain, allowed_domains
                                )));
                            }
                        }
                    }
                }

                RoleConstraintType::MaxConcurrentActions(_max) => {
                    // This would need runtime state to check
                    // For now, the policy enforcer only validates static constraints
                }

                RoleConstraintType::Custom(_) => {
                    // Custom constraints are application-specific
                }
            }
        }

        None
    }

    /// Quick check: is the collective operational?
    pub fn is_operational(&self) -> bool {
        self.collective_status.is_active()
    }

    /// Get current config
    pub fn config(&self) -> &PolicyConfig {
        &self.config
    }

    /// Update config
    pub fn set_config(&mut self, config: PolicyConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collective_types::{MemberRecord, RoleConstraint};

    fn make_member(id: &str, role: &str) -> MemberRecord {
        MemberRecord::new(ResonatorId::new(id)).with_role(RoleId::new(role))
    }

    fn make_request(actor: &str, role: &str) -> PolicyCheckRequest {
        PolicyCheckRequest {
            actor: ResonatorId::new(actor),
            role: RoleId::new(role),
            estimated_value: None,
            attention_cost: None,
            domain: None,
            current_hour: None,
        }
    }

    #[test]
    fn test_allow_basic_action() {
        let enforcer = PolicyEnforcer::with_default_config();
        let member = make_member("res-1", "trader");
        let request = make_request("res-1", "trader");

        let decision = enforcer.check_policy(&request, &member, &[]);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_deny_inactive_collective() {
        let mut enforcer = PolicyEnforcer::with_default_config();
        enforcer.set_collective_status(CollectiveStatus::Suspended);

        let member = make_member("res-1", "trader");
        let request = make_request("res-1", "trader");

        let decision = enforcer.check_policy(&request, &member, &[]);
        assert!(decision.is_denied());
    }

    #[test]
    fn test_deny_wrong_role() {
        let enforcer = PolicyEnforcer::with_default_config();
        let member = make_member("res-1", "viewer");
        let request = make_request("res-1", "admin"); // Claiming admin but only has viewer

        let decision = enforcer.check_policy(&request, &member, &[]);
        assert!(decision.is_denied());
    }

    #[test]
    fn test_deny_budget_exceeded() {
        let enforcer = PolicyEnforcer::with_default_config();
        let member = MemberRecord::new(ResonatorId::new("res-1"))
            .with_role(RoleId::new("trader"))
            .with_budgets(MemberBudgets::new().with_financial(1000));

        let mut request = make_request("res-1", "trader");
        request.estimated_value = Some(5000); // Over budget

        let decision = enforcer.check_policy(&request, &member, &[]);
        assert!(decision.is_denied());
    }

    #[test]
    fn test_requires_approval_over_threshold() {
        let enforcer = PolicyEnforcer::new(PolicyConfig {
            approval_threshold: 1000,
            ..PolicyConfig::default()
        });

        let member = MemberRecord::new(ResonatorId::new("res-1"))
            .with_role(RoleId::new("trader"))
            .with_budgets(MemberBudgets::new().with_financial(100_000));

        let mut request = make_request("res-1", "trader");
        request.estimated_value = Some(5000); // Over threshold but within budget

        let decision = enforcer.check_policy(&request, &member, &[]);
        assert!(matches!(decision, PolicyDecision::RequiresApproval { .. }));
    }

    #[test]
    fn test_time_window_constraint() {
        let enforcer = PolicyEnforcer::with_default_config();
        let member = make_member("res-1", "trader");

        let constraint = RoleConstraint::new(
            RoleConstraintType::TimeWindow {
                start_hour: 9,
                end_hour: 17,
            },
            "Trading hours only",
        );

        // During hours
        let mut request = make_request("res-1", "trader");
        request.current_hour = Some(12);
        let decision = enforcer.check_policy(&request, &member, &[constraint.clone()]);
        assert!(decision.is_allowed());

        // Outside hours
        request.current_hour = Some(3);
        let decision = enforcer.check_policy(&request, &member, &[constraint]);
        assert!(decision.is_denied());
    }

    #[test]
    fn test_domain_restriction() {
        let enforcer = PolicyEnforcer::with_default_config();
        let member = make_member("res-1", "trader");

        let constraint = RoleConstraint::new(
            RoleConstraintType::DomainRestriction(vec!["finance".into()]),
            "Finance only",
        );

        // Allowed domain
        let mut request = make_request("res-1", "trader");
        request.domain = Some("finance".into());
        let decision = enforcer.check_policy(&request, &member, &[constraint.clone()]);
        assert!(decision.is_allowed());

        // Denied domain
        request.domain = Some("infrastructure".into());
        let decision = enforcer.check_policy(&request, &member, &[constraint]);
        assert!(decision.is_denied());
    }
}
