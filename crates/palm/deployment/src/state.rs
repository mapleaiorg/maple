//! Deployment state persistence

use async_trait::async_trait;
use dashmap::DashMap;
use palm_types::{Deployment, DeploymentId, DeploymentStatus};
use semver::Version;

/// State store for deployment persistence
#[async_trait]
pub trait DeploymentStateStore: Send + Sync {
    /// Save a deployment
    async fn save_deployment(&self, deployment: &Deployment) -> Result<(), StateStoreError>;

    /// Get a deployment by ID
    async fn get_deployment(
        &self,
        id: &DeploymentId,
    ) -> Result<Option<Deployment>, StateStoreError>;

    /// List active deployments
    async fn list_active(&self) -> Result<Vec<Deployment>, StateStoreError>;

    /// Update deployment status
    async fn update_status(
        &self,
        id: &DeploymentId,
        status: DeploymentStatus,
    ) -> Result<(), StateStoreError>;

    /// Delete a deployment
    async fn delete_deployment(&self, id: &DeploymentId) -> Result<(), StateStoreError>;

    /// Get previous version for rollback
    async fn get_previous_version(
        &self,
        id: &DeploymentId,
    ) -> Result<Option<Version>, StateStoreError>;

    /// Record version history
    async fn record_version(
        &self,
        id: &DeploymentId,
        version: &Version,
    ) -> Result<(), StateStoreError>;
}

/// State store errors
#[derive(Debug, thiserror::Error)]
pub enum StateStoreError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Storage error: {0}")]
    Storage(String),
}

/// In-memory implementation for development
pub struct InMemoryDeploymentStateStore {
    deployments: DashMap<DeploymentId, Deployment>,
    version_history: DashMap<DeploymentId, Vec<Version>>,
}

impl InMemoryDeploymentStateStore {
    /// Create a new in-memory state store
    pub fn new() -> Self {
        Self {
            deployments: DashMap::new(),
            version_history: DashMap::new(),
        }
    }
}

impl Default for InMemoryDeploymentStateStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DeploymentStateStore for InMemoryDeploymentStateStore {
    async fn save_deployment(&self, deployment: &Deployment) -> Result<(), StateStoreError> {
        self.deployments
            .insert(deployment.id.clone(), deployment.clone());

        // Record version
        self.version_history
            .entry(deployment.id.clone())
            .or_default()
            .push(deployment.version.clone());

        Ok(())
    }

    async fn get_deployment(
        &self,
        id: &DeploymentId,
    ) -> Result<Option<Deployment>, StateStoreError> {
        Ok(self.deployments.get(id).map(|d| d.clone()))
    }

    async fn list_active(&self) -> Result<Vec<Deployment>, StateStoreError> {
        Ok(self.deployments.iter().map(|d| d.clone()).collect())
    }

    async fn update_status(
        &self,
        id: &DeploymentId,
        status: DeploymentStatus,
    ) -> Result<(), StateStoreError> {
        if let Some(mut d) = self.deployments.get_mut(id) {
            d.status = status;
            d.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(StateStoreError::NotFound(id.to_string()))
        }
    }

    async fn delete_deployment(&self, id: &DeploymentId) -> Result<(), StateStoreError> {
        self.deployments.remove(id);
        self.version_history.remove(id);
        Ok(())
    }

    async fn get_previous_version(
        &self,
        id: &DeploymentId,
    ) -> Result<Option<Version>, StateStoreError> {
        if let Some(history) = self.version_history.get(id) {
            if history.len() >= 2 {
                return Ok(Some(history[history.len() - 2].clone()));
            }
        }
        Ok(None)
    }

    async fn record_version(
        &self,
        id: &DeploymentId,
        version: &Version,
    ) -> Result<(), StateStoreError> {
        self.version_history
            .entry(id.clone())
            .or_default()
            .push(version.clone());
        Ok(())
    }
}
