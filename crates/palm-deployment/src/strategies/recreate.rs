//! Recreate deployment strategy

use super::executor::{DeploymentExecutor, DeploymentResult};
use crate::context::DeploymentContext;
use crate::error::Result;
use async_trait::async_trait;
use palm_types::{AgentInstance, AgentSpec, Deployment};
use std::time::Duration;
use tracing::info;

/// Recreate deployment executor
///
/// Terminates all existing instances before creating new ones.
/// Simple but causes downtime.
pub struct RecreateDeploymentExecutor;

impl RecreateDeploymentExecutor {
    /// Create a new recreate deployment executor
    pub fn new() -> Self {
        Self
    }
}

impl Default for RecreateDeploymentExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DeploymentExecutor for RecreateDeploymentExecutor {
    async fn execute(
        &self,
        deployment: &Deployment,
        current_instances: Vec<AgentInstance>,
        _target_spec: &AgentSpec,
        ctx: &DeploymentContext,
    ) -> Result<DeploymentResult> {
        let target_count = deployment.replicas.desired;

        info!(
            deployment_id = %deployment.id,
            target_count = target_count,
            current_count = current_instances.len(),
            "Starting recreate deployment"
        );

        // Phase 1: Terminate all existing instances
        let terminated_count = current_instances.len() as u32;
        for instance in current_instances {
            ctx.terminate_instance_gracefully(&instance).await?;
        }

        info!(
            deployment_id = %deployment.id,
            terminated = terminated_count,
            "All existing instances terminated"
        );

        // Phase 2: Create all new instances
        let mut new_instances = Vec::with_capacity(target_count as usize);
        for _ in 0..target_count {
            match ctx.create_instance().await {
                Ok(instance) => {
                    new_instances.push(instance);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to create instance");
                }
            }
        }

        // Phase 3: Wait for all instances to be healthy
        let mut healthy_count = 0u32;
        for instance in &new_instances {
            ctx.wait_for_presence(instance, Duration::from_secs(60))
                .await?;
            if ctx
                .wait_for_healthy(instance, Duration::from_secs(120))
                .await?
            {
                healthy_count += 1;
            }
        }

        info!(
            deployment_id = %deployment.id,
            healthy_instances = healthy_count,
            terminated_instances = terminated_count,
            "Recreate deployment completed"
        );

        if healthy_count == 0 && target_count > 0 {
            return Ok(DeploymentResult::Failed {
                reason: "No healthy instances created".into(),
                rollback_recommended: true,
            });
        }

        if healthy_count < target_count {
            return Ok(DeploymentResult::PartialSuccess {
                healthy_instances: healthy_count,
                failed_instances: target_count - healthy_count,
                reason: "Some instances failed to become healthy".into(),
            });
        }

        Ok(DeploymentResult::Success {
            healthy_instances: healthy_count,
            terminated_instances: terminated_count,
        })
    }

    fn name(&self) -> &str {
        "recreate"
    }

    fn supports_pause(&self) -> bool {
        false // Recreate should complete atomically to minimize downtime
    }
}
