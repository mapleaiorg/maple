//! In-memory storage implementation

use super::traits::*;
use crate::error::StorageError;
use async_trait::async_trait;
use palm_shared_state::{Activity, PlaygroundConfig, ResonatorStatus};
use palm_types::{
    AgentSpec, AgentSpecId, Deployment, DeploymentId, InstanceId, PalmEventEnvelope,
    instance::{AgentInstance, HealthStatus},
};
use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use tokio::sync::RwLock;
use uuid::Uuid;

/// In-memory storage for development and testing
#[derive(Debug)]
pub struct InMemoryStorage {
    specs: Arc<RwLock<HashMap<AgentSpecId, AgentSpec>>>,
    deployments: Arc<RwLock<HashMap<DeploymentId, Deployment>>>,
    instances: Arc<RwLock<HashMap<InstanceId, AgentInstance>>>,
    events: Arc<RwLock<Vec<PalmEventEnvelope>>>,
    snapshots: Arc<RwLock<HashMap<String, SnapshotInfo>>>,
    playground_config: Arc<RwLock<Option<PlaygroundConfig>>>,
    resonators: Arc<RwLock<HashMap<String, ResonatorStatus>>>,
    activities: Arc<RwLock<Vec<Activity>>>,
    activity_sequence: Arc<AtomicU64>,
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        Self {
            specs: Arc::new(RwLock::new(HashMap::new())),
            deployments: Arc::new(RwLock::new(HashMap::new())),
            instances: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(Vec::new())),
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            playground_config: Arc::new(RwLock::new(None)),
            resonators: Arc::new(RwLock::new(HashMap::new())),
            activities: Arc::new(RwLock::new(Vec::new())),
            activity_sequence: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[async_trait]
impl SpecStorage for InMemoryStorage {
    async fn get_spec(&self, id: &AgentSpecId) -> StorageResult<Option<AgentSpec>> {
        let specs = self.specs.read().await;
        Ok(specs.get(id).cloned())
    }

    async fn list_specs(&self) -> StorageResult<Vec<AgentSpec>> {
        let specs = self.specs.read().await;
        Ok(specs.values().cloned().collect())
    }

    async fn upsert_spec(&self, spec: AgentSpec) -> StorageResult<()> {
        let mut specs = self.specs.write().await;
        specs.insert(spec.id.clone(), spec);
        Ok(())
    }

    async fn delete_spec(&self, id: &AgentSpecId) -> StorageResult<bool> {
        let mut specs = self.specs.write().await;
        Ok(specs.remove(id).is_some())
    }

    async fn get_spec_by_name(&self, name: &str, version: Option<&str>) -> StorageResult<Option<AgentSpec>> {
        let specs = self.specs.read().await;
        Ok(specs.values().find(|s| {
            s.name == name && version.map_or(true, |v| s.version.to_string() == v)
        }).cloned())
    }
}

#[async_trait]
impl DeploymentStorage for InMemoryStorage {
    async fn get_deployment(&self, id: &DeploymentId) -> StorageResult<Option<Deployment>> {
        let deployments = self.deployments.read().await;
        Ok(deployments.get(id).cloned())
    }

    async fn list_deployments(&self) -> StorageResult<Vec<Deployment>> {
        let deployments = self.deployments.read().await;
        Ok(deployments.values().cloned().collect())
    }

    async fn list_deployments_for_spec(&self, spec_id: &AgentSpecId) -> StorageResult<Vec<Deployment>> {
        let deployments = self.deployments.read().await;
        Ok(deployments
            .values()
            .filter(|d| &d.agent_spec_id == spec_id)
            .cloned()
            .collect())
    }

    async fn upsert_deployment(&self, deployment: Deployment) -> StorageResult<()> {
        let mut deployments = self.deployments.write().await;
        deployments.insert(deployment.id.clone(), deployment);
        Ok(())
    }

    async fn delete_deployment(&self, id: &DeploymentId) -> StorageResult<bool> {
        let mut deployments = self.deployments.write().await;
        Ok(deployments.remove(id).is_some())
    }
}

#[async_trait]
impl InstanceStorage for InMemoryStorage {
    async fn get_instance(&self, id: &InstanceId) -> StorageResult<Option<AgentInstance>> {
        let instances = self.instances.read().await;
        Ok(instances.get(id).cloned())
    }

    async fn list_instances(&self) -> StorageResult<Vec<AgentInstance>> {
        let instances = self.instances.read().await;
        Ok(instances.values().cloned().collect())
    }

    async fn list_instances_for_deployment(&self, deployment_id: &DeploymentId) -> StorageResult<Vec<AgentInstance>> {
        let instances = self.instances.read().await;
        Ok(instances
            .values()
            .filter(|i| &i.deployment_id == deployment_id)
            .cloned()
            .collect())
    }

    async fn upsert_instance(&self, instance: AgentInstance) -> StorageResult<()> {
        let mut instances = self.instances.write().await;
        instances.insert(instance.id.clone(), instance);
        Ok(())
    }

    async fn delete_instance(&self, id: &InstanceId) -> StorageResult<bool> {
        let mut instances = self.instances.write().await;
        Ok(instances.remove(id).is_some())
    }

    async fn list_unhealthy_instances(&self) -> StorageResult<Vec<AgentInstance>> {
        let instances = self.instances.read().await;
        Ok(instances
            .values()
            .filter(|i| matches!(i.health, HealthStatus::Unhealthy { .. }))
            .cloned()
            .collect())
    }
}

#[async_trait]
impl EventStorage for InMemoryStorage {
    async fn store_event(&self, event: PalmEventEnvelope) -> StorageResult<()> {
        let mut events = self.events.write().await;
        events.push(event);

        // Keep only last 10000 events in memory
        if events.len() > 10000 {
            events.drain(0..1000);
        }

        Ok(())
    }

    async fn get_recent_events(&self, limit: usize) -> StorageResult<Vec<PalmEventEnvelope>> {
        let events = self.events.read().await;
        let start = events.len().saturating_sub(limit);
        Ok(events[start..].to_vec())
    }

    async fn get_events_for_deployment(&self, deployment_id: &DeploymentId, limit: usize) -> StorageResult<Vec<PalmEventEnvelope>> {
        let events = self.events.read().await;
        let deployment_str = deployment_id.to_string();
        let filtered: Vec<_> = events
            .iter()
            .filter(|e| format!("{:?}", e.event).contains(&deployment_str))
            .cloned()
            .collect();

        let start = filtered.len().saturating_sub(limit);
        Ok(filtered[start..].to_vec())
    }

    async fn get_events_for_instance(&self, instance_id: &InstanceId, limit: usize) -> StorageResult<Vec<PalmEventEnvelope>> {
        let events = self.events.read().await;
        let instance_str = instance_id.to_string();
        let filtered: Vec<_> = events
            .iter()
            .filter(|e| format!("{:?}", e.event).contains(&instance_str))
            .cloned()
            .collect();

        let start = filtered.len().saturating_sub(limit);
        Ok(filtered[start..].to_vec())
    }
}

#[async_trait]
impl SnapshotStorage for InMemoryStorage {
    async fn create_snapshot(&self, instance_id: &InstanceId, reason: &str) -> StorageResult<String> {
        let snapshot_id = Uuid::new_v4().to_string();
        let info = SnapshotInfo {
            id: snapshot_id.clone(),
            instance_id: instance_id.clone(),
            created_at: chrono::Utc::now(),
            reason: reason.to_string(),
            size_bytes: 1024, // Mock size
        };

        let mut snapshots = self.snapshots.write().await;
        snapshots.insert(snapshot_id.clone(), info);

        Ok(snapshot_id)
    }

    async fn list_snapshots(&self, instance_id: &InstanceId) -> StorageResult<Vec<SnapshotInfo>> {
        let snapshots = self.snapshots.read().await;
        Ok(snapshots
            .values()
            .filter(|s| &s.instance_id == instance_id)
            .cloned()
            .collect())
    }

    async fn restore_snapshot(&self, instance_id: &InstanceId, snapshot_id: &str) -> StorageResult<()> {
        let snapshots = self.snapshots.read().await;
        let snapshot = snapshots
            .get(snapshot_id)
            .ok_or_else(|| StorageError::NotFound(format!("Snapshot {} not found", snapshot_id)))?;

        if &snapshot.instance_id != instance_id {
            return Err(StorageError::InvalidData(
                "Snapshot does not belong to this instance".to_string(),
            ));
        }

        // In a real implementation, this would restore the state
        tracing::info!(
            snapshot_id = %snapshot_id,
            instance_id = %instance_id,
            "Restored instance from snapshot"
        );

        Ok(())
    }

    async fn delete_snapshot(&self, snapshot_id: &str) -> StorageResult<bool> {
        let mut snapshots = self.snapshots.write().await;
        Ok(snapshots.remove(snapshot_id).is_some())
    }
}

impl Storage for InMemoryStorage {}

#[async_trait]
impl PlaygroundConfigStorage for InMemoryStorage {
    async fn get_playground_config(&self) -> StorageResult<Option<PlaygroundConfig>> {
        let config = self.playground_config.read().await;
        Ok(config.clone())
    }

    async fn upsert_playground_config(&self, config: PlaygroundConfig) -> StorageResult<()> {
        let mut current = self.playground_config.write().await;
        *current = Some(config);
        Ok(())
    }
}

#[async_trait]
impl ResonatorStorage for InMemoryStorage {
    async fn get_resonator(&self, id: &str) -> StorageResult<Option<ResonatorStatus>> {
        let resonators = self.resonators.read().await;
        Ok(resonators.get(id).cloned())
    }

    async fn list_resonators(&self) -> StorageResult<Vec<ResonatorStatus>> {
        let resonators = self.resonators.read().await;
        Ok(resonators.values().cloned().collect())
    }

    async fn upsert_resonator(&self, resonator: ResonatorStatus) -> StorageResult<()> {
        let mut resonators = self.resonators.write().await;
        resonators.insert(resonator.id.clone(), resonator);
        Ok(())
    }

    async fn delete_resonator(&self, id: &str) -> StorageResult<bool> {
        let mut resonators = self.resonators.write().await;
        Ok(resonators.remove(id).is_some())
    }
}

#[async_trait]
impl ActivityStorage for InMemoryStorage {
    async fn store_activity(&self, mut activity: Activity) -> StorageResult<Activity> {
        if activity.sequence == 0 {
            let seq = self.activity_sequence.fetch_add(1, Ordering::SeqCst) + 1;
            activity.sequence = seq;
        }

        let mut activities = self.activities.write().await;
        activities.push(activity.clone());

        // Keep only last 50k activities in memory
        if activities.len() > 50_000 {
            activities.drain(0..5_000);
        }

        Ok(activity)
    }

    async fn list_activities(
        &self,
        limit: usize,
        after_sequence: Option<u64>,
    ) -> StorageResult<Vec<Activity>> {
        let activities = self.activities.read().await;

        let mut filtered: Vec<Activity> = match after_sequence {
            Some(after) => activities
                .iter()
                .filter(|a| a.sequence > after)
                .cloned()
                .collect(),
            None => activities.iter().cloned().collect(),
        };

        filtered.sort_by_key(|a| a.sequence);

        if filtered.len() > limit {
            filtered = filtered[filtered.len().saturating_sub(limit)..].to_vec();
        }

        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use palm_types::{
        DeploymentStrategy, PlatformProfile, ReplicaConfig,
        instance::{InstanceMetrics, InstancePlacement, InstanceStatus, ResonatorIdRef},
    };

    fn create_test_spec() -> AgentSpec {
        AgentSpec::new("test-agent", semver::Version::new(1, 0, 0))
    }

    fn create_test_deployment(spec_id: &AgentSpecId) -> Deployment {
        Deployment {
            id: DeploymentId::generate(),
            agent_spec_id: spec_id.clone(),
            version: semver::Version::new(1, 0, 0),
            platform: PlatformProfile::Development,
            strategy: DeploymentStrategy::default(),
            status: palm_types::DeploymentStatus::Pending,
            replicas: ReplicaConfig::new(3),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn create_test_instance(deployment_id: &DeploymentId) -> AgentInstance {
        AgentInstance {
            id: InstanceId::generate(),
            deployment_id: deployment_id.clone(),
            resonator_id: ResonatorIdRef::new("resonator-123"),
            status: InstanceStatus::Running,
            health: HealthStatus::Healthy,
            placement: InstancePlacement::default(),
            metrics: InstanceMetrics::default(),
            started_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_spec_crud() {
        let storage = InMemoryStorage::new();
        let spec = create_test_spec();

        // Create
        storage.upsert_spec(spec.clone()).await.unwrap();

        // Read
        let retrieved = storage.get_spec(&spec.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-agent");

        // List
        let specs = storage.list_specs().await.unwrap();
        assert_eq!(specs.len(), 1);

        // Delete
        let deleted = storage.delete_spec(&spec.id).await.unwrap();
        assert!(deleted);

        let retrieved = storage.get_spec(&spec.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_deployment_crud() {
        let storage = InMemoryStorage::new();
        let spec = create_test_spec();
        let deployment = create_test_deployment(&spec.id);

        storage.upsert_spec(spec.clone()).await.unwrap();
        storage.upsert_deployment(deployment.clone()).await.unwrap();

        let retrieved = storage.get_deployment(&deployment.id).await.unwrap();
        assert!(retrieved.is_some());

        let deployments = storage.list_deployments_for_spec(&spec.id).await.unwrap();
        assert_eq!(deployments.len(), 1);
    }

    #[tokio::test]
    async fn test_instance_crud() {
        let storage = InMemoryStorage::new();
        let spec = create_test_spec();
        let deployment = create_test_deployment(&spec.id);
        let instance = create_test_instance(&deployment.id);

        storage.upsert_instance(instance.clone()).await.unwrap();

        let retrieved = storage.get_instance(&instance.id).await.unwrap();
        assert!(retrieved.is_some());

        let instances = storage.list_instances_for_deployment(&deployment.id).await.unwrap();
        assert_eq!(instances.len(), 1);
    }

    #[tokio::test]
    async fn test_snapshot_operations() {
        let storage = InMemoryStorage::new();
        let instance_id = InstanceId::generate();

        // Create snapshot
        let snapshot_id = storage.create_snapshot(&instance_id, "manual").await.unwrap();
        assert!(!snapshot_id.is_empty());

        // List snapshots
        let snapshots = storage.list_snapshots(&instance_id).await.unwrap();
        assert_eq!(snapshots.len(), 1);

        // Restore
        storage.restore_snapshot(&instance_id, &snapshot_id).await.unwrap();

        // Delete
        let deleted = storage.delete_snapshot(&snapshot_id).await.unwrap();
        assert!(deleted);
    }
}
