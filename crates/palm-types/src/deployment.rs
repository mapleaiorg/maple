//! Deployment types for fleet orchestration
//!
//! A Deployment manages a fleet of agent instances based on an AgentSpec.

use crate::{AgentSpecId, DeploymentId, PlatformProfile};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A deployment manages the lifecycle of agent instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    /// Unique deployment identifier
    pub id: DeploymentId,

    /// Agent spec this deployment is based on
    pub agent_spec_id: AgentSpecId,

    /// Current version being deployed
    pub version: semver::Version,

    /// Target platform
    pub platform: PlatformProfile,

    /// Deployment strategy
    pub strategy: DeploymentStrategy,

    /// Current deployment status
    pub status: DeploymentStatus,

    /// Replica configuration
    pub replicas: ReplicaConfig,

    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Deployment strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStrategy {
    /// Rolling update - gradual replacement
    Rolling {
        /// Maximum instances unavailable during update
        max_unavailable: u32,
        /// Maximum extra instances during update
        max_surge: u32,
        /// Minimum seconds instance must be ready
        min_ready_seconds: u32,
    },

    /// Blue-Green - full parallel deployment then switch
    BlueGreen {
        /// Health threshold to switch (0.0 to 1.0)
        switch_threshold: f64,
        /// Validation period before switching
        #[serde(with = "duration_serde")]
        validation_period: Duration,
    },

    /// Canary - gradual traffic shift with evaluation
    Canary {
        /// Initial percentage of traffic to canary
        initial_percent: u32,
        /// Percentage increment per evaluation
        increment_percent: u32,
        /// Time between evaluations
        #[serde(with = "duration_serde")]
        evaluation_period: Duration,
        /// Success criteria for canary
        success_criteria: CanarySuccessCriteria,
    },

    /// Recreate - terminate all, then create new
    Recreate,
}

impl Default for DeploymentStrategy {
    fn default() -> Self {
        DeploymentStrategy::Rolling {
            max_unavailable: 1,
            max_surge: 1,
            min_ready_seconds: 30,
        }
    }
}

/// Success criteria for canary deployments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanarySuccessCriteria {
    /// Maximum error rate (0.0 to 1.0)
    pub max_error_rate: f64,

    /// Maximum P99 latency in milliseconds
    pub max_latency_p99_ms: u64,

    /// Minimum success rate (0.0 to 1.0)
    pub min_success_rate: f64,
}

impl Default for CanarySuccessCriteria {
    fn default() -> Self {
        Self {
            max_error_rate: 0.05,
            max_latency_p99_ms: 500,
            min_success_rate: 0.95,
        }
    }
}

/// Deployment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStatus {
    /// Waiting to be processed
    Pending,

    /// Currently deploying
    InProgress {
        /// Percentage complete (0-100)
        progress: u32,
        /// Current phase description
        phase: String,
    },

    /// Deployment paused
    Paused {
        /// Reason for pause
        reason: String,
        /// Paused at timestamp
        paused_at: chrono::DateTime<chrono::Utc>,
    },

    /// Deployment completed successfully
    Completed {
        /// Completion timestamp
        completed_at: chrono::DateTime<chrono::Utc>,
    },

    /// Deployment failed
    Failed {
        /// Failure reason
        reason: String,
        /// Failed at timestamp
        failed_at: chrono::DateTime<chrono::Utc>,
    },

    /// Rolling back
    RollingBack {
        /// Target version for rollback
        target_version: semver::Version,
    },
}

/// Replica configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaConfig {
    /// Desired number of replicas
    pub desired: u32,

    /// Minimum replicas (for autoscaling)
    pub min: u32,

    /// Maximum replicas (for autoscaling)
    pub max: u32,

    /// Current healthy replicas (read-only, updated by system)
    pub current_healthy: u32,

    /// Current total replicas (read-only, updated by system)
    pub current_total: u32,
}

impl Default for ReplicaConfig {
    fn default() -> Self {
        Self {
            desired: 3,
            min: 1,
            max: 10,
            current_healthy: 0,
            current_total: 0,
        }
    }
}

impl ReplicaConfig {
    pub fn new(desired: u32) -> Self {
        Self {
            desired,
            min: 1,
            max: desired * 3,
            current_healthy: 0,
            current_total: 0,
        }
    }
}

/// Serde helper for Duration
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}
