//! Platform-specific policy configuration

use serde::{Deserialize, Serialize};

/// Policy configuration for a platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformPolicyConfig {
    /// Human approval requirements
    pub human_approval: HumanApprovalConfig,

    /// Operation limits
    pub limits: OperationLimits,

    /// Safety holds configuration
    pub safety_holds: SafetyHoldsConfig,

    /// Accountability requirements
    pub accountability: AccountabilityConfig,
}

/// Human approval configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanApprovalConfig {
    /// Operations that always require human approval
    pub always_required: Vec<String>,

    /// Operations that require approval above threshold
    pub threshold_required: Vec<ThresholdApproval>,

    /// Default timeout for approval requests (seconds)
    pub approval_timeout_secs: u64,

    /// Allow auto-approval for low-risk operations
    pub allow_auto_approval: bool,
}

/// Threshold-based approval requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdApproval {
    /// Operation name
    pub operation: String,
    /// Type of threshold (e.g., "instance_count", "cost")
    pub threshold_type: String,
    /// Threshold value
    pub threshold_value: u64,
}

/// Operation limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLimits {
    /// Maximum concurrent deployments
    pub max_concurrent_deployments: u32,

    /// Maximum scale-up in single operation
    pub max_scale_up: u32,

    /// Maximum scale-down in single operation
    pub max_scale_down: u32,

    /// Rate limit (operations per minute)
    pub rate_limit_per_minute: u32,
}

/// Safety holds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyHoldsConfig {
    /// Enable safety holds
    pub enabled: bool,

    /// Operations blocked during safety hold
    pub blocked_operations: Vec<String>,

    /// Auto-release after duration (seconds, None = manual release only)
    pub auto_release_after_secs: Option<u64>,
}

/// Accountability requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountabilityConfig {
    /// Require proof for operations
    pub require_proof: bool,

    /// Require pre-audit entry
    pub require_pre_audit: bool,

    /// Operations that require commitment reconciliation
    pub reconciliation_required: Vec<String>,
}

impl Default for PlatformPolicyConfig {
    fn default() -> Self {
        Self {
            human_approval: HumanApprovalConfig {
                always_required: vec![],
                threshold_required: vec![],
                approval_timeout_secs: 3600,
                allow_auto_approval: true,
            },
            limits: OperationLimits {
                max_concurrent_deployments: 100,
                max_scale_up: 50,
                max_scale_down: 50,
                rate_limit_per_minute: 60,
            },
            safety_holds: SafetyHoldsConfig {
                enabled: false,
                blocked_operations: vec![],
                auto_release_after_secs: None,
            },
            accountability: AccountabilityConfig {
                require_proof: false,
                require_pre_audit: false,
                reconciliation_required: vec![],
            },
        }
    }
}

impl PlatformPolicyConfig {
    /// Create policy config for Mapleverse (high-throughput, minimal gates)
    pub fn mapleverse() -> Self {
        Self {
            human_approval: HumanApprovalConfig {
                always_required: vec![],
                threshold_required: vec![
                    ThresholdApproval {
                        operation: "scale".to_string(),
                        threshold_type: "instance_count".to_string(),
                        threshold_value: 100,
                    },
                ],
                approval_timeout_secs: 300,
                allow_auto_approval: true,
            },
            limits: OperationLimits {
                max_concurrent_deployments: 500,
                max_scale_up: 100,
                max_scale_down: 100,
                rate_limit_per_minute: 300,
            },
            safety_holds: SafetyHoldsConfig {
                enabled: false,
                blocked_operations: vec![],
                auto_release_after_secs: None,
            },
            accountability: AccountabilityConfig {
                require_proof: false,
                require_pre_audit: false,
                reconciliation_required: vec![],
            },
        }
    }

    /// Create policy config for Finalverse (safety-first)
    pub fn finalverse() -> Self {
        Self {
            human_approval: HumanApprovalConfig {
                always_required: vec![
                    "delete_deployment".to_string(),
                    "force_terminate".to_string(),
                ],
                threshold_required: vec![
                    ThresholdApproval {
                        operation: "scale".to_string(),
                        threshold_type: "instance_count".to_string(),
                        threshold_value: 10,
                    },
                    ThresholdApproval {
                        operation: "create_deployment".to_string(),
                        threshold_type: "instance_count".to_string(),
                        threshold_value: 5,
                    },
                ],
                approval_timeout_secs: 7200,
                allow_auto_approval: false,
            },
            limits: OperationLimits {
                max_concurrent_deployments: 50,
                max_scale_up: 10,
                max_scale_down: 10,
                rate_limit_per_minute: 30,
            },
            safety_holds: SafetyHoldsConfig {
                enabled: true,
                blocked_operations: vec![
                    "scale_down".to_string(),
                    "delete_deployment".to_string(),
                    "force_terminate".to_string(),
                ],
                auto_release_after_secs: Some(3600),
            },
            accountability: AccountabilityConfig {
                require_proof: true,
                require_pre_audit: true,
                reconciliation_required: vec![
                    "create_deployment".to_string(),
                    "scale".to_string(),
                    "delete_deployment".to_string(),
                ],
            },
        }
    }

    /// Create policy config for iBank (accountability-focused)
    pub fn ibank() -> Self {
        Self {
            human_approval: HumanApprovalConfig {
                always_required: vec![
                    "create_deployment".to_string(),
                    "delete_deployment".to_string(),
                    "scale".to_string(),
                ],
                threshold_required: vec![],
                approval_timeout_secs: 86400, // 24 hours
                allow_auto_approval: false,
            },
            limits: OperationLimits {
                max_concurrent_deployments: 20,
                max_scale_up: 5,
                max_scale_down: 5,
                rate_limit_per_minute: 10,
            },
            safety_holds: SafetyHoldsConfig {
                enabled: true,
                blocked_operations: vec![
                    "scale_down".to_string(),
                    "delete_deployment".to_string(),
                    "force_terminate".to_string(),
                    "rollback".to_string(),
                ],
                auto_release_after_secs: None, // Manual release only
            },
            accountability: AccountabilityConfig {
                require_proof: true,
                require_pre_audit: true,
                reconciliation_required: vec![
                    "create_deployment".to_string(),
                    "scale".to_string(),
                    "delete_deployment".to_string(),
                    "rollback".to_string(),
                    "migrate".to_string(),
                ],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy() {
        let config = PlatformPolicyConfig::default();
        assert!(config.human_approval.allow_auto_approval);
        assert!(!config.safety_holds.enabled);
    }

    #[test]
    fn test_mapleverse_policy() {
        let config = PlatformPolicyConfig::mapleverse();
        assert!(config.human_approval.allow_auto_approval);
        assert_eq!(config.limits.max_concurrent_deployments, 500);
    }

    #[test]
    fn test_finalverse_policy() {
        let config = PlatformPolicyConfig::finalverse();
        assert!(!config.human_approval.allow_auto_approval);
        assert!(config.safety_holds.enabled);
        assert!(config.accountability.require_proof);
    }

    #[test]
    fn test_ibank_policy() {
        let config = PlatformPolicyConfig::ibank();
        assert!(!config.human_approval.always_required.is_empty());
        assert!(config.safety_holds.auto_release_after_secs.is_none());
    }
}
