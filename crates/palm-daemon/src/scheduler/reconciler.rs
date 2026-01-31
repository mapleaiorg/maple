//! Reconciliation loop and scheduler

use crate::config::SchedulerConfig;
use crate::storage::{DeploymentStorage, InstanceStorage, InMemoryStorage};
use palm_types::{
    instance::{
        AgentInstance, HealthStatus, InstanceMetrics, InstancePlacement, InstanceStatus,
        ResonatorIdRef,
    },
    DeploymentId, DeploymentStatus, EventSeverity, EventSource, InstanceId, PalmEvent,
    PalmEventEnvelope, PlatformProfile,
};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{interval, Duration};

/// Scheduler state
pub struct Scheduler {
    config: SchedulerConfig,
    storage: Arc<InMemoryStorage>,
    event_tx: broadcast::Sender<PalmEventEnvelope>,
    reconcile_tx: mpsc::Sender<()>,
    running: Arc<RwLock<bool>>,
    platform: PlatformProfile,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new(
        config: SchedulerConfig,
        storage: Arc<InMemoryStorage>,
        event_tx: broadcast::Sender<PalmEventEnvelope>,
    ) -> (Arc<Self>, mpsc::Receiver<()>) {
        Self::with_platform(config, storage, event_tx, PlatformProfile::Development)
    }

    /// Create a new scheduler with a specific platform profile
    pub fn with_platform(
        config: SchedulerConfig,
        storage: Arc<InMemoryStorage>,
        event_tx: broadcast::Sender<PalmEventEnvelope>,
        platform: PlatformProfile,
    ) -> (Arc<Self>, mpsc::Receiver<()>) {
        let (reconcile_tx, reconcile_rx) = mpsc::channel(10);

        let scheduler = Arc::new(Self {
            config,
            storage,
            event_tx,
            reconcile_tx,
            running: Arc::new(RwLock::new(false)),
            platform,
        });

        (scheduler, reconcile_rx)
    }

    /// Trigger an immediate reconciliation
    pub async fn trigger_reconcile(&self) {
        let _ = self.reconcile_tx.send(()).await;
    }

    /// Start the scheduler background tasks
    pub async fn start(self: Arc<Self>, mut reconcile_rx: mpsc::Receiver<()>) {
        {
            let mut running = self.running.write().await;
            *running = true;
        }

        tracing::info!("Scheduler started");

        // Spawn reconciliation loop
        let reconcile_scheduler = self.clone();
        let reconcile_handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(
                reconcile_scheduler.config.reconcile_interval_secs,
            ));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = reconcile_scheduler.reconcile().await {
                            tracing::error!(error = %e, "Reconciliation failed");
                        }
                    }
                    Some(_) = reconcile_rx.recv() => {
                        if let Err(e) = reconcile_scheduler.reconcile().await {
                            tracing::error!(error = %e, "Triggered reconciliation failed");
                        }
                    }
                    else => break,
                }

                let running = reconcile_scheduler.running.read().await;
                if !*running {
                    break;
                }
            }
        });

        // Spawn health check loop
        let health_scheduler = self.clone();
        let health_handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(
                health_scheduler.config.health_check_interval_secs,
            ));

            loop {
                interval.tick().await;

                let running = health_scheduler.running.read().await;
                if !*running {
                    break;
                }

                if let Err(e) = health_scheduler.check_health().await {
                    tracing::error!(error = %e, "Health check failed");
                }
            }
        });

        // Wait for shutdown
        tokio::select! {
            _ = reconcile_handle => {}
            _ = health_handle => {}
        }

        tracing::info!("Scheduler stopped");
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }

    /// Perform reconciliation for all deployments
    async fn reconcile(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let deployments = self.storage.list_deployments().await?;

        for deployment in deployments {
            // Skip paused deployments
            if matches!(deployment.status, DeploymentStatus::Paused { .. }) {
                continue;
            }

            if let Err(e) = self.reconcile_deployment(&deployment.id).await {
                tracing::error!(
                    deployment_id = %deployment.id,
                    error = %e,
                    "Failed to reconcile deployment"
                );

                self.emit_event(
                    PalmEvent::DeploymentFailed {
                        deployment_id: deployment.id.clone(),
                        reason: e.to_string(),
                    },
                    EventSeverity::Error,
                );
            }
        }

        Ok(())
    }

    /// Reconcile a single deployment
    async fn reconcile_deployment(
        &self,
        deployment_id: &DeploymentId,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let deployment = self
            .storage
            .get_deployment(deployment_id)
            .await?
            .ok_or("Deployment not found")?;

        let current_instances = self
            .storage
            .list_instances_for_deployment(deployment_id)
            .await?;

        let desired = deployment.replicas.desired as usize;
        let current = current_instances.len();

        tracing::debug!(
            deployment_id = %deployment_id,
            desired = desired,
            current = current,
            "Reconciling deployment"
        );

        if current < desired {
            // Scale up
            let to_create = desired - current;
            tracing::info!(
                deployment_id = %deployment_id,
                count = to_create,
                "Scaling up deployment"
            );

            for _ in 0..to_create {
                let instance = self.create_instance(deployment_id).await?;
                self.emit_event(
                    PalmEvent::InstanceCreated {
                        instance_id: instance.id.clone(),
                        deployment_id: deployment_id.clone(),
                    },
                    EventSeverity::Info,
                );
            }

            // Update deployment status
            let mut deployment = deployment;
            deployment.status = DeploymentStatus::Completed {
                completed_at: chrono::Utc::now(),
            };
            deployment.replicas.current_total = desired as u32;
            deployment.replicas.current_healthy = desired as u32;
            deployment.updated_at = chrono::Utc::now();
            self.storage.upsert_deployment(deployment).await?;
        } else if current > desired {
            // Scale down
            let to_remove = current - desired;
            tracing::info!(
                deployment_id = %deployment_id,
                count = to_remove,
                "Scaling down deployment"
            );

            // Remove excess instances (prefer unhealthy ones first)
            let mut to_delete: Vec<_> = current_instances
                .iter()
                .filter(|i| !i.health.is_healthy())
                .take(to_remove)
                .collect();

            // If we need more, take healthy ones
            if to_delete.len() < to_remove {
                let remaining = to_remove - to_delete.len();
                let healthy: Vec<_> = current_instances
                    .iter()
                    .filter(|i| i.health.is_healthy())
                    .take(remaining)
                    .collect();
                to_delete.extend(healthy);
            }

            for instance in to_delete {
                self.storage.delete_instance(&instance.id).await?;
                self.emit_event(
                    PalmEvent::InstanceTerminated {
                        instance_id: instance.id.clone(),
                        exit_code: Some(0),
                    },
                    EventSeverity::Info,
                );
            }
        }

        Ok(())
    }

    /// Create a new instance for a deployment
    async fn create_instance(
        &self,
        deployment_id: &DeploymentId,
    ) -> Result<AgentInstance, Box<dyn std::error::Error + Send + Sync>> {
        let instance_id = InstanceId::generate();

        let instance = AgentInstance {
            id: instance_id,
            deployment_id: deployment_id.clone(),
            resonator_id: ResonatorIdRef::new(format!("resonator-{}", uuid::Uuid::new_v4())),
            status: InstanceStatus::Running,
            health: HealthStatus::Healthy,
            placement: InstancePlacement::default(),
            metrics: InstanceMetrics::default(),
            started_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
        };

        self.storage.upsert_instance(instance.clone()).await?;

        tracing::info!(instance_id = %instance.id, "Created instance");

        Ok(instance)
    }

    /// Perform health checks on all instances
    async fn check_health(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let instances = self.storage.list_instances().await?;

        for instance in instances {
            // Simulate health check - in real implementation would check actual instance
            let now = chrono::Utc::now();
            let heartbeat_age = now - instance.last_heartbeat;

            // If heartbeat is too old, mark as unhealthy
            if heartbeat_age.num_seconds() > 30 {
                let mut updated = instance.clone();
                updated.health = HealthStatus::Unhealthy {
                    reasons: vec!["Heartbeat timeout".to_string()],
                };
                self.storage.upsert_instance(updated).await?;

                self.emit_event(
                    PalmEvent::HealthProbeFailed {
                        instance_id: instance.id.clone(),
                        probe_type: "heartbeat".to_string(),
                        reason: "Heartbeat timeout".to_string(),
                    },
                    EventSeverity::Warning,
                );

                // Auto-heal if enabled
                if self.config.auto_healing_enabled {
                    self.auto_heal_instance(&instance).await?;
                }
            }
        }

        Ok(())
    }

    /// Auto-heal an unhealthy instance
    async fn auto_heal_instance(
        &self,
        instance: &AgentInstance,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            instance_id = %instance.id,
            "Auto-healing unhealthy instance"
        );

        // Delete the unhealthy instance
        self.storage.delete_instance(&instance.id).await?;

        self.emit_event(
            PalmEvent::InstanceTerminated {
                instance_id: instance.id.clone(),
                exit_code: Some(1),
            },
            EventSeverity::Info,
        );

        // Reconciliation will create a replacement
        self.trigger_reconcile().await;

        Ok(())
    }

    /// Emit an event
    fn emit_event(&self, event: PalmEvent, severity: EventSeverity) {
        let envelope = PalmEventEnvelope::new(event, EventSource::Scheduler, self.platform.clone())
            .with_actor("scheduler");

        let _ = self.event_tx.send(envelope);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use palm_types::{AgentSpecId, Deployment, DeploymentStrategy, ReplicaConfig};

    fn create_test_deployment() -> Deployment {
        Deployment {
            id: DeploymentId::generate(),
            agent_spec_id: AgentSpecId::new("test-spec"),
            version: semver::Version::new(1, 0, 0),
            platform: PlatformProfile::Development,
            strategy: DeploymentStrategy::default(),
            status: DeploymentStatus::Pending,
            replicas: ReplicaConfig::new(3),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_reconcile_scales_up() {
        let storage = Arc::new(InMemoryStorage::new());
        let (event_tx, _rx) = broadcast::channel(100);
        let config = SchedulerConfig::default();

        let (scheduler, _reconcile_rx) = Scheduler::new(config, storage.clone(), event_tx);

        // Create deployment with 3 replicas
        let deployment = create_test_deployment();
        storage.upsert_deployment(deployment.clone()).await.unwrap();

        // Reconcile
        scheduler
            .reconcile_deployment(&deployment.id)
            .await
            .unwrap();

        // Should have 3 instances
        let instances = storage
            .list_instances_for_deployment(&deployment.id)
            .await
            .unwrap();
        assert_eq!(instances.len(), 3);
    }

    #[tokio::test]
    async fn test_reconcile_scales_down() {
        let storage = Arc::new(InMemoryStorage::new());
        let (event_tx, _rx) = broadcast::channel(100);
        let config = SchedulerConfig::default();

        let (scheduler, _reconcile_rx) = Scheduler::new(config, storage.clone(), event_tx);

        // Create deployment with 1 replica
        let mut deployment = create_test_deployment();
        deployment.replicas = ReplicaConfig::new(1);
        storage.upsert_deployment(deployment.clone()).await.unwrap();

        // Create 3 instances manually
        for _ in 0..3 {
            let instance = AgentInstance {
                id: InstanceId::generate(),
                deployment_id: deployment.id.clone(),
                resonator_id: ResonatorIdRef::new("test"),
                status: InstanceStatus::Running,
                health: HealthStatus::Healthy,
                placement: InstancePlacement::default(),
                metrics: InstanceMetrics::default(),
                started_at: chrono::Utc::now(),
                last_heartbeat: chrono::Utc::now(),
            };
            storage.upsert_instance(instance).await.unwrap();
        }

        // Reconcile
        scheduler
            .reconcile_deployment(&deployment.id)
            .await
            .unwrap();

        // Should have 1 instance
        let instances = storage
            .list_instances_for_deployment(&deployment.id)
            .await
            .unwrap();
        assert_eq!(instances.len(), 1);
    }
}
