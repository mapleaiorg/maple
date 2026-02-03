//! Storage trait definitions

use crate::error::StorageError;
use async_trait::async_trait;
use palm_shared_state::{Activity, PlaygroundConfig, ResonatorStatus};
use palm_types::{
    AgentSpec, AgentSpecId, Deployment, DeploymentId, InstanceId, PalmEventEnvelope,
    instance::AgentInstance,
};

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Combined storage trait
#[async_trait]
pub trait Storage:
    SpecStorage
    + DeploymentStorage
    + InstanceStorage
    + EventStorage
    + SnapshotStorage
    + PlaygroundConfigStorage
    + ResonatorStorage
    + ActivityStorage
    + Send
    + Sync
{
}

/// Storage for agent specifications
#[async_trait]
pub trait SpecStorage: Send + Sync {
    /// Get a spec by ID
    async fn get_spec(&self, id: &AgentSpecId) -> StorageResult<Option<AgentSpec>>;

    /// List all specs
    async fn list_specs(&self) -> StorageResult<Vec<AgentSpec>>;

    /// Create or update a spec
    async fn upsert_spec(&self, spec: AgentSpec) -> StorageResult<()>;

    /// Delete a spec by ID
    async fn delete_spec(&self, id: &AgentSpecId) -> StorageResult<bool>;

    /// Get spec by name and version
    #[allow(dead_code)]
    async fn get_spec_by_name(&self, name: &str, version: Option<&str>) -> StorageResult<Option<AgentSpec>>;
}

/// Storage for deployments
#[async_trait]
pub trait DeploymentStorage: Send + Sync {
    /// Get a deployment by ID
    async fn get_deployment(&self, id: &DeploymentId) -> StorageResult<Option<Deployment>>;

    /// List all deployments
    async fn list_deployments(&self) -> StorageResult<Vec<Deployment>>;

    /// List deployments for a specific spec
    async fn list_deployments_for_spec(&self, spec_id: &AgentSpecId) -> StorageResult<Vec<Deployment>>;

    /// Create or update a deployment
    async fn upsert_deployment(&self, deployment: Deployment) -> StorageResult<()>;

    /// Delete a deployment by ID
    async fn delete_deployment(&self, id: &DeploymentId) -> StorageResult<bool>;
}

/// Storage for instances
#[async_trait]
pub trait InstanceStorage: Send + Sync {
    /// Get an instance by ID
    async fn get_instance(&self, id: &InstanceId) -> StorageResult<Option<AgentInstance>>;

    /// List all instances
    async fn list_instances(&self) -> StorageResult<Vec<AgentInstance>>;

    /// List instances for a specific deployment
    async fn list_instances_for_deployment(&self, deployment_id: &DeploymentId) -> StorageResult<Vec<AgentInstance>>;

    /// Create or update an instance
    async fn upsert_instance(&self, instance: AgentInstance) -> StorageResult<()>;

    /// Delete an instance by ID
    async fn delete_instance(&self, id: &InstanceId) -> StorageResult<bool>;

    /// Get unhealthy instances
    async fn list_unhealthy_instances(&self) -> StorageResult<Vec<AgentInstance>>;
}

/// Storage for events
#[async_trait]
pub trait EventStorage: Send + Sync {
    /// Store an event
    async fn store_event(&self, event: PalmEventEnvelope) -> StorageResult<()>;

    /// Get recent events
    async fn get_recent_events(&self, limit: usize) -> StorageResult<Vec<PalmEventEnvelope>>;

    /// Get events for a deployment
    async fn get_events_for_deployment(&self, deployment_id: &DeploymentId, limit: usize) -> StorageResult<Vec<PalmEventEnvelope>>;

    /// Get events for an instance
    async fn get_events_for_instance(&self, instance_id: &InstanceId, limit: usize) -> StorageResult<Vec<PalmEventEnvelope>>;
}

/// Snapshot information
#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    pub id: String,
    pub instance_id: InstanceId,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub reason: String,
    pub size_bytes: u64,
}

/// Storage for snapshots
#[async_trait]
pub trait SnapshotStorage: Send + Sync {
    /// Create a snapshot for an instance
    async fn create_snapshot(&self, instance_id: &InstanceId, reason: &str) -> StorageResult<String>;

    /// List snapshots for an instance
    async fn list_snapshots(&self, instance_id: &InstanceId) -> StorageResult<Vec<SnapshotInfo>>;

    /// Restore from a snapshot
    async fn restore_snapshot(&self, instance_id: &InstanceId, snapshot_id: &str) -> StorageResult<()>;

    /// Delete a snapshot
    #[allow(dead_code)]
    async fn delete_snapshot(&self, snapshot_id: &str) -> StorageResult<bool>;
}

/// Storage for playground configuration
#[async_trait]
pub trait PlaygroundConfigStorage: Send + Sync {
    /// Get the playground configuration (singleton)
    async fn get_playground_config(&self) -> StorageResult<Option<PlaygroundConfig>>;

    /// Store the playground configuration
    async fn upsert_playground_config(&self, config: PlaygroundConfig) -> StorageResult<()>;
}

/// Storage for resonator state summaries
#[async_trait]
pub trait ResonatorStorage: Send + Sync {
    /// Get a resonator by ID
    #[allow(dead_code)]
    async fn get_resonator(&self, id: &str) -> StorageResult<Option<ResonatorStatus>>;

    /// List all resonators
    async fn list_resonators(&self) -> StorageResult<Vec<ResonatorStatus>>;

    /// Create or update a resonator
    async fn upsert_resonator(&self, resonator: ResonatorStatus) -> StorageResult<()>;

    /// Delete a resonator
    #[allow(dead_code)]
    async fn delete_resonator(&self, id: &str) -> StorageResult<bool>;
}

/// Storage for activity events
#[async_trait]
pub trait ActivityStorage: Send + Sync {
    /// Store an activity entry (returns the stored activity with sequence assigned)
    async fn store_activity(&self, activity: Activity) -> StorageResult<Activity>;

    /// List recent activities
    async fn list_activities(&self, limit: usize, after_sequence: Option<u64>) -> StorageResult<Vec<Activity>>;
}
