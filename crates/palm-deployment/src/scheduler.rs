//! Deployment scheduler and queue management

use crate::error::{DeploymentError, Result};
use crate::state::DeploymentStateStore;
use palm_registry::InstanceRegistry;
use palm_types::{AgentSpec, Deployment, DeploymentId, DeploymentStatus};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Queued deployment for processing
#[derive(Debug, Clone)]
pub struct QueuedDeployment {
    /// The deployment record
    pub deployment: Deployment,
    /// The agent spec to deploy
    pub spec: AgentSpec,
    /// Deployment configuration
    pub config: DeploymentConfig,
    /// Queue priority (higher = processed first)
    pub priority: u32,
    /// Queued timestamp
    pub queued_at: chrono::DateTime<chrono::Utc>,
}

/// Configuration for deployment operations
#[derive(Debug, Clone)]
pub struct DeploymentConfig {
    /// Timeout for the entire deployment
    pub timeout: std::time::Duration,
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(600),
        }
    }
}

/// Configuration for update operations
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    /// Optional new strategy
    pub strategy: Option<palm_types::DeploymentStrategy>,
    /// Timeout for the update
    pub timeout: std::time::Duration,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            strategy: None,
            timeout: std::time::Duration::from_secs(600),
        }
    }
}

/// Deployment scheduler - manages deployment queue and execution
pub struct DeploymentScheduler {
    /// Queue of pending deployments
    queue: Arc<RwLock<Vec<QueuedDeployment>>>,
    /// Instance registry for instance management
    instance_registry: Arc<dyn InstanceRegistry>,
    /// State store for deployment persistence
    state_store: Arc<dyn DeploymentStateStore>,
    /// Paused deployments
    paused: Arc<RwLock<std::collections::HashSet<DeploymentId>>>,
}

impl DeploymentScheduler {
    /// Create a new scheduler
    pub fn new(
        instance_registry: Arc<dyn InstanceRegistry>,
        state_store: Arc<dyn DeploymentStateStore>,
    ) -> Self {
        Self {
            queue: Arc::new(RwLock::new(Vec::new())),
            instance_registry,
            state_store,
            paused: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }

    /// Schedule a new deployment
    pub async fn schedule(
        &self,
        deployment: Deployment,
        spec: AgentSpec,
        config: DeploymentConfig,
    ) -> Result<()> {
        let queued = QueuedDeployment {
            deployment: deployment.clone(),
            spec,
            config,
            priority: self.calculate_priority(&deployment),
            queued_at: chrono::Utc::now(),
        };

        let mut queue = self.queue.write().await;
        queue.push(queued);
        // Sort by priority (descending) then by queue time (ascending)
        queue.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.queued_at.cmp(&b.queued_at))
        });

        info!(
            deployment_id = %deployment.id,
            queue_position = queue.len(),
            "Deployment scheduled"
        );

        Ok(())
    }

    /// Schedule an update deployment
    pub async fn schedule_update(
        &self,
        deployment: Deployment,
        spec: AgentSpec,
        config: UpdateConfig,
    ) -> Result<()> {
        let deploy_config = DeploymentConfig {
            timeout: config.timeout,
        };
        self.schedule(deployment, spec, deploy_config).await
    }

    /// Get the next deployment to process
    pub async fn next(&self) -> Option<QueuedDeployment> {
        let paused = self.paused.read().await;
        let mut queue = self.queue.write().await;

        // Find the first non-paused deployment
        let pos = queue
            .iter()
            .position(|q| !paused.contains(&q.deployment.id))?;

        Some(queue.remove(pos))
    }

    /// Pause a deployment
    pub async fn pause(&self, deployment_id: &DeploymentId) -> Result<()> {
        let mut paused = self.paused.write().await;
        paused.insert(deployment_id.clone());

        // Update deployment status
        self.state_store
            .update_status(
                deployment_id,
                DeploymentStatus::Paused {
                    reason: "Manual pause".into(),
                    paused_at: chrono::Utc::now(),
                },
            )
            .await
            .map_err(|e| DeploymentError::StateStore(e.to_string()))?;

        info!(deployment_id = %deployment_id, "Deployment paused");
        Ok(())
    }

    /// Resume a paused deployment
    pub async fn resume(&self, deployment_id: &DeploymentId) -> Result<()> {
        let mut paused = self.paused.write().await;
        if !paused.remove(deployment_id) {
            warn!(deployment_id = %deployment_id, "Deployment was not paused");
        }

        // Update deployment status back to pending/in-progress
        self.state_store
            .update_status(deployment_id, DeploymentStatus::Pending)
            .await
            .map_err(|e| DeploymentError::StateStore(e.to_string()))?;

        info!(deployment_id = %deployment_id, "Deployment resumed");
        Ok(())
    }

    /// Scale up a deployment
    pub async fn scale_up(&self, deployment_id: &DeploymentId, count: u32) -> Result<()> {
        info!(
            deployment_id = %deployment_id,
            count = count,
            "Scale up requested"
        );
        // The actual scaling is handled by the deployment manager
        // This just updates the desired replica count
        Ok(())
    }

    /// Scale down a deployment
    pub async fn scale_down(&self, deployment_id: &DeploymentId, count: u32) -> Result<()> {
        info!(
            deployment_id = %deployment_id,
            count = count,
            "Scale down requested"
        );
        // The actual scaling is handled by the deployment manager
        Ok(())
    }

    /// Get queue length
    pub async fn queue_length(&self) -> usize {
        self.queue.read().await.len()
    }

    /// Check if a deployment is paused
    pub async fn is_paused(&self, deployment_id: &DeploymentId) -> bool {
        self.paused.read().await.contains(deployment_id)
    }

    /// Calculate priority for a deployment
    fn calculate_priority(&self, deployment: &Deployment) -> u32 {
        // Higher priority for:
        // - IBank (financial) deployments
        // - Finalverse (human-AI) deployments
        // - Production environments (based on metadata)
        match deployment.platform {
            palm_types::PlatformProfile::IBank => 100,
            palm_types::PlatformProfile::Finalverse => 75,
            palm_types::PlatformProfile::Mapleverse => 50,
            palm_types::PlatformProfile::Development => 25,
        }
    }
}
