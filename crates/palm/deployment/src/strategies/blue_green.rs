//! Blue-Green deployment strategy

use super::executor::{DeploymentExecutor, DeploymentResult};
use crate::context::DeploymentContext;
use crate::error::{DeploymentError, Result};
use async_trait::async_trait;
use palm_types::{AgentInstance, AgentSpec, Deployment};
use std::time::Duration;
use tracing::info;

/// Blue-Green deployment executor
///
/// Creates a complete parallel deployment (green), validates it,
/// then switches traffic from the old (blue) deployment.
pub struct BlueGreenDeploymentExecutor {
    /// Health threshold to allow switching (0.0 to 1.0)
    switch_threshold: f64,
    /// Validation period before switching
    validation_period: Duration,
}

impl BlueGreenDeploymentExecutor {
    /// Create a new blue-green deployment executor
    pub fn new(switch_threshold: f64, validation_period: Duration) -> Self {
        Self {
            switch_threshold,
            validation_period,
        }
    }
}

#[async_trait]
impl DeploymentExecutor for BlueGreenDeploymentExecutor {
    async fn execute(
        &self,
        deployment: &Deployment,
        blue_instances: Vec<AgentInstance>,
        _target_spec: &AgentSpec,
        ctx: &DeploymentContext,
    ) -> Result<DeploymentResult> {
        let target_count = deployment.replicas.desired;

        info!(
            deployment_id = %deployment.id,
            target_count = target_count,
            blue_count = blue_instances.len(),
            switch_threshold = self.switch_threshold,
            "Starting blue-green deployment"
        );

        // Phase 1: Create all green instances
        let mut green_instances = Vec::with_capacity(target_count as usize);
        for _ in 0..target_count {
            let instance = ctx.create_instance().await?;
            green_instances.push(instance);
        }

        // Phase 2: Wait for green instances to be healthy
        let mut healthy_count = 0u32;
        for instance in &green_instances {
            ctx.wait_for_presence(instance, Duration::from_secs(60))
                .await?;
            if ctx
                .wait_for_healthy(instance, Duration::from_secs(120))
                .await?
            {
                healthy_count += 1;
            }
        }

        let health_ratio = healthy_count as f64 / target_count as f64;
        if health_ratio < self.switch_threshold {
            // Rollback: terminate green, keep blue
            for instance in green_instances {
                ctx.terminate_instance_forcefully(&instance).await?;
            }
            return Err(DeploymentError::HealthThresholdNotMet {
                required: self.switch_threshold,
                actual: health_ratio,
            });
        }

        // Phase 3: Validation period - run both in parallel
        info!(
            deployment_id = %deployment.id,
            validation_period = ?self.validation_period,
            "Starting validation period"
        );
        tokio::time::sleep(self.validation_period).await;

        // Re-check health after validation
        let still_healthy = ctx.count_healthy(&green_instances).await?;
        let post_validation_ratio = still_healthy as f64 / target_count as f64;
        if post_validation_ratio < self.switch_threshold {
            // Rollback
            for instance in green_instances {
                ctx.terminate_instance_forcefully(&instance).await?;
            }
            return Err(DeploymentError::ValidationFailed);
        }

        // Phase 4: Switch traffic (discovery routing, not HTTP)
        ctx.switch_traffic(&blue_instances, &green_instances)
            .await?;

        // Phase 5: Terminate blue instances
        let terminated = blue_instances.len() as u32;
        for instance in blue_instances {
            ctx.terminate_instance_gracefully(&instance).await?;
        }

        info!(
            deployment_id = %deployment.id,
            healthy_instances = still_healthy,
            terminated_instances = terminated,
            "Blue-green deployment completed"
        );

        Ok(DeploymentResult::Success {
            healthy_instances: still_healthy,
            terminated_instances: terminated,
        })
    }

    fn name(&self) -> &str {
        "blue-green"
    }

    fn supports_pause(&self) -> bool {
        false // Blue-green should complete atomically
    }
}
