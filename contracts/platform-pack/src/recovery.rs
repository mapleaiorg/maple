//! Platform-specific recovery configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Recovery configuration for a platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformRecoveryConfig {
    /// Automatic recovery settings
    pub auto_recovery: AutoRecoveryConfig,

    /// Escalation configuration
    pub escalation: EscalationConfig,

    /// Circuit breaker settings
    pub circuit_breaker: CircuitBreakerConfig,

    /// Graceful degradation settings
    pub degradation: DegradationConfig,
}

/// Automatic recovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoRecoveryConfig {
    /// Enable automatic recovery
    pub enabled: bool,

    /// Maximum recovery attempts
    pub max_attempts: u32,

    /// Backoff configuration
    pub backoff: BackoffConfig,

    /// Actions to attempt for recovery
    pub recovery_actions: Vec<RecoveryAction>,

    /// Timeout for recovery attempts (seconds)
    pub timeout_secs: u64,
}

/// Backoff configuration for retries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackoffConfig {
    /// Initial backoff delay (milliseconds)
    pub initial_delay_ms: u64,

    /// Maximum backoff delay (milliseconds)
    pub max_delay_ms: u64,

    /// Backoff multiplier
    pub multiplier: f64,

    /// Add jitter to delays
    pub jitter: bool,
}

/// Recovery action to attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    /// Action name
    pub name: String,

    /// Action type
    pub action_type: RecoveryActionType,

    /// Order in recovery sequence (lower = earlier)
    pub order: u32,

    /// Conditions for this action
    pub conditions: Vec<String>,

    /// Additional parameters
    pub params: HashMap<String, serde_json::Value>,
}

/// Recovery action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryActionType {
    /// Restart the instance
    Restart,
    /// Restore from checkpoint
    RestoreCheckpoint,
    /// Failover to replica
    Failover,
    /// Scale up to handle load
    ScaleUp,
    /// Migrate to different node
    Migrate,
    /// Rollback to previous version
    Rollback,
    /// Notify operator
    Notify,
    /// Custom recovery action
    Custom(String),
}

/// Escalation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationConfig {
    /// Enable escalation
    pub enabled: bool,

    /// Escalation levels
    pub levels: Vec<EscalationLevel>,

    /// Notification channels
    pub channels: Vec<NotificationChannel>,
}

/// Escalation level definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationLevel {
    /// Level name
    pub name: String,

    /// Level number (1 = lowest)
    pub level: u32,

    /// Trigger conditions
    pub trigger_after_failures: u32,

    /// Trigger after duration (seconds)
    pub trigger_after_secs: u64,

    /// Actions at this level
    pub actions: Vec<String>,

    /// Notification recipients
    pub recipients: Vec<String>,
}

/// Notification channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    /// Channel name
    pub name: String,

    /// Channel type
    pub channel_type: ChannelType,

    /// Channel endpoint
    pub endpoint: String,

    /// Severity filter (minimum severity to notify)
    pub min_severity: Severity,
}

/// Notification channel type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Email,
    Slack,
    Webhook,
    PagerDuty,
    Custom(String),
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Warning
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,

    /// Failure threshold to trip circuit
    pub failure_threshold: u32,

    /// Success threshold to close circuit
    pub success_threshold: u32,

    /// Window size for failure counting (seconds)
    pub window_secs: u64,

    /// Half-open timeout (seconds)
    pub half_open_timeout_secs: u64,

    /// Maximum half-open requests
    pub half_open_max_requests: u32,
}

/// Graceful degradation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationConfig {
    /// Enable graceful degradation
    pub enabled: bool,

    /// Degradation levels
    pub levels: Vec<DegradationLevel>,

    /// Features that can be disabled
    pub degradable_features: Vec<DegradableFeature>,
}

/// Degradation level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationLevel {
    /// Level name
    pub name: String,

    /// Resource utilization threshold to trigger (0-1)
    pub threshold: f64,

    /// Features to disable at this level
    pub disable_features: Vec<String>,

    /// Reduce capacity to percentage
    pub capacity_percentage: u32,
}

/// Feature that can be degraded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradableFeature {
    /// Feature name
    pub name: String,

    /// Priority (lower = disable first)
    pub priority: u32,

    /// Estimated resource savings (0-1)
    pub resource_savings: f64,
}

impl Default for PlatformRecoveryConfig {
    fn default() -> Self {
        Self {
            auto_recovery: AutoRecoveryConfig {
                enabled: true,
                max_attempts: 3,
                backoff: BackoffConfig {
                    initial_delay_ms: 1000,
                    max_delay_ms: 30000,
                    multiplier: 2.0,
                    jitter: true,
                },
                recovery_actions: vec![
                    RecoveryAction {
                        name: "restart".to_string(),
                        action_type: RecoveryActionType::Restart,
                        order: 1,
                        conditions: vec![],
                        params: HashMap::new(),
                    },
                ],
                timeout_secs: 300,
            },
            escalation: EscalationConfig {
                enabled: false,
                levels: vec![],
                channels: vec![],
            },
            circuit_breaker: CircuitBreakerConfig {
                enabled: true,
                failure_threshold: 5,
                success_threshold: 2,
                window_secs: 60,
                half_open_timeout_secs: 30,
                half_open_max_requests: 3,
            },
            degradation: DegradationConfig {
                enabled: false,
                levels: vec![],
                degradable_features: vec![],
            },
        }
    }
}

impl PlatformRecoveryConfig {
    /// Create recovery config for Mapleverse (aggressive, fast recovery)
    pub fn mapleverse() -> Self {
        Self {
            auto_recovery: AutoRecoveryConfig {
                enabled: true,
                max_attempts: 5,
                backoff: BackoffConfig {
                    initial_delay_ms: 500,
                    max_delay_ms: 10000,
                    multiplier: 1.5,
                    jitter: true,
                },
                recovery_actions: vec![
                    RecoveryAction {
                        name: "restart".to_string(),
                        action_type: RecoveryActionType::Restart,
                        order: 1,
                        conditions: vec![],
                        params: HashMap::new(),
                    },
                    RecoveryAction {
                        name: "migrate".to_string(),
                        action_type: RecoveryActionType::Migrate,
                        order: 2,
                        conditions: vec!["node_unhealthy".to_string()],
                        params: HashMap::new(),
                    },
                    RecoveryAction {
                        name: "scale_up".to_string(),
                        action_type: RecoveryActionType::ScaleUp,
                        order: 3,
                        conditions: vec!["overloaded".to_string()],
                        params: HashMap::new(),
                    },
                ],
                timeout_secs: 120,
            },
            escalation: EscalationConfig {
                enabled: true,
                levels: vec![
                    EscalationLevel {
                        name: "L1".to_string(),
                        level: 1,
                        trigger_after_failures: 3,
                        trigger_after_secs: 300,
                        actions: vec!["notify".to_string()],
                        recipients: vec!["oncall".to_string()],
                    },
                ],
                channels: vec![],
            },
            circuit_breaker: CircuitBreakerConfig {
                enabled: true,
                failure_threshold: 10,
                success_threshold: 3,
                window_secs: 30,
                half_open_timeout_secs: 15,
                half_open_max_requests: 5,
            },
            degradation: DegradationConfig {
                enabled: true,
                levels: vec![
                    DegradationLevel {
                        name: "level1".to_string(),
                        threshold: 0.8,
                        disable_features: vec!["analytics".to_string()],
                        capacity_percentage: 90,
                    },
                    DegradationLevel {
                        name: "level2".to_string(),
                        threshold: 0.9,
                        disable_features: vec!["analytics".to_string(), "non_critical_features".to_string()],
                        capacity_percentage: 70,
                    },
                ],
                degradable_features: vec![
                    DegradableFeature {
                        name: "analytics".to_string(),
                        priority: 1,
                        resource_savings: 0.1,
                    },
                ],
            },
        }
    }

    /// Create recovery config for Finalverse (careful, deliberate recovery)
    pub fn finalverse() -> Self {
        Self {
            auto_recovery: AutoRecoveryConfig {
                enabled: true,
                max_attempts: 3,
                backoff: BackoffConfig {
                    initial_delay_ms: 2000,
                    max_delay_ms: 60000,
                    multiplier: 2.0,
                    jitter: true,
                },
                recovery_actions: vec![
                    RecoveryAction {
                        name: "restore_checkpoint".to_string(),
                        action_type: RecoveryActionType::RestoreCheckpoint,
                        order: 1,
                        conditions: vec![],
                        params: HashMap::new(),
                    },
                    RecoveryAction {
                        name: "restart".to_string(),
                        action_type: RecoveryActionType::Restart,
                        order: 2,
                        conditions: vec!["checkpoint_unavailable".to_string()],
                        params: HashMap::new(),
                    },
                    RecoveryAction {
                        name: "notify".to_string(),
                        action_type: RecoveryActionType::Notify,
                        order: 3,
                        conditions: vec![],
                        params: HashMap::new(),
                    },
                ],
                timeout_secs: 600,
            },
            escalation: EscalationConfig {
                enabled: true,
                levels: vec![
                    EscalationLevel {
                        name: "L1".to_string(),
                        level: 1,
                        trigger_after_failures: 2,
                        trigger_after_secs: 180,
                        actions: vec!["notify".to_string()],
                        recipients: vec!["team".to_string()],
                    },
                    EscalationLevel {
                        name: "L2".to_string(),
                        level: 2,
                        trigger_after_failures: 3,
                        trigger_after_secs: 600,
                        actions: vec!["notify".to_string(), "page".to_string()],
                        recipients: vec!["oncall".to_string(), "manager".to_string()],
                    },
                ],
                channels: vec![],
            },
            circuit_breaker: CircuitBreakerConfig {
                enabled: true,
                failure_threshold: 3,
                success_threshold: 3,
                window_secs: 120,
                half_open_timeout_secs: 60,
                half_open_max_requests: 2,
            },
            degradation: DegradationConfig {
                enabled: false, // Disabled for safety
                levels: vec![],
                degradable_features: vec![],
            },
        }
    }

    /// Create recovery config for iBank (conservative, human-involved recovery)
    pub fn ibank() -> Self {
        Self {
            auto_recovery: AutoRecoveryConfig {
                enabled: false, // Manual recovery required
                max_attempts: 1,
                backoff: BackoffConfig {
                    initial_delay_ms: 5000,
                    max_delay_ms: 60000,
                    multiplier: 2.0,
                    jitter: false,
                },
                recovery_actions: vec![
                    RecoveryAction {
                        name: "notify".to_string(),
                        action_type: RecoveryActionType::Notify,
                        order: 1,
                        conditions: vec![],
                        params: HashMap::new(),
                    },
                ],
                timeout_secs: 900,
            },
            escalation: EscalationConfig {
                enabled: true,
                levels: vec![
                    EscalationLevel {
                        name: "L1".to_string(),
                        level: 1,
                        trigger_after_failures: 1,
                        trigger_after_secs: 60,
                        actions: vec!["notify".to_string(), "create_ticket".to_string()],
                        recipients: vec!["operations".to_string()],
                    },
                    EscalationLevel {
                        name: "L2".to_string(),
                        level: 2,
                        trigger_after_failures: 1,
                        trigger_after_secs: 300,
                        actions: vec!["page".to_string()],
                        recipients: vec!["oncall".to_string(), "compliance".to_string()],
                    },
                    EscalationLevel {
                        name: "L3".to_string(),
                        level: 3,
                        trigger_after_failures: 1,
                        trigger_after_secs: 900,
                        actions: vec!["page".to_string(), "executive_notify".to_string()],
                        recipients: vec!["senior_management".to_string()],
                    },
                ],
                channels: vec![],
            },
            circuit_breaker: CircuitBreakerConfig {
                enabled: true,
                failure_threshold: 2,
                success_threshold: 5,
                window_secs: 300,
                half_open_timeout_secs: 120,
                half_open_max_requests: 1,
            },
            degradation: DegradationConfig {
                enabled: false, // Not allowed in banking
                levels: vec![],
                degradable_features: vec![],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_recovery() {
        let config = PlatformRecoveryConfig::default();
        assert!(config.auto_recovery.enabled);
        assert!(config.circuit_breaker.enabled);
    }

    #[test]
    fn test_mapleverse_recovery() {
        let config = PlatformRecoveryConfig::mapleverse();
        assert_eq!(config.auto_recovery.max_attempts, 5);
        assert!(config.degradation.enabled);
    }

    #[test]
    fn test_finalverse_recovery() {
        let config = PlatformRecoveryConfig::finalverse();
        assert!(!config.degradation.enabled);
        assert_eq!(config.escalation.levels.len(), 2);
    }

    #[test]
    fn test_ibank_recovery() {
        let config = PlatformRecoveryConfig::ibank();
        assert!(!config.auto_recovery.enabled);
        assert_eq!(config.escalation.levels.len(), 3);
        assert!(!config.degradation.enabled);
    }
}
