//! PALM Types - Core types for fleet orchestration
//!
//! PALM (Persistent Agent Lifecycle Manager) is the fleet orchestration layer
//! for MAPLE's Resonance Architecture. It manages deployment, scaling, and
//! health monitoring of Resonator fleets.
//!
//! ## Architectural Boundaries
//!
//! - **PALM** owns: Fleet-level orchestration, deployment strategies, discovery
//! - **Resonator Runtime** owns: Individual Resonator lifecycle, state, execution
//! - **maple-runtime** owns: Coupling, attention, presence for single instances
//!
//! ## Key Concepts
//!
//! - **AgentSpec**: Template defining what to deploy
//! - **Deployment**: Manages a fleet of instances based on a spec
//! - **AgentInstance**: A single running instance of a spec
//! - **Health**: Multi-dimensional health assessment (not binary)
//! - **Policy**: Governance gates for deployment operations
//! - **Events**: Unified observability stream

#![deny(unsafe_code)]
#![cfg_attr(feature = "strict-docs", warn(missing_docs))]
#![cfg_attr(not(feature = "strict-docs"), allow(missing_docs))]

pub mod ids;
pub mod platform;
pub mod spec;
pub mod deployment;
pub mod instance;
pub mod health;
pub mod policy;
pub mod events;

// Re-export main types
pub use ids::{AgentSpecId, DeploymentId, InstanceId, NodeId};
pub use platform::PlatformProfile;
pub use spec::{
    AgentSpec, AutonomyLevel, CapabilityRef, HealthConfig, ProbeConfig, ProbeType,
    ResonatorProfileConfig, ResourceRequirements, RiskTolerance, SpecValidationError,
};
pub use deployment::{
    CanarySuccessCriteria, Deployment, DeploymentStatus, DeploymentStrategy, ReplicaConfig,
};
pub use instance::{
    AgentInstance, DrainReason, HealthStatus, InstanceMetrics, InstancePlacement, InstanceStatus,
    ResonatorIdRef, StartupPhase, TerminationReason,
};
pub use health::{
    AlertCategory, AlertSeverity, HealthAlert, HealthAssessment, HealthDimensions, ProbeDetails,
    ProbeResult,
};
pub use policy::{
    OperationType, PalmOperation, PolicyAction, PolicyCondition, PolicyContext, PolicyDecision,
    PolicyError, PolicyRule,
};
pub use events::{EventSeverity, EventSource, PalmEvent, PalmEventEnvelope};
