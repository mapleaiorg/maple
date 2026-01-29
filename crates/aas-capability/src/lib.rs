//! AAS Capability - Capability management for agents
//!
//! Capabilities define what actions an agent is authorized to perform.
//! The AAS is the ONLY authority layer - capabilities must be granted here.

#![deny(unsafe_code)]

use aas_types::{AgentId, Capability, CapabilityStatus};
use rcl_types::{EffectDomain, ScopeConstraint, TemporalValidity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;

/// Capability registry for managing agent capabilities
pub struct CapabilityRegistry {
    capabilities: RwLock<HashMap<String, CapabilityGrant>>,
    agent_capabilities: RwLock<HashMap<AgentId, Vec<String>>>,
}

impl CapabilityRegistry {
    /// Create a new capability registry
    pub fn new() -> Self {
        Self {
            capabilities: RwLock::new(HashMap::new()),
            agent_capabilities: RwLock::new(HashMap::new()),
        }
    }

    /// Grant a capability to an agent
    pub fn grant(&self, request: GrantRequest) -> Result<CapabilityGrant, CapabilityError> {
        let capability_id = uuid::Uuid::new_v4().to_string();

        let capability = Capability {
            capability_id: capability_id.clone(),
            domain: request.domain.clone(),
            scope: request.scope.clone(),
            validity: request.validity.clone(),
            status: CapabilityStatus::Active,
            issuer: request.issuer.clone(),
        };

        let grant = CapabilityGrant {
            grant_id: uuid::Uuid::new_v4().to_string(),
            capability,
            grantee: request.grantee.clone(),
            granted_at: chrono::Utc::now(),
            granted_by: request.issuer,
            conditions: request.conditions,
            revocation: None,
        };

        // Store the grant
        let mut capabilities = self.capabilities.write().map_err(|_| CapabilityError::LockError)?;
        capabilities.insert(capability_id.clone(), grant.clone());

        // Update agent's capability list
        let mut agent_caps = self.agent_capabilities.write().map_err(|_| CapabilityError::LockError)?;
        agent_caps
            .entry(request.grantee)
            .or_default()
            .push(capability_id);

        Ok(grant)
    }

    /// Check if an agent has a capability for a given domain and scope
    pub fn check(
        &self,
        agent_id: &AgentId,
        domain: &EffectDomain,
        scope: &ScopeConstraint,
    ) -> Result<CapabilityCheckResult, CapabilityError> {
        let capabilities = self.capabilities.read().map_err(|_| CapabilityError::LockError)?;
        let agent_caps = self.agent_capabilities.read().map_err(|_| CapabilityError::LockError)?;

        let cap_ids = match agent_caps.get(agent_id) {
            Some(ids) => ids,
            None => return Ok(CapabilityCheckResult::denied("No capabilities granted")),
        };

        let now = chrono::Utc::now();

        for cap_id in cap_ids {
            if let Some(grant) = capabilities.get(cap_id) {
                // Check status
                if grant.capability.status != CapabilityStatus::Active {
                    continue;
                }

                // Check validity
                if !grant.capability.validity.is_valid_at(now) {
                    continue;
                }

                // Check domain match
                if !domain_matches(&grant.capability.domain, domain) {
                    continue;
                }

                // Check scope match
                if !scope_matches(&grant.capability.scope, scope) {
                    continue;
                }

                // Found a matching capability
                return Ok(CapabilityCheckResult {
                    authorized: true,
                    capability_id: Some(grant.capability.capability_id.clone()),
                    grant_id: Some(grant.grant_id.clone()),
                    denial_reason: None,
                    conditions: grant.conditions.clone(),
                });
            }
        }

        Ok(CapabilityCheckResult::denied("No matching capability found"))
    }

    /// Revoke a capability
    pub fn revoke(&self, capability_id: &str, reason: &str) -> Result<(), CapabilityError> {
        let mut capabilities = self.capabilities.write().map_err(|_| CapabilityError::LockError)?;

        if let Some(grant) = capabilities.get_mut(capability_id) {
            grant.capability.status = CapabilityStatus::Revoked;
            grant.revocation = Some(Revocation {
                reason: reason.to_string(),
                revoked_at: chrono::Utc::now(),
            });
            Ok(())
        } else {
            Err(CapabilityError::NotFound(capability_id.to_string()))
        }
    }

    /// List all capabilities for an agent
    pub fn list_for_agent(&self, agent_id: &AgentId) -> Result<Vec<CapabilityGrant>, CapabilityError> {
        let capabilities = self.capabilities.read().map_err(|_| CapabilityError::LockError)?;
        let agent_caps = self.agent_capabilities.read().map_err(|_| CapabilityError::LockError)?;

        let cap_ids = match agent_caps.get(agent_id) {
            Some(ids) => ids,
            None => return Ok(vec![]),
        };

        let grants: Vec<_> = cap_ids
            .iter()
            .filter_map(|id| capabilities.get(id).cloned())
            .collect();

        Ok(grants)
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if domains match
fn domain_matches(granted: &EffectDomain, requested: &EffectDomain) -> bool {
    granted.matches(requested)
}

/// Check if scopes match
fn scope_matches(granted: &ScopeConstraint, requested: &ScopeConstraint) -> bool {
    // Global scope matches everything
    if granted.is_global() {
        return true;
    }

    // Check target patterns
    for target in &requested.targets {
        let matched = granted.targets.iter().any(|g| {
            g == "*" || g == target || (g.ends_with('*') && target.starts_with(g.trim_end_matches('*')))
        });
        if !matched {
            return false;
        }
    }

    // Check operation patterns
    for op in &requested.operations {
        let matched = granted.operations.iter().any(|g| g == "*" || g == op);
        if !matched {
            return false;
        }
    }

    true
}

/// A capability grant record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityGrant {
    pub grant_id: String,
    pub capability: Capability,
    pub grantee: AgentId,
    pub granted_at: chrono::DateTime<chrono::Utc>,
    pub granted_by: AgentId,
    pub conditions: Vec<GrantCondition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revocation: Option<Revocation>,
}

/// Request to grant a capability
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GrantRequest {
    pub grantee: AgentId,
    pub domain: EffectDomain,
    pub scope: ScopeConstraint,
    pub validity: TemporalValidity,
    pub issuer: AgentId,
    pub conditions: Vec<GrantCondition>,
}

/// Conditions attached to a grant
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GrantCondition {
    pub condition_type: ConditionType,
    pub parameters: HashMap<String, String>,
}

/// Types of grant conditions
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionType {
    HumanApproval,
    RateLimit,
    TimeRestriction,
    AdditionalVerification,
    Custom(String),
}

/// Revocation record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Revocation {
    pub reason: String,
    pub revoked_at: chrono::DateTime<chrono::Utc>,
}

/// Result of a capability check
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityCheckResult {
    pub authorized: bool,
    pub capability_id: Option<String>,
    pub grant_id: Option<String>,
    pub denial_reason: Option<String>,
    pub conditions: Vec<GrantCondition>,
}

impl CapabilityCheckResult {
    fn denied(reason: &str) -> Self {
        Self {
            authorized: false,
            capability_id: None,
            grant_id: None,
            denial_reason: Some(reason.to_string()),
            conditions: vec![],
        }
    }
}

/// Capability-related errors
#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("Capability not found: {0}")]
    NotFound(String),

    #[error("Capability expired")]
    Expired,

    #[error("Capability revoked: {0}")]
    Revoked(String),

    #[error("Insufficient scope")]
    InsufficientScope,

    #[error("Domain mismatch")]
    DomainMismatch,

    #[error("Lock error")]
    LockError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grant_and_check() {
        let registry = CapabilityRegistry::new();

        let issuer = AgentId::new("system");
        let grantee = AgentId::new("agent-1");

        let request = GrantRequest {
            grantee: grantee.clone(),
            domain: EffectDomain::Data,
            scope: ScopeConstraint::global(),
            validity: TemporalValidity::unbounded(),
            issuer,
            conditions: vec![],
        };

        registry.grant(request).unwrap();

        let result = registry
            .check(
                &grantee,
                &EffectDomain::Data,
                &ScopeConstraint::default(),
            )
            .unwrap();

        assert!(result.authorized);
    }
}
