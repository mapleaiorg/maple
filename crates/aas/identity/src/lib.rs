//! AAS Identity - Agent identity management
//!
//! This crate provides identity management for agents in the Maple framework.
//! Identity is the foundation of accountability - every action must be traceable.

#![deny(unsafe_code)]

use aas_types::AgentId;
use rcf_types::{ContinuityRef, IdentityRef};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;

/// Identity registry for managing agent identities
pub struct IdentityRegistry {
    agents: RwLock<HashMap<AgentId, RegisteredAgent>>,
}

impl IdentityRegistry {
    /// Create a new identity registry
    pub fn new() -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new agent identity
    pub fn register(&self, request: RegistrationRequest) -> Result<RegisteredAgent, IdentityError> {
        let agent_id = AgentId::new(uuid::Uuid::new_v4().to_string());
        let identity_ref = IdentityRef::new(agent_id.0.clone());

        let agent = RegisteredAgent {
            agent_id: agent_id.clone(),
            identity_ref,
            agent_type: request.agent_type,
            metadata: request.metadata,
            status: AgentStatus::Active,
            registered_at: chrono::Utc::now(),
            continuity: ContinuityRef::new(),
        };

        let mut agents = self.agents.write().map_err(|_| IdentityError::LockError)?;
        agents.insert(agent_id, agent.clone());

        Ok(agent)
    }

    /// Lookup an agent by ID
    pub fn lookup(&self, agent_id: &AgentId) -> Result<Option<RegisteredAgent>, IdentityError> {
        let agents = self.agents.read().map_err(|_| IdentityError::LockError)?;
        Ok(agents.get(agent_id).cloned())
    }

    /// Verify an identity reference
    pub fn verify(&self, identity: &IdentityRef) -> Result<VerificationResult, IdentityError> {
        let agents = self.agents.read().map_err(|_| IdentityError::LockError)?;

        let agent_id = AgentId::new(&identity.id);
        if let Some(agent) = agents.get(&agent_id) {
            if agent.status == AgentStatus::Active {
                Ok(VerificationResult {
                    valid: true,
                    agent_id: Some(agent.agent_id.clone()),
                    continuity_valid: identity.continuity_ref.verify(),
                    issues: vec![],
                })
            } else {
                Ok(VerificationResult {
                    valid: false,
                    agent_id: Some(agent.agent_id.clone()),
                    continuity_valid: false,
                    issues: vec![format!("Agent status: {:?}", agent.status)],
                })
            }
        } else {
            Ok(VerificationResult {
                valid: false,
                agent_id: None,
                continuity_valid: false,
                issues: vec!["Agent not found".to_string()],
            })
        }
    }

    /// Suspend an agent
    pub fn suspend(&self, agent_id: &AgentId, reason: &str) -> Result<(), IdentityError> {
        let mut agents = self.agents.write().map_err(|_| IdentityError::LockError)?;

        if let Some(agent) = agents.get_mut(agent_id) {
            agent.status = AgentStatus::Suspended(reason.to_string());
            Ok(())
        } else {
            Err(IdentityError::NotFound(agent_id.0.clone()))
        }
    }

    /// Revoke an agent's identity
    pub fn revoke(&self, agent_id: &AgentId, reason: &str) -> Result<(), IdentityError> {
        let mut agents = self.agents.write().map_err(|_| IdentityError::LockError)?;

        if let Some(agent) = agents.get_mut(agent_id) {
            agent.status = AgentStatus::Revoked(reason.to_string());
            Ok(())
        } else {
            Err(IdentityError::NotFound(agent_id.0.clone()))
        }
    }
}

impl Default for IdentityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A registered agent in the system
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisteredAgent {
    pub agent_id: AgentId,
    pub identity_ref: IdentityRef,
    pub agent_type: AgentType,
    pub metadata: AgentMetadata,
    pub status: AgentStatus,
    pub registered_at: chrono::DateTime<chrono::Utc>,
    pub continuity: ContinuityRef,
}

/// Request to register a new agent
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistrationRequest {
    pub agent_type: AgentType,
    pub metadata: AgentMetadata,
}

/// Types of agents in the system
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    /// A Resonator (cognitive agent with no execution authority)
    Resonator,
    /// A human user
    Human,
    /// An external system or service
    External,
    /// A composite agent (delegation chain)
    Composite,
}

/// Agent metadata
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AgentMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub tags: Vec<String>,
    pub custom: HashMap<String, String>,
}

/// Status of an agent
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Active,
    Suspended(String),
    Revoked(String),
    Pending,
}

/// Result of identity verification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationResult {
    pub valid: bool,
    pub agent_id: Option<AgentId>,
    pub continuity_valid: bool,
    pub issues: Vec<String>,
}

/// Identity-related errors
#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Identity verification failed: {0}")]
    VerificationFailed(String),

    #[error("Invalid identity format: {0}")]
    InvalidFormat(String),

    #[error("Continuity chain broken")]
    ContinuityBroken,

    #[error("Lock error")]
    LockError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_lookup() {
        let registry = IdentityRegistry::new();

        let request = RegistrationRequest {
            agent_type: AgentType::Resonator,
            metadata: AgentMetadata {
                name: Some("TestResonator".to_string()),
                ..Default::default()
            },
        };

        let agent = registry.register(request).unwrap();
        let found = registry.lookup(&agent.agent_id).unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().agent_id, agent.agent_id);
    }

    #[test]
    fn test_verify_identity() {
        let registry = IdentityRegistry::new();

        let request = RegistrationRequest {
            agent_type: AgentType::Human,
            metadata: AgentMetadata::default(),
        };

        let agent = registry.register(request).unwrap();
        let result = registry.verify(&agent.identity_ref).unwrap();

        assert!(result.valid);
    }
}
