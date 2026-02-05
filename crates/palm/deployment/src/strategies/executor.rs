//! Deployment executor trait

use crate::context::DeploymentContext;
use crate::error::Result;
use async_trait::async_trait;
use palm_types::{AgentInstance, AgentSpec, Deployment};

/// Result of a deployment execution
#[derive(Debug)]
pub enum DeploymentResult {
    /// Deployment completed successfully
    Success {
        /// Number of healthy instances
        healthy_instances: u32,
        /// Number of terminated instances
        terminated_instances: u32,
    },

    /// Deployment partially succeeded
    PartialSuccess {
        /// Number of healthy instances
        healthy_instances: u32,
        /// Number of failed instances
        failed_instances: u32,
        /// Reason for partial success
        reason: String,
    },

    /// Deployment failed
    Failed {
        /// Reason for failure
        reason: String,
        /// Whether rollback is recommended
        rollback_recommended: bool,
    },
}

impl DeploymentResult {
    /// Check if the deployment was successful
    pub fn is_success(&self) -> bool {
        matches!(self, DeploymentResult::Success { .. })
    }

    /// Check if rollback is recommended
    pub fn should_rollback(&self) -> bool {
        matches!(
            self,
            DeploymentResult::Failed {
                rollback_recommended: true,
                ..
            }
        )
    }
}

/// Trait for deployment strategy executors
#[async_trait]
pub trait DeploymentExecutor: Send + Sync {
    /// Execute the deployment strategy
    ///
    /// # Arguments
    ///
    /// * `deployment` - The deployment to execute
    /// * `current_instances` - Existing instances to replace/update
    /// * `target_spec` - The target agent spec
    /// * `ctx` - Deployment context for instance operations
    ///
    /// # Returns
    ///
    /// The result of the deployment execution
    async fn execute(
        &self,
        deployment: &Deployment,
        current_instances: Vec<AgentInstance>,
        target_spec: &AgentSpec,
        ctx: &DeploymentContext,
    ) -> Result<DeploymentResult>;

    /// Strategy name for logging
    fn name(&self) -> &str;

    /// Whether this strategy supports pause/resume
    fn supports_pause(&self) -> bool {
        true
    }
}
