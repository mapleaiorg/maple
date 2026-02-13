//! Self-observation event types.
//!
//! All self-observation flows through a unified event system. These events
//! are INTERNAL -- they feed the meaning formation engine, not external APIs.

use std::time::Duration;

use chrono::{DateTime, Utc};
use maple_kernel_fabric::ResonanceStage;
use maple_mwl_types::{CommitmentId, EventId, WorldlineId};
use serde::{Deserialize, Serialize};

/// Identifies which kernel subsystem produced an observation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubsystemId {
    /// Event Fabric (event emission, WAL, routing)
    EventFabric,
    /// Commitment Gate (7-stage pipeline)
    CommitmentGate,
    /// Two-Plane Memory Engine
    MemoryEngine,
    /// Message Routing Protocol
    MrpRouter,
    /// Provenance Index (causal DAG)
    ProvenanceIndex,
    /// Governance Engine (AAS, policies, invariants)
    GovernanceEngine,
    /// Safety Suite (consent, coercion, boundaries)
    SafetySuite,
    /// Performance Profiler (self-monitoring)
    Profiler,
    /// Custom subsystem extension
    Custom(String),
}

impl std::fmt::Display for SubsystemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventFabric => write!(f, "event-fabric"),
            Self::CommitmentGate => write!(f, "commitment-gate"),
            Self::MemoryEngine => write!(f, "memory-engine"),
            Self::MrpRouter => write!(f, "mrp-router"),
            Self::ProvenanceIndex => write!(f, "provenance-index"),
            Self::GovernanceEngine => write!(f, "governance-engine"),
            Self::SafetySuite => write!(f, "safety-suite"),
            Self::Profiler => write!(f, "profiler"),
            Self::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// Metadata attached to every observation event.
///
/// Per I.OBS-3: all observation data is provenance-tagged with source and time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObservationMetadata {
    /// When this observation was recorded.
    pub timestamp: DateTime<Utc>,
    /// Which subsystem produced this observation.
    pub subsystem: SubsystemId,
    /// Associated worldline (if applicable).
    pub worldline_id: Option<WorldlineId>,
    /// Sampling weight for unbiased estimation (1.0 / sampling_rate).
    pub sampling_weight: f64,
}

impl ObservationMetadata {
    /// Create metadata for a fully-sampled event (weight = 1.0).
    pub fn now(subsystem: SubsystemId) -> Self {
        Self {
            timestamp: Utc::now(),
            subsystem,
            worldline_id: None,
            sampling_weight: 1.0,
        }
    }

    /// Create metadata with a specific worldline and sampling weight.
    pub fn with_worldline(
        subsystem: SubsystemId,
        worldline: WorldlineId,
        sampling_rate: f64,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            subsystem,
            worldline_id: Some(worldline),
            sampling_weight: if sampling_rate > 0.0 {
                1.0 / sampling_rate
            } else {
                1.0
            },
        }
    }
}

/// Type of memory operation observed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MemoryOperationType {
    /// Store data to a memory plane.
    Store,
    /// Retrieve data from a memory plane.
    Retrieve,
    /// Consolidate working plane to episodic.
    Consolidate,
    /// Rebuild working plane from episodic + fabric.
    Rebuild,
}

/// Self-observation events emitted by kernel subsystems.
///
/// Per I.OBS-2: these events are MEANING input only -- they never
/// directly trigger action. The type system enforces this: there is
/// no `execute()` method on `SelfObservationEvent`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SelfObservationEvent {
    // ── Event Fabric observations ───────────────────────────────────
    /// An event was emitted into the fabric.
    FabricEventEmitted {
        event_id: EventId,
        stage: ResonanceStage,
        latency: Duration,
        payload_bytes: usize,
    },

    /// An event was routed to subscribers.
    FabricEventRouted {
        event_id: EventId,
        subscriber_count: usize,
        route_latency: Duration,
    },

    // ── Commitment Gate observations ────────────────────────────────
    /// A commitment was submitted through the gate.
    GateSubmission {
        commitment_id: CommitmentId,
        stages_evaluated: u8,
        total_latency: Duration,
        approved: bool,
    },

    /// A specific gate stage was evaluated.
    GateStageEvaluated {
        commitment_id: CommitmentId,
        stage_name: String,
        stage_number: u8,
        latency: Duration,
        passed: bool,
    },

    // ── Memory Engine observations ──────────────────────────────────
    /// A memory operation was performed.
    MemoryOperation {
        operation: MemoryOperationType,
        plane: String,
        latency: Duration,
        entries_affected: usize,
    },

    // ── MRP Router observations ─────────────────────────────────────
    /// A meaning envelope was routed.
    MrpEnvelopeRouted {
        resonance_type: String,
        route_decision: String,
        latency: Duration,
    },

    // ── Governance observations ─────────────────────────────────────
    /// A policy was evaluated.
    PolicyEvaluated {
        policy_id: String,
        latency: Duration,
        result: String,
    },

    /// A constitutional invariant was checked.
    InvariantChecked {
        invariant_name: String,
        passed: bool,
        check_latency: Duration,
    },

    // ── System resource observations ────────────────────────────────
    /// Periodic self-monitoring sample of the observation subsystem itself.
    SystemResourceSample {
        observation_memory_bytes: usize,
        observation_overhead_fraction: f64,
        active_subscriptions: usize,
        ring_buffer_utilization: f64,
    },

    // ── Profiler observations ───────────────────────────────────────
    /// A profiling session completed.
    ProfilingCompleted {
        session_id: String,
        subsystem: SubsystemId,
        duration: Duration,
        samples_collected: usize,
    },
}

impl SelfObservationEvent {
    /// Extract the latency from this event, if applicable.
    pub fn latency(&self) -> Option<Duration> {
        match self {
            Self::FabricEventEmitted { latency, .. } => Some(*latency),
            Self::FabricEventRouted { route_latency, .. } => Some(*route_latency),
            Self::GateSubmission { total_latency, .. } => Some(*total_latency),
            Self::GateStageEvaluated { latency, .. } => Some(*latency),
            Self::MemoryOperation { latency, .. } => Some(*latency),
            Self::MrpEnvelopeRouted { latency, .. } => Some(*latency),
            Self::PolicyEvaluated { latency, .. } => Some(*latency),
            Self::InvariantChecked { check_latency, .. } => Some(*check_latency),
            Self::ProfilingCompleted { duration, .. } => Some(*duration),
            Self::SystemResourceSample { .. } => None,
        }
    }

    /// Whether this event indicates an error or failure.
    pub fn is_error(&self) -> bool {
        match self {
            Self::GateSubmission { approved, .. } => !approved,
            Self::GateStageEvaluated { passed, .. } => !passed,
            Self::InvariantChecked { passed, .. } => !passed,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsystem_id_display() {
        assert_eq!(SubsystemId::EventFabric.to_string(), "event-fabric");
        assert_eq!(SubsystemId::Custom("test".into()).to_string(), "custom:test");
    }

    #[test]
    fn metadata_now_creates_valid_instance() {
        let meta = ObservationMetadata::now(SubsystemId::EventFabric);
        assert_eq!(meta.subsystem, SubsystemId::EventFabric);
        assert_eq!(meta.sampling_weight, 1.0);
        assert!(meta.worldline_id.is_none());
    }

    #[test]
    fn metadata_with_sampling_weight() {
        let wid = maple_mwl_types::WorldlineId::derive(
            &maple_mwl_types::IdentityMaterial::GenesisHash([1u8; 32]),
        );
        let meta = ObservationMetadata::with_worldline(SubsystemId::CommitmentGate, wid, 0.1);
        assert!((meta.sampling_weight - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_latency_extraction() {
        let event = SelfObservationEvent::FabricEventEmitted {
            event_id: maple_mwl_types::EventId::new(),
            stage: ResonanceStage::Meaning,
            latency: Duration::from_millis(5),
            payload_bytes: 256,
        };
        assert_eq!(event.latency(), Some(Duration::from_millis(5)));

        let resource = SelfObservationEvent::SystemResourceSample {
            observation_memory_bytes: 1000,
            observation_overhead_fraction: 0.005,
            active_subscriptions: 2,
            ring_buffer_utilization: 0.5,
        };
        assert_eq!(resource.latency(), None);
    }

    #[test]
    fn event_error_detection() {
        let denied = SelfObservationEvent::GateSubmission {
            commitment_id: maple_mwl_types::CommitmentId::new(),
            stages_evaluated: 3,
            total_latency: Duration::from_millis(10),
            approved: false,
        };
        assert!(denied.is_error());

        let approved = SelfObservationEvent::GateSubmission {
            commitment_id: maple_mwl_types::CommitmentId::new(),
            stages_evaluated: 7,
            total_latency: Duration::from_millis(15),
            approved: true,
        };
        assert!(!approved.is_error());
    }

    #[test]
    fn all_events_serialize() {
        let events = vec![
            SelfObservationEvent::FabricEventEmitted {
                event_id: maple_mwl_types::EventId::new(),
                stage: ResonanceStage::System,
                latency: Duration::from_millis(1),
                payload_bytes: 64,
            },
            SelfObservationEvent::GateSubmission {
                commitment_id: maple_mwl_types::CommitmentId::new(),
                stages_evaluated: 7,
                total_latency: Duration::from_millis(50),
                approved: true,
            },
            SelfObservationEvent::MemoryOperation {
                operation: MemoryOperationType::Store,
                plane: "working".into(),
                latency: Duration::from_micros(200),
                entries_affected: 1,
            },
            SelfObservationEvent::SystemResourceSample {
                observation_memory_bytes: 1024,
                observation_overhead_fraction: 0.003,
                active_subscriptions: 1,
                ring_buffer_utilization: 0.1,
            },
        ];
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let _: SelfObservationEvent = serde_json::from_str(&json).unwrap();
        }
    }
}
