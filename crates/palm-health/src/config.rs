//! Health monitoring configuration.
//!
//! Defines configuration for probes, thresholds, and resilience behavior.

use std::time::Duration;

use palm_types::PlatformProfile;
use serde::{Deserialize, Serialize};

/// Configuration for health monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Probe configuration.
    pub probes: ProbeConfig,

    /// Thresholds for health assessment.
    pub thresholds: HealthThresholds,

    /// Resilience configuration.
    pub resilience: ResilienceConfig,

    /// Platform-specific overrides.
    pub platform_overrides: Option<PlatformOverrides>,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            probes: ProbeConfig::default(),
            thresholds: HealthThresholds::default(),
            resilience: ResilienceConfig::default(),
            platform_overrides: None,
        }
    }
}

impl HealthConfig {
    /// Create config optimized for a specific platform.
    pub fn for_platform(platform: PlatformProfile) -> Self {
        let mut config = Self::default();

        match platform {
            PlatformProfile::Mapleverse => {
                // Pure AI environment: optimize for throughput
                config.thresholds.presence_healthy = 0.6;
                config.thresholds.coupling_healthy = 0.5;
                config.resilience.recovery_delay = Duration::from_millis(100);
            }
            PlatformProfile::Finalverse => {
                // Human-AI interaction: prioritize safety and stability
                config.thresholds.presence_healthy = 0.9;
                config.thresholds.coupling_healthy = 0.85;
                config.thresholds.attention_healthy = 0.8;
                config.resilience.max_recovery_attempts = 5;
                config.resilience.recovery_delay = Duration::from_secs(2);
            }
            PlatformProfile::IBank => {
                // Autonomous finance: maximum accountability
                config.thresholds.presence_healthy = 0.95;
                config.thresholds.coupling_healthy = 0.9;
                config.thresholds.attention_healthy = 0.85;
                config.probes.presence_interval = Duration::from_secs(5);
                config.resilience.require_human_approval = true;
            }
            PlatformProfile::Development => {
                // Development: relaxed thresholds for testing
                config.thresholds.presence_healthy = 0.3;
                config.thresholds.coupling_healthy = 0.2;
                config.thresholds.attention_healthy = 0.2;
                config.resilience.max_recovery_attempts = 10;
            }
        }

        config
    }
}

/// Probe configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeConfig {
    /// Interval between presence probes.
    pub presence_interval: Duration,

    /// Interval between coupling probes.
    pub coupling_interval: Duration,

    /// Interval between attention probes.
    pub attention_interval: Duration,

    /// Probe timeout.
    pub probe_timeout: Duration,

    /// Number of consecutive failures before marking unhealthy.
    pub failure_threshold: u32,

    /// Number of consecutive successes before marking healthy.
    pub success_threshold: u32,

    /// Enable presence probe.
    pub enable_presence: bool,

    /// Enable coupling probe.
    pub enable_coupling: bool,

    /// Enable attention probe.
    pub enable_attention: bool,
}

impl Default for ProbeConfig {
    fn default() -> Self {
        Self {
            presence_interval: Duration::from_secs(10),
            coupling_interval: Duration::from_secs(30),
            attention_interval: Duration::from_secs(60),
            probe_timeout: Duration::from_secs(5),
            failure_threshold: 3,
            success_threshold: 2,
            enable_presence: true,
            enable_coupling: true,
            enable_attention: true,
        }
    }
}

/// Thresholds for determining health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthThresholds {
    /// Minimum presence gradient for healthy status (0.0-1.0).
    pub presence_healthy: f64,

    /// Minimum presence gradient for degraded status (below this is unhealthy).
    pub presence_degraded: f64,

    /// Minimum coupling capacity for healthy status (0.0-1.0).
    pub coupling_healthy: f64,

    /// Minimum coupling capacity for degraded status.
    pub coupling_degraded: f64,

    /// Minimum attention budget for healthy status (0.0-1.0).
    pub attention_healthy: f64,

    /// Minimum attention budget for degraded status.
    pub attention_degraded: f64,

    /// Weight for presence in overall health score.
    pub presence_weight: f64,

    /// Weight for coupling in overall health score.
    pub coupling_weight: f64,

    /// Weight for attention in overall health score.
    pub attention_weight: f64,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            presence_healthy: 0.8,
            presence_degraded: 0.5,
            coupling_healthy: 0.7,
            coupling_degraded: 0.4,
            attention_healthy: 0.6,
            attention_degraded: 0.3,
            presence_weight: 0.4,
            coupling_weight: 0.35,
            attention_weight: 0.25,
        }
    }
}

impl HealthThresholds {
    /// Calculate overall health score from individual dimensions.
    pub fn calculate_overall_score(&self, presence: f64, coupling: f64, attention: f64) -> f64 {
        let total_weight = self.presence_weight + self.coupling_weight + self.attention_weight;

        (presence * self.presence_weight
            + coupling * self.coupling_weight
            + attention * self.attention_weight)
            / total_weight
    }

    /// Determine if the given scores indicate healthy status.
    pub fn is_healthy(&self, presence: f64, coupling: f64, attention: f64) -> bool {
        presence >= self.presence_healthy
            && coupling >= self.coupling_healthy
            && attention >= self.attention_healthy
    }

    /// Determine if the given scores indicate degraded status.
    pub fn is_degraded(&self, presence: f64, coupling: f64, attention: f64) -> bool {
        !self.is_healthy(presence, coupling, attention)
            && presence >= self.presence_degraded
            && coupling >= self.coupling_degraded
            && attention >= self.attention_degraded
    }
}

/// Resilience configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResilienceConfig {
    /// Maximum number of recovery attempts before giving up.
    pub max_recovery_attempts: u32,

    /// Delay between recovery attempts.
    pub recovery_delay: Duration,

    /// Enable automatic recovery actions.
    pub auto_recovery_enabled: bool,

    /// Require human approval for destructive actions.
    pub require_human_approval: bool,

    /// Circuit breaker configuration.
    pub circuit_breaker: CircuitBreakerConfig,

    /// Enable isolation of unhealthy instances.
    pub enable_isolation: bool,

    /// Drain timeout for graceful shutdown.
    pub drain_timeout: Duration,
}

impl Default for ResilienceConfig {
    fn default() -> Self {
        Self {
            max_recovery_attempts: 3,
            recovery_delay: Duration::from_secs(5),
            auto_recovery_enabled: true,
            require_human_approval: false,
            circuit_breaker: CircuitBreakerConfig::default(),
            enable_isolation: true,
            drain_timeout: Duration::from_secs(30),
        }
    }
}

/// Circuit breaker configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of failures to open the circuit.
    pub failure_threshold: u32,

    /// Number of successes in half-open to close the circuit.
    pub success_threshold: u32,

    /// Time to wait before transitioning from open to half-open.
    pub reset_timeout: Duration,

    /// Maximum requests allowed in half-open state.
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            reset_timeout: Duration::from_secs(30),
            half_open_max_requests: 3,
        }
    }
}

/// Platform-specific configuration overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformOverrides {
    /// Platform profile these overrides apply to.
    pub platform: PlatformProfile,

    /// Override probe configuration.
    pub probes: Option<ProbeConfig>,

    /// Override thresholds.
    pub thresholds: Option<HealthThresholds>,

    /// Override resilience configuration.
    pub resilience: Option<ResilienceConfig>,
}
