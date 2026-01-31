//! # PALM Health - Health Monitoring and Resilience for Resonator Fleets
//!
//! This crate provides multi-dimensional health monitoring and resilience
//! capabilities for PALM (Persistent Agent Lifecycle Manager).
//!
//! ## Overview
//!
//! PALM Health implements Resonance-native health concepts:
//!
//! - **Presence Gradient**: Is the agent "present" in the resonance field?
//! - **Coupling Capacity**: Can the agent couple with other agents?
//! - **Attention Budget**: Does the agent have attention remaining?
//!
//! These are NOT traditional liveness/readiness probes - they measure the
//! agent's participation in the shared reality of the MAPLE framework.
//!
//! ## Key Components
//!
//! - [`HealthMonitor`]: Continuous fleet health monitoring
//! - [`HealthAssessment`]: Multi-dimensional health assessment
//! - [`probes`]: Health probe implementations
//! - [`resilience`]: Circuit breaker and recovery actions
//!
//! ## Example
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use palm_health::{
//!     HealthMonitor, HealthConfig,
//!     resilience::{ResilienceController, NoOpRecoveryExecutor},
//! };
//! use palm_types::{InstanceId, PlatformProfile};
//!
//! # async fn example() {
//! // Create configuration
//! let config = HealthConfig::for_platform(PlatformProfile::Development);
//!
//! // Create resilience controller
//! let executor = Arc::new(NoOpRecoveryExecutor);
//! let resilience = Arc::new(ResilienceController::new(
//!     config.resilience.clone(),
//!     PlatformProfile::Development,
//!     executor,
//! ));
//!
//! // Create health monitor
//! let monitor = HealthMonitor::new(config, resilience);
//!
//! // Register an instance
//! let instance_id = InstanceId::generate();
//! monitor.register_instance(instance_id.clone()).unwrap();
//!
//! // Probe the instance
//! let assessment = monitor.probe_instance(&instance_id).await.unwrap();
//! println!("Health: {:?}", assessment.overall);
//! # }
//! ```
//!
//! ## Platform-Specific Behavior
//!
//! Health monitoring adapts to the platform profile:
//!
//! - **Mapleverse**: Optimized for throughput (pure AI environment)
//! - **Finalverse**: Prioritizes safety (human-AI interaction)
//! - **IBank**: Maximum accountability (autonomous finance)
//! - **Development**: Relaxed thresholds for testing
//!
//! ## Resilience Patterns
//!
//! The crate implements several resilience patterns:
//!
//! - **Circuit Breaker**: Prevents cascading failures
//! - **Recovery Actions**: Graduated response from restart to replace
//! - **Isolation**: Remove unhealthy instances from discovery
//!
//! ## Integration Points
//!
//! PALM Health integrates with:
//!
//! - `palm-types`: Core type definitions
//! - `palm-registry`: Service discovery (for isolation)
//! - `palm-deployment`: Deployment orchestration (for recovery)
//! - `resonator-runtime`: Individual Resonator health (future)

pub mod assessment;
pub mod config;
pub mod error;
pub mod monitor;
pub mod probes;
pub mod resilience;

// Re-export main types
pub use assessment::{
    DimensionAssessment, DimensionHealth, FleetHealthSummary, HealthAssessment, OverallHealth,
};
pub use config::{
    CircuitBreakerConfig, HealthConfig, HealthThresholds, ProbeConfig, ResilienceConfig,
};
pub use error::{HealthError, HealthResult};
pub use monitor::{HealthEvent, HealthMonitor};
pub use probes::{Probe, ProbeResult, ProbeSet, ProbeType};
pub use resilience::{
    CircuitBreaker, CircuitState, RecoveryAction, RecoveryContext, RecoveryOutcome,
    ResilienceController,
};

#[cfg(test)]
mod tests {
    use super::*;
    use palm_types::{InstanceId, PlatformProfile};
    use std::sync::Arc;

    #[test]
    fn test_platform_config() {
        let dev_config = HealthConfig::for_platform(PlatformProfile::Development);
        let ibank_config = HealthConfig::for_platform(PlatformProfile::IBank);

        // Development should have lower thresholds
        assert!(
            dev_config.thresholds.presence_healthy < ibank_config.thresholds.presence_healthy
        );

        // IBank should require human approval
        assert!(ibank_config.resilience.require_human_approval);
    }

    #[tokio::test]
    async fn test_health_monitor_integration() {
        let config = HealthConfig::for_platform(PlatformProfile::Development);
        let executor = Arc::new(resilience::NoOpRecoveryExecutor);
        let resilience = Arc::new(ResilienceController::new(
            config.resilience.clone(),
            PlatformProfile::Development,
            executor,
        ));

        let monitor = HealthMonitor::new(config, resilience);

        // Register and probe
        let instance_id = InstanceId::generate();
        monitor.register_instance(instance_id.clone()).unwrap();

        let assessment = monitor.probe_instance(&instance_id).await.unwrap();
        assert_eq!(assessment.instance_id, instance_id);

        // Get fleet summary
        let summary = monitor.get_fleet_summary();
        assert_eq!(summary.total_instances, 1);
    }
}
