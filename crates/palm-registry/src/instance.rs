//! Instance registry trait and implementations
//!
//! The InstanceRegistry manages running agent instances.

use crate::error::{RegistryError, Result};
use async_trait::async_trait;
use palm_types::{AgentInstance, DeploymentId, HealthStatus, InstanceId, InstanceStatus};

/// Registry for agent instances
#[async_trait]
pub trait InstanceRegistry: Send + Sync {
    /// Register a new instance
    async fn register(&self, instance: AgentInstance) -> Result<()>;

    /// Get an instance by ID
    async fn get(&self, id: &InstanceId) -> Result<Option<AgentInstance>>;

    /// List all instances for a deployment
    async fn list_for_deployment(&self, deployment_id: &DeploymentId) -> Result<Vec<AgentInstance>>;

    /// List all instances
    async fn list_all(&self) -> Result<Vec<AgentInstance>>;

    /// Update instance status
    async fn update_status(&self, id: &InstanceId, status: InstanceStatus) -> Result<()>;

    /// Update instance health
    async fn update_health(&self, id: &InstanceId, health: HealthStatus) -> Result<()>;

    /// Update instance heartbeat
    async fn update_heartbeat(&self, id: &InstanceId) -> Result<()>;

    /// Remove an instance
    async fn remove(&self, id: &InstanceId) -> Result<()>;

    /// Count instances for a deployment
    async fn count_for_deployment(&self, deployment_id: &DeploymentId) -> Result<u32>;

    /// Count healthy instances for a deployment
    async fn count_healthy_for_deployment(&self, deployment_id: &DeploymentId) -> Result<u32>;
}
