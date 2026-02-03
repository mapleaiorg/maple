//! Deployment Manager - High-level deployment operations
//!
//! The DeploymentManager is the main entry point for deployment operations.
//! It orchestrates deployments using pluggable strategies and delegates
//! instance operations through the DeploymentContext.

use crate::context::DeploymentContext;
use crate::error::{DeploymentError, Result};
use crate::routing::DiscoveryRoutingManager;
use crate::scheduler::{DeploymentConfig, DeploymentScheduler, UpdateConfig};
use crate::state::DeploymentStateStore;
use crate::strategies;
use async_trait::async_trait;
use palm_registry::{AgentRegistry, InstanceRegistry};
use palm_types::*;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, instrument};

/// Configuration for delete operations
#[derive(Debug, Clone)]
pub struct DeleteConfig {
    /// Whether to drain gracefully
    pub graceful: bool,
    /// Timeout for the operation
    pub timeout: std::time::Duration,
}

impl Default for DeleteConfig {
    fn default() -> Self {
        Self {
            graceful: true,
            timeout: std::time::Duration::from_secs(300),
        }
    }
}

/// Policy gate trait - implemented by platform packs
#[async_trait]
pub trait PolicyGate: Send + Sync {
    /// Validate an operation against policies
    async fn validate_operation(
        &self,
        op: &PalmOperation,
        ctx: &PolicyContext,
    ) -> std::result::Result<(), PolicyError>;

    /// Get the policy gate name
    fn name(&self) -> &str;
}

/// Default policy gate that allows all operations
pub struct AllowAllPolicyGate;

#[async_trait]
impl PolicyGate for AllowAllPolicyGate {
    async fn validate_operation(
        &self,
        _op: &PalmOperation,
        _ctx: &PolicyContext,
    ) -> std::result::Result<(), PolicyError> {
        Ok(())
    }

    fn name(&self) -> &str {
        "allow-all"
    }
}

/// Deployment Manager orchestrates fleet-level deployments
pub struct DeploymentManager {
    /// Agent registry for specs
    agent_registry: Arc<dyn AgentRegistry>,
    /// Instance registry for instances
    instance_registry: Arc<dyn InstanceRegistry>,
    /// Deployment scheduler
    scheduler: Arc<DeploymentScheduler>,
    /// State store for persistence
    state_store: Arc<dyn DeploymentStateStore>,
    /// Routing manager
    routing_manager: Arc<DiscoveryRoutingManager>,
    /// Policy gate
    policy_gate: Arc<dyn PolicyGate>,
    /// Event channel
    event_tx: broadcast::Sender<PalmEventEnvelope>,
}

impl DeploymentManager {
    /// Create a new deployment manager
    pub fn new(
        agent_registry: Arc<dyn AgentRegistry>,
        instance_registry: Arc<dyn InstanceRegistry>,
        state_store: Arc<dyn DeploymentStateStore>,
        routing_manager: Arc<DiscoveryRoutingManager>,
        policy_gate: Arc<dyn PolicyGate>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(4096);
        let scheduler = Arc::new(DeploymentScheduler::new(
            instance_registry.clone(),
            state_store.clone(),
        ));

        Self {
            agent_registry,
            instance_registry,
            scheduler,
            state_store,
            routing_manager,
            policy_gate,
            event_tx,
        }
    }

    /// Create a new deployment from an agent specification
    #[instrument(skip(self), fields(spec_id = %spec_id))]
    pub async fn create_deployment(
        &self,
        spec_id: &AgentSpecId,
        strategy: DeploymentStrategy,
        replicas: ReplicaConfig,
        policy_ctx: &PolicyContext,
    ) -> Result<Deployment> {
        // 1. Validate policy
        self.policy_gate
            .validate_operation(
                &PalmOperation::CreateDeployment {
                    spec_id: spec_id.to_string(),
                },
                policy_ctx,
            )
            .await?;

        // 2. Get and validate spec
        let spec = self
            .agent_registry
            .get(spec_id)
            .await?
            .ok_or_else(|| DeploymentError::SpecNotFound(spec_id.to_string()))?;

        spec.validate()
            .map_err(|e| DeploymentError::Internal(e.to_string()))?;

        // 3. Create deployment record
        let deployment = Deployment {
            id: DeploymentId::generate(),
            agent_spec_id: spec_id.clone(),
            version: spec.version.clone(),
            platform: spec.platform,
            strategy: strategy.clone(),
            status: DeploymentStatus::Pending,
            replicas: replicas.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // 4. Persist deployment
        self.state_store
            .save_deployment(&deployment)
            .await
            .map_err(|e| DeploymentError::StateStore(e.to_string()))?;

        // 5. Emit event
        self.emit_event(
            PalmEvent::DeploymentCreated {
                deployment_id: deployment.id.clone(),
                spec_id: spec_id.clone(),
            },
            &spec.platform,
        );

        // 6. Schedule for execution
        let config = DeploymentConfig::default();
        self.scheduler.schedule(deployment.clone(), spec, config).await?;

        info!(deployment_id = %deployment.id, "Deployment created and scheduled");

        Ok(deployment)
    }

    /// Update an existing deployment to a new spec version
    #[instrument(skip(self, config), fields(deployment_id = %deployment_id))]
    pub async fn update_deployment(
        &self,
        deployment_id: &DeploymentId,
        new_spec_id: &AgentSpecId,
        config: UpdateConfig,
        policy_ctx: &PolicyContext,
    ) -> Result<Deployment> {
        // 1. Validate policy
        self.policy_gate
            .validate_operation(
                &PalmOperation::UpdateDeployment {
                    deployment_id: deployment_id.to_string(),
                },
                policy_ctx,
            )
            .await?;

        // 2. Get current deployment
        let mut deployment = self
            .state_store
            .get_deployment(deployment_id)
            .await
            .map_err(|e| DeploymentError::StateStore(e.to_string()))?
            .ok_or_else(|| DeploymentError::NotFound(deployment_id.clone()))?;

        // 3. Validate state allows update
        match &deployment.status {
            DeploymentStatus::Completed { .. } | DeploymentStatus::Paused { .. } => {}
            other => {
                return Err(DeploymentError::InvalidState {
                    current: format!("{:?}", other),
                    expected: vec!["Completed".into(), "Paused".into()],
                });
            }
        }

        // 4. Get new spec
        let new_spec = self
            .agent_registry
            .get(new_spec_id)
            .await?
            .ok_or_else(|| DeploymentError::SpecNotFound(new_spec_id.to_string()))?;

        // 5. Update deployment record
        deployment.agent_spec_id = new_spec_id.clone();
        deployment.version = new_spec.version.clone();
        deployment.status = DeploymentStatus::Pending;
        if let Some(ref strategy) = config.strategy {
            deployment.strategy = strategy.clone();
        }
        deployment.updated_at = chrono::Utc::now();

        // 6. Persist and schedule
        self.state_store
            .save_deployment(&deployment)
            .await
            .map_err(|e| DeploymentError::StateStore(e.to_string()))?;

        self.scheduler
            .schedule_update(deployment.clone(), new_spec, config)
            .await?;

        Ok(deployment)
    }

    /// Scale a deployment to a new replica count
    #[instrument(skip(self), fields(deployment_id = %deployment_id, replicas = replicas))]
    pub async fn scale(
        &self,
        deployment_id: &DeploymentId,
        replicas: u32,
        policy_ctx: &PolicyContext,
    ) -> Result<()> {
        // 1. Validate policy
        self.policy_gate
            .validate_operation(
                &PalmOperation::ScaleDeployment {
                    deployment_id: deployment_id.to_string(),
                    target_replicas: replicas,
                },
                policy_ctx,
            )
            .await?;

        // 2. Get deployment
        let mut deployment = self
            .state_store
            .get_deployment(deployment_id)
            .await
            .map_err(|e| DeploymentError::StateStore(e.to_string()))?
            .ok_or_else(|| DeploymentError::NotFound(deployment_id.clone()))?;

        // 3. Update replica count
        let old_desired = deployment.replicas.desired;
        deployment.replicas.desired = replicas;
        deployment.updated_at = chrono::Utc::now();

        self.state_store
            .save_deployment(&deployment)
            .await
            .map_err(|e| DeploymentError::StateStore(e.to_string()))?;

        // 4. Trigger scaling
        if replicas > old_desired {
            self.scheduler
                .scale_up(deployment_id, replicas - old_desired)
                .await?;
        } else if replicas < old_desired {
            self.scheduler
                .scale_down(deployment_id, old_desired - replicas)
                .await?;
        }

        // 5. Emit event
        self.emit_event(
            PalmEvent::DeploymentScaled {
                deployment_id: deployment_id.clone(),
                from_replicas: old_desired,
                to_replicas: replicas,
            },
            &deployment.platform,
        );

        info!(
            deployment_id = %deployment_id,
            old_replicas = old_desired,
            new_replicas = replicas,
            "Deployment scaled"
        );

        Ok(())
    }

    /// Rollback deployment to a previous version
    #[instrument(skip(self), fields(deployment_id = %deployment_id))]
    pub async fn rollback(
        &self,
        deployment_id: &DeploymentId,
        target_version: Option<semver::Version>,
        policy_ctx: &PolicyContext,
    ) -> Result<Deployment> {
        // 1. Validate policy
        self.policy_gate
            .validate_operation(
                &PalmOperation::RollbackDeployment {
                    deployment_id: deployment_id.to_string(),
                },
                policy_ctx,
            )
            .await?;

        // 2. Get current deployment
        let deployment = self
            .state_store
            .get_deployment(deployment_id)
            .await
            .map_err(|e| DeploymentError::StateStore(e.to_string()))?
            .ok_or_else(|| DeploymentError::NotFound(deployment_id.clone()))?;

        // 3. Determine target version
        let target = if let Some(v) = target_version {
            v
        } else {
            // Find previous version from history
            self.state_store
                .get_previous_version(deployment_id)
                .await
                .map_err(|e| DeploymentError::StateStore(e.to_string()))?
                .ok_or_else(|| DeploymentError::Internal("No previous version found".into()))?
        };

        // 4. Find spec for target version
        let spec = self
            .agent_registry
            .get_by_name_version(&deployment.agent_spec_id.to_string(), &target)
            .await?
            .ok_or_else(|| DeploymentError::SpecNotFound(format!("version {}", target)))?;

        // 5. Emit rollback event
        self.emit_event(
            PalmEvent::DeploymentRolledBack {
                deployment_id: deployment_id.clone(),
                to_version: target.clone(),
            },
            &deployment.platform,
        );

        // 6. Trigger rollback via update
        self.update_deployment(
            deployment_id,
            &spec.id,
            UpdateConfig {
                strategy: None, // Use existing strategy
                timeout: std::time::Duration::from_secs(600),
            },
            policy_ctx,
        )
        .await
    }

    /// Pause a deployment
    pub async fn pause(
        &self,
        deployment_id: &DeploymentId,
        _policy_ctx: &PolicyContext,
    ) -> Result<()> {
        self.scheduler.pause(deployment_id).await
    }

    /// Resume a paused deployment
    pub async fn resume(
        &self,
        deployment_id: &DeploymentId,
        _policy_ctx: &PolicyContext,
    ) -> Result<()> {
        self.scheduler.resume(deployment_id).await
    }

    /// Delete a deployment
    #[instrument(skip(self, config), fields(deployment_id = %deployment_id))]
    pub async fn delete(
        &self,
        deployment_id: &DeploymentId,
        config: DeleteConfig,
        policy_ctx: &PolicyContext,
    ) -> Result<()> {
        // 1. Validate policy
        self.policy_gate
            .validate_operation(
                &PalmOperation::DeleteDeployment {
                    deployment_id: deployment_id.to_string(),
                },
                policy_ctx,
            )
            .await?;

        // 2. Get all instances
        let instances = self
            .instance_registry
            .list_for_deployment(deployment_id)
            .await?;

        // 3. Terminate all instances
        let ctx = self.create_context(deployment_id).await?;
        for instance in instances {
            if config.graceful {
                ctx.terminate_instance_gracefully(&instance).await?;
            } else {
                ctx.terminate_instance_forcefully(&instance).await?;
            }
        }

        // 4. Remove deployment record
        self.state_store
            .delete_deployment(deployment_id)
            .await
            .map_err(|e| DeploymentError::StateStore(e.to_string()))?;

        info!(deployment_id = %deployment_id, "Deployment deleted");

        Ok(())
    }

    /// Get deployment status
    pub async fn get_status(&self, deployment_id: &DeploymentId) -> Result<Deployment> {
        self.state_store
            .get_deployment(deployment_id)
            .await
            .map_err(|e| DeploymentError::StateStore(e.to_string()))?
            .ok_or_else(|| DeploymentError::NotFound(deployment_id.clone()))
    }

    /// List all deployments
    pub async fn list(&self) -> Result<Vec<Deployment>> {
        self.state_store
            .list_active()
            .await
            .map_err(|e| DeploymentError::StateStore(e.to_string()))
    }

    /// Subscribe to deployment events
    pub fn subscribe(&self) -> broadcast::Receiver<PalmEventEnvelope> {
        self.event_tx.subscribe()
    }

    /// Execute a queued deployment
    ///
    /// This is called by the scheduler to actually run deployments.
    pub async fn execute_deployment(&self, deployment_id: &DeploymentId) -> Result<()> {
        let deployment = self.get_status(deployment_id).await?;
        let spec = self
            .agent_registry
            .get(&deployment.agent_spec_id)
            .await?
            .ok_or_else(|| DeploymentError::SpecNotFound(deployment.agent_spec_id.to_string()))?;

        // Update status to in-progress
        self.state_store
            .update_status(
                deployment_id,
                DeploymentStatus::InProgress {
                    progress: 0,
                    phase: "Starting".into(),
                },
            )
            .await
            .map_err(|e| DeploymentError::StateStore(e.to_string()))?;

        // Get current instances
        let current_instances = self
            .instance_registry
            .list_for_deployment(deployment_id)
            .await?;

        // Create context
        let ctx = DeploymentContext::new(
            self.instance_registry.clone(),
            self.routing_manager.clone(),
            deployment.clone(),
            spec.clone(),
        );

        // Create executor
        let executor = strategies::create_executor(&deployment.strategy);

        // Execute
        let result = executor
            .execute(&deployment, current_instances, &spec, &ctx)
            .await?;

        // Update status based on result
        match result {
            strategies::DeploymentResult::Success { .. } => {
                self.state_store
                    .update_status(
                        deployment_id,
                        DeploymentStatus::Completed {
                            completed_at: chrono::Utc::now(),
                        },
                    )
                    .await
                    .map_err(|e| DeploymentError::StateStore(e.to_string()))?;

                self.emit_event(
                    PalmEvent::DeploymentCompleted {
                        deployment_id: deployment_id.clone(),
                        duration_seconds: (chrono::Utc::now() - deployment.created_at)
                            .num_seconds() as u64,
                    },
                    &deployment.platform,
                );
            }
            strategies::DeploymentResult::PartialSuccess { reason, .. } => {
                self.state_store
                    .update_status(
                        deployment_id,
                        DeploymentStatus::Failed {
                            reason: reason.clone(),
                            failed_at: chrono::Utc::now(),
                        },
                    )
                    .await
                    .map_err(|e| DeploymentError::StateStore(e.to_string()))?;

                self.emit_event(
                    PalmEvent::DeploymentFailed {
                        deployment_id: deployment_id.clone(),
                        reason,
                    },
                    &deployment.platform,
                );
            }
            strategies::DeploymentResult::Failed { reason, .. } => {
                self.state_store
                    .update_status(
                        deployment_id,
                        DeploymentStatus::Failed {
                            reason: reason.clone(),
                            failed_at: chrono::Utc::now(),
                        },
                    )
                    .await
                    .map_err(|e| DeploymentError::StateStore(e.to_string()))?;

                self.emit_event(
                    PalmEvent::DeploymentFailed {
                        deployment_id: deployment_id.clone(),
                        reason,
                    },
                    &deployment.platform,
                );
            }
        }

        Ok(())
    }

    // --- Internal helpers ---

    async fn create_context(&self, deployment_id: &DeploymentId) -> Result<DeploymentContext> {
        let deployment = self.get_status(deployment_id).await?;
        let spec = self
            .agent_registry
            .get(&deployment.agent_spec_id)
            .await?
            .ok_or_else(|| DeploymentError::SpecNotFound(deployment.agent_spec_id.to_string()))?;

        Ok(DeploymentContext::new(
            self.instance_registry.clone(),
            self.routing_manager.clone(),
            deployment,
            spec,
        ))
    }

    fn emit_event(&self, event: PalmEvent, platform: &PlatformProfile) {
        let envelope = PalmEventEnvelope::new(event, EventSource::Deployment, *platform);
        let _ = self.event_tx.send(envelope);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use palm_registry::{InMemoryAgentRegistry, InMemoryInstanceRegistry};
    use crate::state::InMemoryDeploymentStateStore;

    #[tokio::test]
    async fn test_create_deployment() {
        let agent_registry = Arc::new(InMemoryAgentRegistry::new());
        let instance_registry = Arc::new(InMemoryInstanceRegistry::new());
        let state_store = Arc::new(InMemoryDeploymentStateStore::new());
        let routing_manager = Arc::new(DiscoveryRoutingManager::new());
        let policy_gate = Arc::new(AllowAllPolicyGate);

        let manager = DeploymentManager::new(
            agent_registry.clone(),
            instance_registry,
            state_store,
            routing_manager,
            policy_gate,
        );

        // Register a spec first
        let spec = AgentSpec::new("test-agent", semver::Version::new(1, 0, 0));
        let spec_id = agent_registry.register(spec).await.unwrap();

        // Create deployment
        let deployment = manager
            .create_deployment(
                &spec_id,
                DeploymentStrategy::default(),
                ReplicaConfig::new(3),
                &PolicyContext::default(),
            )
            .await
            .unwrap();

        assert_eq!(deployment.replicas.desired, 3);
        assert!(matches!(deployment.status, DeploymentStatus::Pending));
    }
}
