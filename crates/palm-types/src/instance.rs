//! Instance types for individual agent instances
//!
//! An AgentInstance is a running instance of an AgentSpec managed by a Deployment.

use crate::{DeploymentId, InstanceId, NodeId};
use serde::{Deserialize, Serialize};

/// A running agent instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInstance {
    /// Unique instance identifier
    pub id: InstanceId,

    /// Parent deployment
    pub deployment_id: DeploymentId,

    /// Underlying resonator ID (from maple-runtime)
    pub resonator_id: ResonatorIdRef,

    /// Current instance status
    pub status: InstanceStatus,

    /// Current health status
    pub health: HealthStatus,

    /// Placement information
    pub placement: InstancePlacement,

    /// Instance metrics
    pub metrics: InstanceMetrics,

    /// Start timestamp
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// Last heartbeat timestamp
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

/// Reference to a resonator ID (avoiding direct dependency on maple-runtime)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResonatorIdRef(String);

impl ResonatorIdRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ResonatorIdRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Instance lifecycle status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstanceStatus {
    /// Instance is starting up
    Starting {
        phase: StartupPhase,
    },

    /// Instance is running normally
    Running,

    /// Instance is draining (preparing for termination)
    Draining {
        reason: DrainReason,
    },

    /// Instance is being terminated
    Terminating {
        reason: TerminationReason,
    },

    /// Instance has terminated
    Terminated {
        exit_code: Option<i32>,
    },

    /// Instance is in error state
    Error {
        message: String,
    },
}

/// Startup phases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StartupPhase {
    /// Initializing resources
    Initializing,
    /// Establishing presence
    EstablishingPresence,
    /// Waiting for readiness probe
    WaitingForReadiness,
}

/// Reasons for draining
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DrainReason {
    /// Deployment update
    Deployment,
    /// Manual scale down
    ScaleDown,
    /// Node maintenance
    NodeMaintenance,
    /// Health failure
    HealthFailure,
}

/// Reasons for termination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminationReason {
    /// Deployment update/rollout
    Deployment,
    /// Scale down operation
    ScaleDown,
    /// Health check failure
    HealthFailure,
    /// Manual termination
    Manual,
    /// Resource exhaustion
    ResourceExhaustion,
    /// Policy violation
    PolicyViolation,
}

/// Health status (multi-dimensional, not binary)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Health unknown (not yet probed)
    Unknown,

    /// Instance is healthy
    Healthy,

    /// Instance is degraded but functional
    Degraded {
        /// Degradation factors
        factors: Vec<String>,
    },

    /// Instance is unhealthy
    Unhealthy {
        /// Failure reasons
        reasons: Vec<String>,
    },
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    pub fn is_operational(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded { .. })
    }
}

impl Default for HealthStatus {
    fn default() -> Self {
        HealthStatus::Unknown
    }
}

/// Instance placement information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstancePlacement {
    /// Node this instance is running on
    pub node_id: Option<NodeId>,

    /// Availability zone
    pub zone: Option<String>,

    /// Region
    pub region: Option<String>,
}

/// Instance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceMetrics {
    /// Attention utilization (0.0 to 1.0)
    pub attention_utilization: f64,

    /// Active coupling count
    pub active_couplings: u32,

    /// Requests processed
    pub requests_processed: u64,

    /// Error count
    pub error_count: u64,

    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
}
