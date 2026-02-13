//! Observation Bridge — connects EventFabric to the ObservationCollector.
//!
//! The bridge passively subscribes to the EventFabric and translates
//! `KernelEvent`/`EventPayload` into `SelfObservationEvent`s.
//!
//! Per I.OBS-2: the bridge is read-only — it never modifies existing crates.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use maple_kernel_fabric::{EventFabric, EventPayload, KernelEvent, ResonanceStage};
use maple_mwl_types::CommitmentId;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::collector::ObservationCollector;
use crate::error::ObservationResult;
use crate::events::{
    MemoryOperationType, ObservationMetadata, SelfObservationEvent, SubsystemId,
};

/// Bridge connecting EventFabric subscriptions to the ObservationCollector.
///
/// The bridge:
/// 1. Subscribes to EventFabric (all stages, all worldlines)
/// 2. Maps `KernelEvent` → `SelfObservationEvent`
/// 3. Feeds events into the `ObservationCollector`
///
/// It does NOT modify the EventFabric or any other kernel crate.
pub struct ObservationBridge {
    /// The collector receiving mapped events.
    collector: Arc<Mutex<ObservationCollector>>,
}

impl ObservationBridge {
    /// Create a new bridge wrapping a collector.
    pub fn new(collector: Arc<Mutex<ObservationCollector>>) -> Self {
        Self { collector }
    }

    /// Start observing the EventFabric.
    ///
    /// Spawns a background tokio task that reads from the fabric's subscription
    /// channel and feeds events into the collector. Returns a handle to stop
    /// the observation.
    pub async fn observe_fabric(
        &self,
        fabric: &EventFabric,
    ) -> ObservationResult<ObservationHandle> {
        let rx = fabric.subscribe(None, None).await;
        let collector = self.collector.clone();

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);

        let handle = tokio::spawn(async move {
            let mut rx = rx;
            loop {
                tokio::select! {
                    event = rx.recv() => {
                        match event {
                            Some(kernel_event) => {
                                let observation = Self::map_kernel_event(&kernel_event);
                                if let Some((obs_event, metadata)) = observation {
                                    if let Ok(mut c) = collector.lock() {
                                        if let Err(e) = c.record(obs_event, metadata) {
                                            warn!(error = %e, "observation recording failed");
                                        }
                                    }
                                }
                            }
                            None => {
                                debug!("fabric subscription closed");
                                break;
                            }
                        }
                    }
                    _ = stop_rx.recv() => {
                        debug!("observation bridge stopped");
                        break;
                    }
                }
            }
        });

        Ok(ObservationHandle {
            _task: handle,
            stop: stop_tx,
        })
    }

    /// Map a KernelEvent to a SelfObservationEvent.
    ///
    /// Returns None if the event type doesn't have a meaningful observation mapping.
    fn map_kernel_event(
        event: &KernelEvent,
    ) -> Option<(SelfObservationEvent, ObservationMetadata)> {
        let worldline_id = event.worldline_id.clone();
        let metadata = ObservationMetadata::with_worldline(
            Self::stage_to_subsystem(&event.stage),
            worldline_id,
            1.0, // bridge events are not pre-sampled; collector handles sampling
        );

        let obs_event = match &event.payload {
            // Presence events → Fabric observations
            EventPayload::PresenceAsserted { .. } | EventPayload::PresenceWithdrawn { .. } => {
                SelfObservationEvent::FabricEventEmitted {
                    event_id: event.id.clone(),
                    stage: event.stage,
                    latency: Duration::from_micros(0), // no latency info in payload
                    payload_bytes: estimate_payload_size(&event.payload),
                }
            }

            // Coupling events → Fabric observations
            EventPayload::CouplingEstablished { .. }
            | EventPayload::CouplingModified { .. }
            | EventPayload::CouplingSevered { .. } => {
                SelfObservationEvent::FabricEventRouted {
                    event_id: event.id.clone(),
                    subscriber_count: 0, // not available from event alone
                    route_latency: Duration::from_micros(0),
                }
            }

            // Meaning events → Fabric observations
            EventPayload::MeaningFormed { .. } | EventPayload::MeaningRevised { .. } => {
                SelfObservationEvent::FabricEventEmitted {
                    event_id: event.id.clone(),
                    stage: event.stage,
                    latency: Duration::from_micros(0),
                    payload_bytes: estimate_payload_size(&event.payload),
                }
            }

            // Commitment events → Gate observations
            EventPayload::CommitmentDeclared { commitment_id, .. } => {
                SelfObservationEvent::GateSubmission {
                    commitment_id: commitment_id.clone(),
                    stages_evaluated: 0,
                    total_latency: Duration::from_micros(0),
                    approved: true, // declared = in progress
                }
            }
            EventPayload::CommitmentApproved { commitment_id, .. } => {
                SelfObservationEvent::GateSubmission {
                    commitment_id: commitment_id.clone(),
                    stages_evaluated: 7,
                    total_latency: Duration::from_micros(0),
                    approved: true,
                }
            }
            EventPayload::CommitmentDenied { commitment_id, .. } => {
                SelfObservationEvent::GateSubmission {
                    commitment_id: commitment_id.clone(),
                    stages_evaluated: 0,
                    total_latency: Duration::from_micros(0),
                    approved: false,
                }
            }
            EventPayload::CommitmentFulfilled { .. }
            | EventPayload::CommitmentFailed { .. } => {
                SelfObservationEvent::FabricEventEmitted {
                    event_id: event.id.clone(),
                    stage: event.stage,
                    latency: Duration::from_micros(0),
                    payload_bytes: estimate_payload_size(&event.payload),
                }
            }

            // Governance events
            EventPayload::PolicyEvaluated { policy_id, result } => {
                SelfObservationEvent::PolicyEvaluated {
                    policy_id: policy_id.clone(),
                    latency: Duration::from_micros(0),
                    result: result.clone(),
                }
            }
            EventPayload::InvariantChecked {
                invariant_id,
                passed,
            } => SelfObservationEvent::InvariantChecked {
                invariant_name: invariant_id.clone(),
                passed: *passed,
                check_latency: Duration::from_micros(0),
            },

            // System events → Fabric observations
            EventPayload::WorldlineCreated { .. }
            | EventPayload::WorldlineDestroyed { .. }
            | EventPayload::CheckpointCreated { .. } => {
                SelfObservationEvent::FabricEventEmitted {
                    event_id: event.id.clone(),
                    stage: event.stage,
                    latency: Duration::from_micros(0),
                    payload_bytes: estimate_payload_size(&event.payload),
                }
            }

            // Remaining events
            _ => {
                SelfObservationEvent::FabricEventEmitted {
                    event_id: event.id.clone(),
                    stage: event.stage,
                    latency: Duration::from_micros(0),
                    payload_bytes: estimate_payload_size(&event.payload),
                }
            }
        };

        Some((obs_event, metadata))
    }

    /// Map a resonance stage to the most likely subsystem.
    fn stage_to_subsystem(stage: &ResonanceStage) -> SubsystemId {
        match stage {
            ResonanceStage::Presence => SubsystemId::EventFabric,
            ResonanceStage::Coupling => SubsystemId::EventFabric,
            ResonanceStage::Meaning => SubsystemId::EventFabric,
            ResonanceStage::Intent => SubsystemId::EventFabric,
            ResonanceStage::Commitment => SubsystemId::CommitmentGate,
            ResonanceStage::Consequence => SubsystemId::EventFabric,
            ResonanceStage::Governance => SubsystemId::GovernanceEngine,
            ResonanceStage::System => SubsystemId::EventFabric,
        }
    }

    // ── Manual recording methods ────────────────────────────────────

    /// Manually record a gate submission observation.
    pub fn record_gate_submission(
        &self,
        commitment_id: CommitmentId,
        stages: u8,
        latency: Duration,
        approved: bool,
    ) {
        let event = SelfObservationEvent::GateSubmission {
            commitment_id,
            stages_evaluated: stages,
            total_latency: latency,
            approved,
        };
        let metadata = ObservationMetadata::now(SubsystemId::CommitmentGate);
        if let Ok(mut c) = self.collector.lock() {
            let _ = c.record(event, metadata);
        }
    }

    /// Manually record a memory operation observation.
    pub fn record_memory_operation(
        &self,
        operation: MemoryOperationType,
        plane: &str,
        latency: Duration,
        entries: usize,
    ) {
        let event = SelfObservationEvent::MemoryOperation {
            operation,
            plane: plane.to_string(),
            latency,
            entries_affected: entries,
        };
        let metadata = ObservationMetadata::now(SubsystemId::MemoryEngine);
        if let Ok(mut c) = self.collector.lock() {
            let _ = c.record(event, metadata);
        }
    }

    /// Manually record a governance check observation.
    pub fn record_governance_check(
        &self,
        invariant_name: &str,
        passed: bool,
        latency: Duration,
    ) {
        let event = SelfObservationEvent::InvariantChecked {
            invariant_name: invariant_name.to_string(),
            passed,
            check_latency: latency,
        };
        let metadata = ObservationMetadata::now(SubsystemId::GovernanceEngine);
        if let Ok(mut c) = self.collector.lock() {
            let _ = c.record(event, metadata);
        }
    }

    /// Get access to the underlying collector.
    pub fn collector(&self) -> &Arc<Mutex<ObservationCollector>> {
        &self.collector
    }
}

/// Estimate the serialized size of an event payload.
fn estimate_payload_size(payload: &EventPayload) -> usize {
    serde_json::to_vec(payload).map(|v| v.len()).unwrap_or(0)
}

/// Handle for a running observation bridge task.
pub struct ObservationHandle {
    _task: tokio::task::JoinHandle<()>,
    stop: mpsc::Sender<()>,
}

impl ObservationHandle {
    /// Stop the observation bridge.
    pub async fn stop(self) {
        let _ = self.stop.send(()).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_kernel_fabric::FabricConfig;
    use maple_mwl_types::IdentityMaterial;

    fn test_worldline() -> maple_mwl_types::WorldlineId {
        maple_mwl_types::WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    #[test]
    fn map_meaning_event() {
        let event = KernelEvent::new(
            maple_mwl_types::EventId::new(),
            maple_kernel_fabric::HlcTimestamp {
                physical: 1000,
                logical: 0,
                node_id: maple_kernel_fabric::NodeId(1),
            },
            test_worldline(),
            ResonanceStage::Meaning,
            EventPayload::MeaningFormed {
                interpretation_count: 3,
                confidence: 0.85,
                ambiguity_preserved: true,
            },
            vec![],
        );

        let result = ObservationBridge::map_kernel_event(&event);
        assert!(result.is_some());
        let (obs, meta) = result.unwrap();
        assert_eq!(meta.subsystem, SubsystemId::EventFabric);
        matches!(obs, SelfObservationEvent::FabricEventEmitted { .. });
    }

    #[test]
    fn map_commitment_approved() {
        let cid = maple_mwl_types::CommitmentId::new();
        let event = KernelEvent::new(
            maple_mwl_types::EventId::new(),
            maple_kernel_fabric::HlcTimestamp {
                physical: 2000,
                logical: 0,
                node_id: maple_kernel_fabric::NodeId(1),
            },
            test_worldline(),
            ResonanceStage::Commitment,
            EventPayload::CommitmentApproved {
                commitment_id: cid.clone(),
                decision_card: serde_json::json!({"result": "approved"}),
            },
            vec![],
        );

        let result = ObservationBridge::map_kernel_event(&event);
        assert!(result.is_some());
        let (obs, meta) = result.unwrap();
        assert_eq!(meta.subsystem, SubsystemId::CommitmentGate);
        match obs {
            SelfObservationEvent::GateSubmission { approved, .. } => assert!(approved),
            _ => panic!("expected GateSubmission"),
        }
    }

    #[test]
    fn map_policy_evaluated() {
        let event = KernelEvent::new(
            maple_mwl_types::EventId::new(),
            maple_kernel_fabric::HlcTimestamp {
                physical: 3000,
                logical: 0,
                node_id: maple_kernel_fabric::NodeId(1),
            },
            test_worldline(),
            ResonanceStage::Governance,
            EventPayload::PolicyEvaluated {
                policy_id: "POL-001".into(),
                result: "allow".into(),
            },
            vec![],
        );

        let result = ObservationBridge::map_kernel_event(&event);
        assert!(result.is_some());
        let (obs, meta) = result.unwrap();
        assert_eq!(meta.subsystem, SubsystemId::GovernanceEngine);
        match obs {
            SelfObservationEvent::PolicyEvaluated { policy_id, .. } => {
                assert_eq!(policy_id, "POL-001");
            }
            _ => panic!("expected PolicyEvaluated"),
        }
    }

    #[test]
    fn manual_recording() {
        let collector = Arc::new(Mutex::new(ObservationCollector::with_defaults()));
        let bridge = ObservationBridge::new(collector.clone());

        bridge.record_gate_submission(
            maple_mwl_types::CommitmentId::new(),
            7,
            Duration::from_millis(50),
            true,
        );

        bridge.record_memory_operation(
            MemoryOperationType::Store,
            "working",
            Duration::from_micros(200),
            1,
        );

        bridge.record_governance_check("I.AAS-1", true, Duration::from_millis(1));

        let c = collector.lock().unwrap();
        assert_eq!(c.total_events(), 3);
    }

    #[tokio::test]
    async fn bridge_observe_fabric() {
        let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();
        let collector = Arc::new(Mutex::new(ObservationCollector::with_defaults()));
        let bridge = ObservationBridge::new(collector.clone());

        let handle = bridge.observe_fabric(&fabric).await.unwrap();

        // Emit some events
        for i in 0..5 {
            fabric
                .emit(
                    test_worldline(),
                    ResonanceStage::Meaning,
                    EventPayload::MeaningFormed {
                        interpretation_count: i,
                        confidence: 0.7,
                        ambiguity_preserved: true,
                    },
                    vec![],
                )
                .await
                .unwrap();
        }

        // Give the bridge time to process
        tokio::time::sleep(Duration::from_millis(100)).await;

        let c = collector.lock().unwrap();
        assert!(c.total_events() >= 1, "bridge should have recorded events");

        drop(c);
        handle.stop().await;
    }

    #[test]
    fn stage_to_subsystem_mapping() {
        assert_eq!(
            ObservationBridge::stage_to_subsystem(&ResonanceStage::Commitment),
            SubsystemId::CommitmentGate
        );
        assert_eq!(
            ObservationBridge::stage_to_subsystem(&ResonanceStage::Governance),
            SubsystemId::GovernanceEngine
        );
        assert_eq!(
            ObservationBridge::stage_to_subsystem(&ResonanceStage::Meaning),
            SubsystemId::EventFabric
        );
    }
}
