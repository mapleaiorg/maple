//! Rolling deployment strategy

use super::executor::{DeploymentExecutor, DeploymentResult};
use crate::context::DeploymentContext;
use crate::error::Result;
use async_trait::async_trait;
use palm_types::{AgentInstance, AgentSpec, Deployment};
use std::time::Duration;
use tracing::{info, warn};

/// Rolling deployment executor
///
/// Gradually replaces old instances with new ones, maintaining availability.
pub struct RollingDeploymentExecutor {
    /// Maximum instances that can be unavailable during update
    max_unavailable: u32,
    /// Maximum extra instances during update
    max_surge: u32,
    /// Minimum seconds an instance must be ready
    min_ready_seconds: u32,
}

impl RollingDeploymentExecutor {
    /// Create a new rolling deployment executor
    pub fn new(max_unavailable: u32, max_surge: u32, min_ready_seconds: u32) -> Self {
        Self {
            max_unavailable,
            max_surge,
            min_ready_seconds,
        }
    }
}

#[async_trait]
impl DeploymentExecutor for RollingDeploymentExecutor {
    async fn execute(
        &self,
        deployment: &Deployment,
        current_instances: Vec<AgentInstance>,
        _target_spec: &AgentSpec,
        ctx: &DeploymentContext,
    ) -> Result<DeploymentResult> {
        let target_count = deployment.replicas.desired;
        let mut new_instances: Vec<AgentInstance> = Vec::new();
        let mut old_instances = current_instances;
        let mut terminated_count = 0u32;

        info!(
            deployment_id = %deployment.id,
            target_count = target_count,
            current_count = old_instances.len(),
            max_unavailable = self.max_unavailable,
            max_surge = self.max_surge,
            "Starting rolling deployment"
        );

        // Calculate bounds
        let min_available = target_count.saturating_sub(self.max_unavailable);
        let max_total = target_count + self.max_surge;

        let mut iterations = 0;
        let max_iterations = (target_count * 3) as usize;

        loop {
            iterations += 1;
            if iterations > max_iterations {
                return Ok(DeploymentResult::Failed {
                    reason: "Max iterations exceeded".into(),
                    rollback_recommended: true,
                });
            }

            let total_instances = new_instances.len() as u32 + old_instances.len() as u32;
            let healthy_new = ctx.count_healthy(&new_instances).await?;
            let healthy_old = ctx.count_healthy(&old_instances).await?;
            let total_healthy = healthy_new + healthy_old;

            // Phase 1: Create new instances (if under max_total and need more)
            if healthy_new < target_count && total_instances < max_total {
                let can_create = (max_total - total_instances).min(self.max_surge);

                for _ in 0..can_create {
                    match ctx.create_instance().await {
                        Ok(instance) => {
                            // Wait for presence
                            if let Err(e) = ctx
                                .wait_for_presence(&instance, Duration::from_secs(60))
                                .await
                            {
                                warn!(instance_id = %instance.id, error = %e, "Presence timeout");
                                ctx.terminate_instance_forcefully(&instance).await?;
                                continue;
                            }

                            // Wait for health
                            let timeout =
                                Duration::from_secs(self.min_ready_seconds as u64 + 30);
                            match ctx.wait_for_healthy(&instance, timeout).await {
                                Ok(true) => {
                                    new_instances.push(instance);
                                    info!(
                                        healthy_new = new_instances.len(),
                                        "New instance healthy"
                                    );
                                }
                                Ok(false) => {
                                    warn!(
                                        instance_id = %instance.id,
                                        "Instance failed health check"
                                    );
                                    ctx.terminate_instance_forcefully(&instance).await?;
                                }
                                Err(e) => {
                                    warn!(
                                        instance_id = %instance.id,
                                        error = %e,
                                        "Health check error"
                                    );
                                    ctx.terminate_instance_forcefully(&instance).await?;
                                }
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to create instance");
                        }
                    }
                }
            }

            // Phase 2: Terminate old instances (if safe to do so)
            let healthy_new = ctx.count_healthy(&new_instances).await?;
            if !old_instances.is_empty() && healthy_new + healthy_old > min_available {
                let can_terminate =
                    ((healthy_new + healthy_old) - min_available).min(old_instances.len() as u32);

                for _ in 0..can_terminate {
                    if let Some(instance) = old_instances.pop() {
                        ctx.terminate_instance_gracefully(&instance).await?;
                        terminated_count += 1;
                    }
                }
            }

            // Check completion
            let healthy_new = ctx.count_healthy(&new_instances).await?;
            if healthy_new >= target_count && old_instances.is_empty() {
                info!(
                    deployment_id = %deployment.id,
                    healthy_instances = healthy_new,
                    terminated_instances = terminated_count,
                    "Rolling deployment completed"
                );
                return Ok(DeploymentResult::Success {
                    healthy_instances: healthy_new,
                    terminated_instances: terminated_count,
                });
            }

            // Progress check to avoid infinite loops
            if new_instances.len() as u32 >= target_count * 2 {
                return Ok(DeploymentResult::Failed {
                    reason: "Too many failed instances".into(),
                    rollback_recommended: true,
                });
            }

            // Brief pause between iterations
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    fn name(&self) -> &str {
        "rolling"
    }
}
