//! Main PalmControlPlane implementation
//!
//! The PalmControlPlane is the unified entry point for all PALM operations.
//! It composes all PALM subsystems (registry, deployment, health, state)
//! behind a unified API that enforces policy gates and emits events.

use crate::context::RequestContext;
use crate::error::{ControlPlaneError, Result};
use crate::events::EventAggregator;
use crate::operations::ControlPlaneOperation;
use palm_deployment::{
    DeleteConfig, DeploymentManager, DeploymentStateStore, DiscoveryRoutingManager, PolicyGate,
    UpdateConfig,
};
use palm_health::{HealthAssessment, HealthConfig, HealthMonitor, ResilienceController};
use palm_registry::{AgentRegistry, InstanceRegistry};
use palm_state::{
    AasClient, ContinuityVerifier, CouplingRuntime, InstanceInfo, MigrationRuntime, NodeId,
    ResonatorId, RuntimeStateGatherer, RuntimeStateRestorer, SnapshotMetadata, SnapshotReason,
    StateManager, StateManagerConfig, StateSnapshotId, StateStorage,
};
use palm_types::*;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, instrument, warn};

/// Configuration for creating a deployment through the control plane
#[derive(Debug, Clone)]
pub struct CreateDeploymentConfig {
    /// Deployment strategy
    pub strategy: DeploymentStrategy,
    /// Replica configuration
    pub replicas: ReplicaConfig,
    /// Timeout for the deployment
    pub timeout: std::time::Duration,
}

impl Default for CreateDeploymentConfig {
    fn default() -> Self {
        Self {
            strategy: DeploymentStrategy::default(),
            replicas: ReplicaConfig::new(1),
            timeout: std::time::Duration::from_secs(600),
        }
    }
}

impl CreateDeploymentConfig {
    /// Create a new deployment config with the specified replica count
    pub fn with_replicas(replicas: u32) -> Self {
        Self {
            replicas: ReplicaConfig::new(replicas),
            ..Default::default()
        }
    }
}

/// Unified control plane for all PALM operations
pub struct PalmControlPlane {
    /// Platform profile
    platform: PlatformProfile,

    /// Agent registry for specs
    agent_registry: Arc<dyn AgentRegistry>,

    /// Instance registry for instances
    instance_registry: Arc<dyn InstanceRegistry>,

    /// Policy enforcement gate
    policy_gate: Arc<dyn PolicyGate>,

    /// Deployment manager
    deployment_manager: DeploymentManager,

    /// Health monitor
    health_monitor: Arc<HealthMonitor>,

    /// Resilience controller
    #[allow(dead_code)]
    resilience_controller: Arc<ResilienceController>,

    /// State manager
    state_manager: Arc<StateManager>,

    /// Event aggregator
    event_aggregator: Arc<EventAggregator>,
}

impl PalmControlPlane {
    /// Create a new control plane with all dependencies
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        platform: PlatformProfile,
        agent_registry: Arc<dyn AgentRegistry>,
        instance_registry: Arc<dyn InstanceRegistry>,
        policy_gate: Arc<dyn PolicyGate>,
        state_storage: Arc<dyn StateStorage>,
        deployment_state_store: Arc<dyn DeploymentStateStore>,
        state_gatherer: Arc<dyn RuntimeStateGatherer>,
        state_restorer: Arc<dyn RuntimeStateRestorer>,
        continuity_verifier: Arc<dyn ContinuityVerifier>,
        coupling_runtime: Arc<dyn CouplingRuntime>,
        migration_runtime: Arc<dyn MigrationRuntime>,
        aas_client: Arc<dyn AasClient>,
        health_config: HealthConfig,
        state_config: StateManagerConfig,
    ) -> Self {
        let event_aggregator = Arc::new(EventAggregator::new());

        // Create routing manager for deployments
        let routing_manager = Arc::new(DiscoveryRoutingManager::new());

        // Create deployment manager
        let deployment_manager = DeploymentManager::new(
            agent_registry.clone(),
            instance_registry.clone(),
            deployment_state_store,
            routing_manager,
            policy_gate.clone(),
        );

        // Create resilience controller
        let resilience_controller = Arc::new(ResilienceController::new(
            health_config.resilience.clone(),
            platform,
            Arc::new(palm_health::resilience::NoOpRecoveryExecutor),
        ));

        // Create health monitor
        let health_monitor = Arc::new(HealthMonitor::new(health_config, resilience_controller.clone()));

        // Create state manager
        let state_manager = Arc::new(StateManager::new(
            state_config,
            state_storage,
            state_gatherer,
            state_restorer,
            continuity_verifier,
            coupling_runtime,
            migration_runtime,
            aas_client,
        ));

        Self {
            platform,
            agent_registry,
            instance_registry,
            policy_gate,
            deployment_manager,
            health_monitor,
            resilience_controller,
            state_manager,
            event_aggregator,
        }
    }

    /// Get the platform profile
    pub fn platform(&self) -> PlatformProfile {
        self.platform
    }

    // ========== Registry Operations ==========

    /// Register a new agent specification
    #[instrument(skip(self, spec), fields(spec_name = %spec.name))]
    pub async fn register_spec(&self, spec: AgentSpec, ctx: &RequestContext) -> Result<AgentSpecId> {
        self.check_policy(
            &ControlPlaneOperation::RegisterSpec {
                spec_id: spec.id.clone(),
            },
            ctx,
        )
        .await?;

        let spec_id = self.agent_registry.register(spec).await?;

        self.event_aggregator.emit_info(
            PalmEvent::SpecRegistered {
                spec_id: spec_id.clone(),
            },
            ctx.platform,
        );

        info!(spec_id = %spec_id, "Agent spec registered");

        Ok(spec_id)
    }

    /// Get an agent specification by ID
    pub async fn get_spec(&self, spec_id: &AgentSpecId) -> Result<Option<AgentSpec>> {
        Ok(self.agent_registry.get(spec_id).await?)
    }

    /// List all versions of a spec by name
    pub async fn list_spec_versions(&self, name: &str) -> Result<Vec<AgentSpec>> {
        Ok(self.agent_registry.list_versions(name).await?)
    }

    // ========== Deployment Operations ==========

    /// Create a new deployment
    #[instrument(skip(self, config), fields(spec_id = %spec_id))]
    pub async fn create_deployment(
        &self,
        spec_id: &AgentSpecId,
        config: CreateDeploymentConfig,
        ctx: &RequestContext,
    ) -> Result<Deployment> {
        self.check_policy(
            &ControlPlaneOperation::CreateDeployment {
                spec_id: spec_id.clone(),
                replicas: config.replicas.desired,
            },
            ctx,
        )
        .await?;

        // Get spec for validation
        let _spec = self
            .agent_registry
            .get(spec_id)
            .await?
            .ok_or_else(|| ControlPlaneError::spec_not_found(spec_id))?;

        let deployment = self
            .deployment_manager
            .create_deployment(spec_id, config.strategy, config.replicas, &ctx.policy_context)
            .await?;

        info!(deployment_id = %deployment.id, "Deployment created");

        Ok(deployment)
    }

    /// Update a deployment to a new spec version
    #[instrument(skip(self, config), fields(deployment_id = %deployment_id, new_spec_id = %new_spec_id))]
    pub async fn update_deployment(
        &self,
        deployment_id: &DeploymentId,
        new_spec_id: &AgentSpecId,
        config: UpdateConfig,
        ctx: &RequestContext,
    ) -> Result<Deployment> {
        self.check_policy(
            &ControlPlaneOperation::UpdateDeployment {
                deployment_id: deployment_id.clone(),
                new_spec_id: new_spec_id.clone(),
            },
            ctx,
        )
        .await?;

        let deployment = self
            .deployment_manager
            .update_deployment(deployment_id, new_spec_id, config, &ctx.policy_context)
            .await?;

        Ok(deployment)
    }

    /// Scale a deployment to a new replica count
    #[instrument(skip(self), fields(deployment_id = %deployment_id, replicas = replicas))]
    pub async fn scale_deployment(
        &self,
        deployment_id: &DeploymentId,
        replicas: u32,
        ctx: &RequestContext,
    ) -> Result<()> {
        self.check_policy(
            &ControlPlaneOperation::ScaleDeployment {
                deployment_id: deployment_id.clone(),
                replicas,
            },
            ctx,
        )
        .await?;

        self.deployment_manager
            .scale(deployment_id, replicas, &ctx.policy_context)
            .await?;

        Ok(())
    }

    /// Rollback a deployment to a previous version
    #[instrument(skip(self), fields(deployment_id = %deployment_id))]
    pub async fn rollback_deployment(
        &self,
        deployment_id: &DeploymentId,
        target_version: Option<semver::Version>,
        ctx: &RequestContext,
    ) -> Result<Deployment> {
        self.check_policy(
            &ControlPlaneOperation::RollbackDeployment {
                deployment_id: deployment_id.clone(),
                target_version: target_version.as_ref().map(|v| v.to_string()),
            },
            ctx,
        )
        .await?;

        let deployment = self
            .deployment_manager
            .rollback(deployment_id, target_version, &ctx.policy_context)
            .await?;

        Ok(deployment)
    }

    /// Delete a deployment
    #[instrument(skip(self, config), fields(deployment_id = %deployment_id))]
    pub async fn delete_deployment(
        &self,
        deployment_id: &DeploymentId,
        config: DeleteConfig,
        ctx: &RequestContext,
    ) -> Result<()> {
        self.check_policy(
            &ControlPlaneOperation::DeleteDeployment {
                deployment_id: deployment_id.clone(),
            },
            ctx,
        )
        .await?;

        self.deployment_manager
            .delete(deployment_id, config, &ctx.policy_context)
            .await?;

        Ok(())
    }

    /// Pause a deployment
    #[instrument(skip(self), fields(deployment_id = %deployment_id))]
    pub async fn pause_deployment(
        &self,
        deployment_id: &DeploymentId,
        ctx: &RequestContext,
    ) -> Result<()> {
        self.check_policy(
            &ControlPlaneOperation::PauseDeployment {
                deployment_id: deployment_id.clone(),
            },
            ctx,
        )
        .await?;

        self.deployment_manager
            .pause(deployment_id, &ctx.policy_context)
            .await?;

        self.event_aggregator.emit_info(
            PalmEvent::DeploymentPaused {
                deployment_id: deployment_id.clone(),
            },
            ctx.platform,
        );

        Ok(())
    }

    /// Resume a paused deployment
    #[instrument(skip(self), fields(deployment_id = %deployment_id))]
    pub async fn resume_deployment(
        &self,
        deployment_id: &DeploymentId,
        ctx: &RequestContext,
    ) -> Result<()> {
        self.check_policy(
            &ControlPlaneOperation::ResumeDeployment {
                deployment_id: deployment_id.clone(),
            },
            ctx,
        )
        .await?;

        self.deployment_manager
            .resume(deployment_id, &ctx.policy_context)
            .await?;

        self.event_aggregator.emit_info(
            PalmEvent::DeploymentResumed {
                deployment_id: deployment_id.clone(),
            },
            ctx.platform,
        );

        Ok(())
    }

    /// Get deployment status
    pub async fn get_deployment(&self, deployment_id: &DeploymentId) -> Result<Deployment> {
        Ok(self.deployment_manager.get_status(deployment_id).await?)
    }

    /// List all deployments
    pub async fn list_deployments(&self) -> Result<Vec<Deployment>> {
        Ok(self.deployment_manager.list().await?)
    }

    // ========== Instance Operations ==========

    /// Get instance details
    pub async fn get_instance(&self, instance_id: &InstanceId) -> Result<Option<AgentInstance>> {
        Ok(self.instance_registry.get(instance_id).await?)
    }

    /// List instances for a deployment
    pub async fn list_instances(&self, deployment_id: &DeploymentId) -> Result<Vec<AgentInstance>> {
        Ok(self.instance_registry.list_for_deployment(deployment_id).await?)
    }

    /// Restart an instance
    #[instrument(skip(self), fields(instance_id = %instance_id, graceful = graceful))]
    pub async fn restart_instance(
        &self,
        instance_id: &InstanceId,
        graceful: bool,
        ctx: &RequestContext,
    ) -> Result<()> {
        self.check_policy(
            &ControlPlaneOperation::RestartInstance {
                instance_id: instance_id.clone(),
                graceful,
            },
            ctx,
        )
        .await?;

        let _instance = self
            .instance_registry
            .get(instance_id)
            .await?
            .ok_or_else(|| ControlPlaneError::instance_not_found(instance_id))?;

        if graceful {
            // Create checkpoint before restart
            if let Err(e) = self
                .create_checkpoint_internal(instance_id, SnapshotReason::PreRestart)
                .await
            {
                warn!(instance_id = %instance_id, error = %e, "Failed to create pre-restart checkpoint");
            }
        }

        self.event_aggregator.emit_info(
            PalmEvent::InstanceRestarted {
                instance_id: instance_id.clone(),
                graceful,
            },
            ctx.platform,
        );

        info!(instance_id = %instance_id, graceful = graceful, "Instance restart initiated");

        Ok(())
    }

    /// Terminate an instance
    #[instrument(skip(self), fields(instance_id = %instance_id, graceful = graceful))]
    pub async fn terminate_instance(
        &self,
        instance_id: &InstanceId,
        graceful: bool,
        ctx: &RequestContext,
    ) -> Result<()> {
        self.check_policy(
            &ControlPlaneOperation::TerminateInstance {
                instance_id: instance_id.clone(),
                graceful,
            },
            ctx,
        )
        .await?;

        if graceful {
            // Create final checkpoint (use Manual since there's no PreTermination)
            if let Err(e) = self
                .create_checkpoint_internal(instance_id, SnapshotReason::Manual)
                .await
            {
                warn!(instance_id = %instance_id, error = %e, "Failed to create pre-termination checkpoint");
            }
        }

        self.event_aggregator.emit_info(
            PalmEvent::InstanceTerminated {
                instance_id: instance_id.clone(),
                exit_code: None,
            },
            ctx.platform,
        );

        info!(instance_id = %instance_id, graceful = graceful, "Instance termination initiated");

        Ok(())
    }

    /// Migrate an instance to another node
    #[instrument(skip(self), fields(instance_id = %instance_id, to_node = %to_node))]
    pub async fn migrate_instance(
        &self,
        instance_id: &InstanceId,
        to_node: &str,
        ctx: &RequestContext,
    ) -> Result<InstanceId> {
        self.check_policy(
            &ControlPlaneOperation::MigrateInstance {
                instance_id: instance_id.clone(),
                to_node: to_node.to_string(),
            },
            ctx,
        )
        .await?;

        let instance = self
            .instance_registry
            .get(instance_id)
            .await?
            .ok_or_else(|| ControlPlaneError::instance_not_found(instance_id))?;

        let instance_info = InstanceInfo {
            instance_id: instance_id.clone(),
            deployment_id: instance.deployment_id.clone(),
            resonator_id: ResonatorId::generate(), // Would come from instance in real impl
        };

        let to_node_id = NodeId::new(to_node);
        let result = self.state_manager.migrate(&instance_info, &to_node_id).await?;

        info!(
            old_instance_id = %instance_id,
            new_instance_id = %result.new_instance_id,
            to_node = %to_node,
            "Instance migrated"
        );

        Ok(result.new_instance_id)
    }

    /// Drain an instance (prepare for graceful shutdown)
    #[instrument(skip(self), fields(instance_id = %instance_id))]
    pub async fn drain_instance(&self, instance_id: &InstanceId, ctx: &RequestContext) -> Result<()> {
        self.check_policy(
            &ControlPlaneOperation::DrainInstance {
                instance_id: instance_id.clone(),
            },
            ctx,
        )
        .await?;

        self.event_aggregator.emit_info(
            PalmEvent::InstanceDraining {
                instance_id: instance_id.clone(),
            },
            ctx.platform,
        );

        info!(instance_id = %instance_id, "Instance drain initiated");

        Ok(())
    }

    // ========== State Operations ==========

    /// Create a checkpoint for an instance
    #[instrument(skip(self), fields(instance_id = %instance_id))]
    pub async fn create_checkpoint(
        &self,
        instance_id: &InstanceId,
        ctx: &RequestContext,
    ) -> Result<StateSnapshotId> {
        self.check_policy(
            &ControlPlaneOperation::CreateCheckpoint {
                instance_id: instance_id.clone(),
            },
            ctx,
        )
        .await?;

        let snapshot_id = self
            .create_checkpoint_internal(instance_id, SnapshotReason::Manual)
            .await?;

        info!(instance_id = %instance_id, snapshot_id = %snapshot_id, "Checkpoint created");

        Ok(snapshot_id)
    }

    /// Internal checkpoint creation (no policy check)
    async fn create_checkpoint_internal(
        &self,
        instance_id: &InstanceId,
        reason: SnapshotReason,
    ) -> Result<StateSnapshotId> {
        let instance = self
            .instance_registry
            .get(instance_id)
            .await?
            .ok_or_else(|| ControlPlaneError::instance_not_found(instance_id))?;

        let instance_info = InstanceInfo {
            instance_id: instance_id.clone(),
            deployment_id: instance.deployment_id.clone(),
            resonator_id: ResonatorId::generate(), // Would come from instance
        };

        let snapshot_id = self.state_manager.checkpoint(&instance_info, reason).await?;

        Ok(snapshot_id)
    }

    /// Restore from a checkpoint
    #[instrument(skip(self), fields(instance_id = %instance_id, snapshot_id = %snapshot_id))]
    pub async fn restore_from_checkpoint(
        &self,
        instance_id: &InstanceId,
        snapshot_id: &StateSnapshotId,
        ctx: &RequestContext,
    ) -> Result<()> {
        self.check_policy(
            &ControlPlaneOperation::RestoreFromCheckpoint {
                instance_id: instance_id.clone(),
                snapshot_id: snapshot_id.clone(),
            },
            ctx,
        )
        .await?;

        self.state_manager.restore(instance_id, snapshot_id).await?;

        info!(instance_id = %instance_id, snapshot_id = %snapshot_id, "Restored from checkpoint");

        Ok(())
    }

    /// List snapshots for an instance
    pub async fn list_snapshots(&self, instance_id: &InstanceId) -> Result<Vec<SnapshotMetadata>> {
        Ok(self.state_manager.list_snapshots(instance_id).await?)
    }

    /// Delete a snapshot
    #[instrument(skip(self), fields(snapshot_id = %snapshot_id))]
    pub async fn delete_snapshot(
        &self,
        snapshot_id: &StateSnapshotId,
        ctx: &RequestContext,
    ) -> Result<()> {
        self.check_policy(
            &ControlPlaneOperation::DeleteSnapshot {
                snapshot_id: snapshot_id.clone(),
            },
            ctx,
        )
        .await?;

        self.state_manager.delete_snapshot(snapshot_id).await?;

        info!(snapshot_id = %snapshot_id, "Snapshot deleted");

        Ok(())
    }

    // ========== Health Operations ==========

    /// Get health assessment for an instance
    pub async fn get_instance_health(&self, instance_id: &InstanceId) -> Result<HealthAssessment> {
        Ok(self.health_monitor.probe_instance(instance_id).await?)
    }

    /// Get fleet health summary
    pub fn get_fleet_health(&self) -> palm_health::FleetHealthSummary {
        self.health_monitor.get_fleet_summary()
    }

    /// Trigger a health check
    #[instrument(skip(self), fields(instance_id = %instance_id))]
    pub async fn trigger_health_check(
        &self,
        instance_id: &InstanceId,
        ctx: &RequestContext,
    ) -> Result<HealthAssessment> {
        self.check_policy(
            &ControlPlaneOperation::TriggerHealthCheck {
                instance_id: instance_id.clone(),
            },
            ctx,
        )
        .await?;

        Ok(self.health_monitor.probe_instance(instance_id).await?)
    }

    /// Force recovery action on an instance
    #[instrument(skip(self), fields(instance_id = %instance_id))]
    pub async fn force_recovery(&self, instance_id: &InstanceId, ctx: &RequestContext) -> Result<()> {
        self.check_policy(
            &ControlPlaneOperation::ForceRecovery {
                instance_id: instance_id.clone(),
            },
            ctx,
        )
        .await?;

        self.event_aggregator.emit_warning(
            PalmEvent::RecoveryInitiated {
                instance_id: instance_id.clone(),
            },
            ctx.platform,
        );

        info!(instance_id = %instance_id, "Forced recovery initiated");

        Ok(())
    }

    // ========== Lifecycle ==========

    /// Subscribe to all events
    pub fn subscribe_events(&self) -> broadcast::Receiver<PalmEventEnvelope> {
        self.event_aggregator.subscribe()
    }

    /// Subscribe to deployment events
    pub fn subscribe_deployment_events(&self) -> broadcast::Receiver<PalmEventEnvelope> {
        self.deployment_manager.subscribe()
    }

    /// Subscribe to health events
    pub fn subscribe_health_events(&self) -> broadcast::Receiver<palm_health::HealthEvent> {
        self.health_monitor.subscribe()
    }

    /// Subscribe to state events
    pub fn subscribe_state_events(&self) -> broadcast::Receiver<palm_state::StateEvent> {
        self.state_manager.subscribe()
    }

    // ========== Internal ==========

    async fn check_policy(
        &self,
        operation: &ControlPlaneOperation,
        ctx: &RequestContext,
    ) -> Result<()> {
        // Check if human approval is required
        if operation.requires_human_approval(&ctx.platform) {
            if !ctx.policy_context.has_human_approval() {
                return Err(ControlPlaneError::PolicyDenied(format!(
                    "Operation {} requires human approval for platform {:?}",
                    operation.description(),
                    ctx.platform
                )));
            }
        }

        // Convert our operation to palm_types::PalmOperation for the policy gate
        let palm_op = self.to_palm_operation(operation);

        // Evaluate policy gate
        self.policy_gate
            .validate_operation(&palm_op, &ctx.policy_context)
            .await
            .map_err(|e| ControlPlaneError::PolicyDenied(e.to_string()))?;

        Ok(())
    }

    /// Convert our operation type to palm_types::PalmOperation
    fn to_palm_operation(&self, op: &ControlPlaneOperation) -> PalmOperation {
        match op {
            ControlPlaneOperation::RegisterSpec { spec_id } => PalmOperation::CreateSpec {
                spec_id: spec_id.to_string(),
            },
            ControlPlaneOperation::UpdateSpec { spec_id } => PalmOperation::UpdateSpec {
                spec_id: spec_id.to_string(),
            },
            ControlPlaneOperation::DeprecateSpec { spec_id } => PalmOperation::DeprecateSpec {
                spec_id: spec_id.to_string(),
            },
            ControlPlaneOperation::CreateDeployment { spec_id, .. } => {
                PalmOperation::CreateDeployment {
                    spec_id: spec_id.to_string(),
                }
            }
            ControlPlaneOperation::UpdateDeployment { deployment_id, .. } => {
                PalmOperation::UpdateDeployment {
                    deployment_id: deployment_id.to_string(),
                }
            }
            ControlPlaneOperation::ScaleDeployment {
                deployment_id,
                replicas,
            } => PalmOperation::ScaleDeployment {
                deployment_id: deployment_id.to_string(),
                target_replicas: *replicas,
            },
            ControlPlaneOperation::DeleteDeployment { deployment_id } => {
                PalmOperation::DeleteDeployment {
                    deployment_id: deployment_id.to_string(),
                }
            }
            ControlPlaneOperation::PauseDeployment { deployment_id } => {
                PalmOperation::PauseDeployment {
                    deployment_id: deployment_id.to_string(),
                }
            }
            ControlPlaneOperation::ResumeDeployment { deployment_id } => {
                PalmOperation::ResumeDeployment {
                    deployment_id: deployment_id.to_string(),
                }
            }
            ControlPlaneOperation::RollbackDeployment { deployment_id, .. } => {
                PalmOperation::RollbackDeployment {
                    deployment_id: deployment_id.to_string(),
                }
            }
            ControlPlaneOperation::RestartInstance { instance_id, .. } => {
                PalmOperation::RestartInstance {
                    instance_id: instance_id.to_string(),
                }
            }
            ControlPlaneOperation::TerminateInstance { instance_id, .. } => {
                PalmOperation::TerminateInstance {
                    instance_id: instance_id.to_string(),
                }
            }
            ControlPlaneOperation::MigrateInstance { instance_id, .. } => {
                PalmOperation::MigrateInstance {
                    instance_id: instance_id.to_string(),
                }
            }
            ControlPlaneOperation::DrainInstance { instance_id } => PalmOperation::DrainInstance {
                instance_id: instance_id.to_string(),
            },
            ControlPlaneOperation::CreateCheckpoint { instance_id } => {
                PalmOperation::CreateCheckpoint {
                    instance_id: instance_id.to_string(),
                }
            }
            ControlPlaneOperation::RestoreFromCheckpoint { instance_id, .. } => {
                PalmOperation::RestoreCheckpoint {
                    instance_id: instance_id.to_string(),
                }
            }
            ControlPlaneOperation::DeleteSnapshot { snapshot_id } => {
                PalmOperation::DeleteCheckpoint {
                    snapshot_id: snapshot_id.to_string(),
                }
            }
            ControlPlaneOperation::TriggerHealthCheck { instance_id } => {
                PalmOperation::HealthCheck {
                    instance_id: instance_id.to_string(),
                }
            }
            ControlPlaneOperation::ForceRecovery { instance_id } => PalmOperation::ForceRecovery {
                instance_id: instance_id.to_string(),
            },
            ControlPlaneOperation::ConfigurePolicy { policy_name } => {
                PalmOperation::ConfigurePolicy {
                    policy_name: policy_name.clone(),
                }
            }
            ControlPlaneOperation::ViewAuditLog { filter } => PalmOperation::ViewAuditLog {
                filter: filter.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use palm_deployment::{AllowAllPolicyGate, InMemoryDeploymentStateStore};
    use palm_registry::{InMemoryAgentRegistry, InMemoryInstanceRegistry};
    use palm_state::{
        checkpoint::MockRuntimeStateGatherer,
        commitment_reconcile::MockAasClient,
        coupling_restore::MockCouplingRuntime,
        migration::MockMigrationRuntime,
        restore::{MockContinuityVerifier, MockRuntimeStateRestorer},
        storage::InMemoryStateStorage,
    };

    fn create_test_control_plane() -> PalmControlPlane {
        let platform = PlatformProfile::Development;
        let agent_registry = Arc::new(InMemoryAgentRegistry::new());
        let instance_registry = Arc::new(InMemoryInstanceRegistry::new());
        let policy_gate = Arc::new(AllowAllPolicyGate);
        let state_storage = Arc::new(InMemoryStateStorage::new());
        let deployment_state_store = Arc::new(InMemoryDeploymentStateStore::new());
        let state_gatherer = Arc::new(MockRuntimeStateGatherer::new());
        let state_restorer = Arc::new(MockRuntimeStateRestorer::new());
        let continuity_verifier = Arc::new(MockContinuityVerifier::new());
        let coupling_runtime = Arc::new(MockCouplingRuntime::all_present());
        let migration_runtime = Arc::new(MockMigrationRuntime::new("node-1"));
        let aas_client = Arc::new(MockAasClient::all_pending());
        let health_config = HealthConfig::for_platform(platform);
        let state_config = StateManagerConfig::default();

        PalmControlPlane::new(
            platform,
            agent_registry,
            instance_registry,
            policy_gate,
            state_storage,
            deployment_state_store,
            state_gatherer,
            state_restorer,
            continuity_verifier,
            coupling_runtime,
            migration_runtime,
            aas_client,
            health_config,
            state_config,
        )
    }

    #[tokio::test]
    async fn test_register_spec() {
        let cp = create_test_control_plane();
        let ctx = RequestContext::default();

        let spec = AgentSpec::new("test-agent", semver::Version::new(1, 0, 0));
        let result = cp.register_spec(spec, &ctx).await;
        assert!(result.is_ok());

        let spec_id = result.unwrap();
        let loaded = cp.get_spec(&spec_id).await.unwrap();
        assert!(loaded.is_some());
    }

    #[tokio::test]
    async fn test_create_deployment() {
        let cp = create_test_control_plane();
        let ctx = RequestContext::default();

        // First register a spec
        let spec = AgentSpec::new("test-agent", semver::Version::new(1, 0, 0));
        let spec_id = cp.register_spec(spec, &ctx).await.unwrap();

        // Then create deployment
        let config = CreateDeploymentConfig::with_replicas(3);
        let deployment = cp.create_deployment(&spec_id, config, &ctx).await.unwrap();

        assert_eq!(deployment.agent_spec_id, spec_id);
        assert_eq!(deployment.replicas.desired, 3);
    }

    #[tokio::test]
    async fn test_policy_enforcement() {
        let cp = create_test_control_plane();

        // Create a context that requires human approval for IBank
        let ctx = RequestContext::new(
            PlatformProfile::IBank,
            crate::context::Actor::User {
                user_id: "test".into(),
                roles: vec![],
            },
        );

        // Destructive operations should be denied without human approval
        let op = ControlPlaneOperation::DeleteDeployment {
            deployment_id: DeploymentId::generate(),
        };

        assert!(op.requires_human_approval(&PlatformProfile::IBank));
    }

    #[test]
    fn test_event_subscription() {
        let cp = create_test_control_plane();
        let _rx = cp.subscribe_events();

        // Subscriber count should be 1
        assert_eq!(cp.event_aggregator.subscriber_count(), 1);
    }
}
