//! Deployment strategy implementations

pub mod blue_green;
pub mod canary;
pub mod executor;
pub mod recreate;
pub mod rolling;

pub use blue_green::BlueGreenDeploymentExecutor;
pub use canary::CanaryDeploymentExecutor;
pub use executor::{DeploymentExecutor, DeploymentResult};
pub use recreate::RecreateDeploymentExecutor;
pub use rolling::RollingDeploymentExecutor;

use palm_types::DeploymentStrategy;
use std::sync::Arc;

/// Factory for creating deployment executors
pub fn create_executor(strategy: &DeploymentStrategy) -> Arc<dyn DeploymentExecutor> {
    match strategy {
        DeploymentStrategy::Rolling {
            max_unavailable,
            max_surge,
            min_ready_seconds,
        } => Arc::new(RollingDeploymentExecutor::new(
            *max_unavailable,
            *max_surge,
            *min_ready_seconds,
        )),
        DeploymentStrategy::BlueGreen {
            switch_threshold,
            validation_period,
        } => Arc::new(BlueGreenDeploymentExecutor::new(
            *switch_threshold,
            *validation_period,
        )),
        DeploymentStrategy::Canary {
            initial_percent,
            increment_percent,
            evaluation_period,
            success_criteria,
        } => Arc::new(CanaryDeploymentExecutor::new(
            *initial_percent,
            *increment_percent,
            *evaluation_period,
            success_criteria.clone(),
        )),
        DeploymentStrategy::Recreate => Arc::new(RecreateDeploymentExecutor::new()),
    }
}
