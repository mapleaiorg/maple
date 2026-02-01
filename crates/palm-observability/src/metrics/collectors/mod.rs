//! Metric collectors for PALM components

pub mod deployment;
pub mod health;
pub mod instance;
pub mod resonance;

use prometheus::Registry;

/// All PALM metrics combined
pub struct PalmMetrics {
    /// Deployment-related metrics
    pub deployment: deployment::DeploymentMetrics,
    /// Instance-related metrics
    pub instance: instance::InstanceMetrics,
    /// Health-related metrics
    pub health: health::HealthMetrics,
    /// Resonance-specific metrics
    pub resonance: resonance::ResonanceMetrics,
}

impl PalmMetrics {
    /// Create all PALM metrics and register them
    pub fn new(registry: &Registry) -> Self {
        Self {
            deployment: deployment::DeploymentMetrics::new(registry),
            instance: instance::InstanceMetrics::new(registry),
            health: health::HealthMetrics::new(registry),
            resonance: resonance::ResonanceMetrics::new(registry),
        }
    }
}
