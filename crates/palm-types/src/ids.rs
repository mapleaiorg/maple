//! Strongly-typed identifiers for PALM entities
//!
//! All IDs are UUID-based but wrapped in newtype structs for type safety.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a deployment
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeploymentId(Uuid);

impl DeploymentId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for DeploymentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "deploy:{}", self.0)
    }
}

/// Unique identifier for an agent specification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentSpecId(String);

impl AgentSpecId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl fmt::Display for AgentSpecId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "spec:{}", self.0)
    }
}

/// Unique identifier for an agent instance
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstanceId(Uuid);

impl InstanceId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Create an InstanceId from a resonator ID reference
    pub fn from_resonator(resonator_id: &impl fmt::Display) -> Self {
        // Parse the UUID from the resonator ID string, or generate a new one
        let id_str = resonator_id.to_string();
        if let Some(uuid_part) = id_str.strip_prefix("resonator:") {
            if let Ok(uuid) = Uuid::parse_str(uuid_part) {
                return Self(uuid);
            }
        }
        Self::generate()
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for InstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "instance:{}", self.0)
    }
}

/// Unique identifier for a node in the cluster
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "node:{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_id_generation() {
        let id1 = DeploymentId::generate();
        let id2 = DeploymentId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_instance_id_display() {
        let id = InstanceId::generate();
        let display = format!("{}", id);
        assert!(display.starts_with("instance:"));
    }
}
