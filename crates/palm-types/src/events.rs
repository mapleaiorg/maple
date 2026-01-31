//! Event types for PALM observability
//!
//! Events provide a unified stream of deployment lifecycle activities.

use crate::{AgentSpecId, DeploymentId, InstanceId, PlatformProfile};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Envelope wrapping all PALM events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PalmEventEnvelope {
    /// Unique event ID
    pub id: Uuid,

    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Event source
    pub source: EventSource,

    /// Event severity
    pub severity: EventSeverity,

    /// Platform context
    pub platform: PlatformProfile,

    /// Correlation ID for tracing
    pub correlation_id: Option<String>,

    /// Actor who triggered the event
    pub actor: Option<String>,

    /// The actual event
    pub event: PalmEvent,
}

/// Event sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSource {
    /// Deployment manager
    Deployment,
    /// Instance lifecycle
    Instance,
    /// Health monitoring
    Health,
    /// Scheduler
    Scheduler,
    /// Policy engine
    Policy,
    /// Discovery/routing
    Discovery,
    /// Control plane
    ControlPlane,
    /// State management
    State,
    /// Registry
    Registry,
}

/// Event severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSeverity {
    /// Debug-level event
    Debug,
    /// Informational event
    Info,
    /// Warning event
    Warning,
    /// Error event
    Error,
    /// Critical event requiring immediate attention
    Critical,
}

/// PALM events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PalmEvent {
    // ═══════════════════════════════════════════════════════════════════
    // DEPLOYMENT EVENTS
    // ═══════════════════════════════════════════════════════════════════
    /// Deployment created
    DeploymentCreated {
        deployment_id: DeploymentId,
        spec_id: AgentSpecId,
    },

    /// Deployment started
    DeploymentStarted {
        deployment_id: DeploymentId,
    },

    /// Deployment progress updated
    DeploymentProgress {
        deployment_id: DeploymentId,
        progress: u32,
        phase: String,
    },

    /// Deployment completed
    DeploymentCompleted {
        deployment_id: DeploymentId,
        duration_seconds: u64,
    },

    /// Deployment failed
    DeploymentFailed {
        deployment_id: DeploymentId,
        reason: String,
    },

    /// Deployment paused
    DeploymentPaused {
        deployment_id: DeploymentId,
    },

    /// Deployment resumed
    DeploymentResumed {
        deployment_id: DeploymentId,
    },

    /// Deployment rolled back
    DeploymentRolledBack {
        deployment_id: DeploymentId,
        to_version: semver::Version,
    },

    /// Deployment scaled
    DeploymentScaled {
        deployment_id: DeploymentId,
        from_replicas: u32,
        to_replicas: u32,
    },

    // ═══════════════════════════════════════════════════════════════════
    // INSTANCE EVENTS
    // ═══════════════════════════════════════════════════════════════════
    /// Instance created
    InstanceCreated {
        instance_id: InstanceId,
        deployment_id: DeploymentId,
    },

    /// Instance started
    InstanceStarted {
        instance_id: InstanceId,
    },

    /// Instance ready (passed readiness probe)
    InstanceReady {
        instance_id: InstanceId,
    },

    /// Instance health changed
    InstanceHealthChanged {
        instance_id: InstanceId,
        old_status: String,
        new_status: String,
    },

    /// Instance draining
    InstanceDraining {
        instance_id: InstanceId,
    },

    /// Instance terminated
    InstanceTerminated {
        instance_id: InstanceId,
        exit_code: Option<i32>,
    },

    /// Instance restarted
    InstanceRestarted {
        instance_id: InstanceId,
        graceful: bool,
    },

    // ═══════════════════════════════════════════════════════════════════
    // REGISTRY EVENTS
    // ═══════════════════════════════════════════════════════════════════
    /// Agent spec registered
    SpecRegistered {
        spec_id: AgentSpecId,
    },

    /// Agent spec deprecated
    SpecDeprecated {
        spec_id: AgentSpecId,
    },

    // ═══════════════════════════════════════════════════════════════════
    // RECOVERY EVENTS
    // ═══════════════════════════════════════════════════════════════════
    /// Recovery initiated for an instance
    RecoveryInitiated {
        instance_id: InstanceId,
    },

    // ═══════════════════════════════════════════════════════════════════
    // HEALTH EVENTS
    // ═══════════════════════════════════════════════════════════════════
    /// Health probe succeeded
    HealthProbeSuccess {
        instance_id: InstanceId,
        probe_type: String,
    },

    /// Health probe failed
    HealthProbeFailed {
        instance_id: InstanceId,
        probe_type: String,
        reason: String,
    },

    /// Health threshold breached
    HealthThresholdBreached {
        deployment_id: DeploymentId,
        healthy_ratio: f64,
        required_ratio: f64,
    },

    // ═══════════════════════════════════════════════════════════════════
    // POLICY EVENTS
    // ═══════════════════════════════════════════════════════════════════
    /// Policy check passed
    PolicyPassed {
        operation: String,
    },

    /// Policy check denied
    PolicyDenied {
        operation: String,
        reason: String,
    },

    /// Policy approval required
    PolicyApprovalRequired {
        operation: String,
        approvers: Vec<String>,
    },

    // ═══════════════════════════════════════════════════════════════════
    // DISCOVERY EVENTS
    // ═══════════════════════════════════════════════════════════════════
    /// Instance registered in discovery
    DiscoveryRegistered {
        instance_id: InstanceId,
    },

    /// Instance removed from discovery
    DiscoveryRemoved {
        instance_id: InstanceId,
    },

    /// Traffic split updated
    TrafficSplitUpdated {
        deployment_id: DeploymentId,
        new_percentage: u32,
    },
}

impl PalmEventEnvelope {
    /// Create a new event envelope
    pub fn new(event: PalmEvent, source: EventSource, platform: PlatformProfile) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source,
            severity: Self::infer_severity(&event),
            platform,
            correlation_id: None,
            actor: None,
            event,
        }
    }

    /// Create with correlation ID
    pub fn with_correlation(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    /// Create with actor
    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    /// Infer severity from event type
    fn infer_severity(event: &PalmEvent) -> EventSeverity {
        match event {
            PalmEvent::DeploymentFailed { .. }
            | PalmEvent::HealthThresholdBreached { .. }
            | PalmEvent::PolicyDenied { .. } => EventSeverity::Error,

            PalmEvent::DeploymentPaused { .. }
            | PalmEvent::HealthProbeFailed { .. }
            | PalmEvent::InstanceDraining { .. } => EventSeverity::Warning,

            PalmEvent::DeploymentCompleted { .. }
            | PalmEvent::InstanceReady { .. }
            | PalmEvent::PolicyPassed { .. } => EventSeverity::Info,

            _ => EventSeverity::Info,
        }
    }
}
