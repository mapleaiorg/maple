//! Event Fabric: foundational event-sourced substrate for the MWL kernel.
//!
//! All state changes flow through the Event Fabric as immutable, causally-ordered events.
//! The fabric provides:
//! - Crash-recoverable persistence via WAL
//! - Causal ordering via HLC
//! - Event routing to kernel modules
//! - Integrity verification

pub mod error;
pub mod event;
pub mod hlc;
pub mod metrics;
pub mod router;
pub mod traits;
pub mod types;
pub mod wal;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::info;

pub use error::FabricError;
pub use event::{EventPayload, KernelEvent, ResonanceStage};
pub use hlc::{HlcTimestamp, HybridLogicalClock};
pub use metrics::FabricMetrics;
pub use router::CausalRouter;
pub use traits::{FabricConsumer, FabricProducer};
pub use types::*;
pub use wal::{SyncMode, WalConfig, WriteAheadLog};

/// Configuration for the Event Fabric.
pub struct FabricConfig {
    /// Node identifier for this instance
    pub node_id: NodeId,
    /// WAL data directory (None = in-memory)
    pub data_dir: Option<PathBuf>,
    /// WAL configuration
    pub wal: WalConfig,
    /// Maximum HLC clock drift (ms)
    pub max_clock_drift_ms: u64,
}

impl Default for FabricConfig {
    fn default() -> Self {
        Self {
            node_id: NodeId(1),
            data_dir: None,
            wal: WalConfig::default(),
            max_clock_drift_ms: 1000,
        }
    }
}

/// The Event Fabric: foundational event-sourced substrate of the MWL kernel.
///
/// All state changes flow through the Event Fabric as immutable, causally-ordered events.
pub struct EventFabric {
    hlc: Arc<HybridLogicalClock>,
    wal: Arc<WriteAheadLog>,
    router: Arc<CausalRouter>,
}

impl EventFabric {
    /// Initialize the Event Fabric.
    pub async fn init(config: FabricConfig) -> Result<Self, FabricError> {
        let node_id_val = config.node_id.0;
        let hlc = Arc::new(HybridLogicalClock::with_max_drift(
            config.node_id,
            config.max_clock_drift_ms,
        ));

        let wal = match config.data_dir {
            Some(dir) => Arc::new(WriteAheadLog::open_file(config.wal, dir).await?),
            None => Arc::new(WriteAheadLog::open_memory(config.wal).await?),
        };

        let router = Arc::new(CausalRouter::new(hlc.clone()));

        info!(node_id = node_id_val, "Event Fabric initialized");

        Ok(Self { hlc, wal, router })
    }

    /// Emit an event into the fabric.
    ///
    /// This:
    /// 1. Generates an HLC timestamp
    /// 2. Computes integrity hash
    /// 3. Appends to WAL
    /// 4. Routes to subscribers
    pub async fn emit(
        &self,
        worldline_id: WorldlineId,
        stage: ResonanceStage,
        payload: EventPayload,
        parents: Vec<EventId>,
    ) -> Result<KernelEvent, FabricError> {
        let timestamp = self.hlc.now();
        let id = EventId::new();

        let event = KernelEvent::new(id, timestamp, worldline_id, stage, payload, parents);

        // Persist to WAL
        self.wal.append(&event).await?;

        // Route to subscribers
        self.router.route(&event).await?;

        Ok(event)
    }

    /// Emit a batch of events atomically.
    pub async fn emit_batch(
        &self,
        items: Vec<(WorldlineId, ResonanceStage, EventPayload, Vec<EventId>)>,
    ) -> Result<Vec<KernelEvent>, FabricError> {
        let mut events = Vec::with_capacity(items.len());

        for (worldline_id, stage, payload, parents) in items {
            let timestamp = self.hlc.now();
            let id = EventId::new();
            let event = KernelEvent::new(id, timestamp, worldline_id, stage, payload, parents);
            events.push(event);
        }

        // Persist all to WAL
        self.wal.append_batch(&events).await?;

        // Route each event
        for event in &events {
            self.router.route(event).await?;
        }

        Ok(events)
    }

    /// Subscribe to events.
    pub async fn subscribe(
        &self,
        stages: Option<Vec<ResonanceStage>>,
        worldlines: Option<Vec<WorldlineId>>,
    ) -> mpsc::Receiver<KernelEvent> {
        let (_id, rx) = self.router.subscribe(stages, worldlines).await;
        rx
    }

    /// Recover state from WAL after crash.
    pub async fn recover<F>(&self, handler: F) -> Result<u64, FabricError>
    where
        F: FnMut(u64, KernelEvent) -> Result<(), FabricError>,
    {
        self.wal.replay(1, handler).await
    }

    /// Get the HLC for external use.
    pub fn clock(&self) -> &HybridLogicalClock {
        &self.hlc
    }

    /// Verify fabric integrity.
    pub async fn verify(&self) -> Result<IntegrityReport, FabricError> {
        self.wal.verify_integrity().await
    }

    /// Checkpoint and return the sequence number.
    pub async fn checkpoint(&self) -> Result<u64, FabricError> {
        self.wal.checkpoint().await
    }

    /// Get current fabric metrics.
    pub async fn metrics(&self) -> FabricMetrics {
        let segments = {
            // Access WAL segments info via latest_sequence
            // For a complete implementation we'd expose segment count from WAL
            0u32
        };

        FabricMetrics {
            events_total: self.wal.latest_sequence(),
            wal_size_bytes: 0, // Would need WAL to expose this
            wal_segments: segments,
            latest_sequence: self.wal.latest_sequence(),
            subscribers_active: self.router.subscription_count().await as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_mwl_types::IdentityMaterial;

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    #[tokio::test]
    async fn fabric_emit_and_subscribe() {
        let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();

        let mut rx = fabric.subscribe(None, None).await;

        let event = fabric
            .emit(
                test_worldline(),
                ResonanceStage::Meaning,
                EventPayload::MeaningFormed {
                    interpretation_count: 1,
                    confidence: 0.85,
                    ambiguity_preserved: true,
                },
                vec![],
            )
            .await
            .unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.id, event.id);
        assert!(received.verify_integrity());
    }

    #[tokio::test]
    async fn fabric_emit_batch() {
        let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();

        let wid = test_worldline();
        let items = vec![
            (
                wid.clone(),
                ResonanceStage::Meaning,
                EventPayload::MeaningFormed {
                    interpretation_count: 1,
                    confidence: 0.7,
                    ambiguity_preserved: true,
                },
                vec![],
            ),
            (
                wid.clone(),
                ResonanceStage::Intent,
                EventPayload::IntentStabilized {
                    direction: "forward".into(),
                    confidence: 0.9,
                    conditions: vec![],
                },
                vec![],
            ),
        ];

        let events = fabric.emit_batch(items).await.unwrap();
        assert_eq!(events.len(), 2);

        // Verify causal ordering
        assert!(events[0].timestamp < events[1].timestamp);
    }

    #[tokio::test]
    async fn fabric_recover() {
        let dir = tempfile::tempdir().unwrap();

        let wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]));

        // Emit events
        {
            let config = FabricConfig {
                data_dir: Some(dir.path().to_path_buf()),
                ..FabricConfig::default()
            };
            let fabric = EventFabric::init(config).await.unwrap();

            for i in 0..5 {
                fabric
                    .emit(
                        wid.clone(),
                        ResonanceStage::Meaning,
                        EventPayload::MeaningFormed {
                            interpretation_count: i,
                            confidence: 0.5,
                            ambiguity_preserved: true,
                        },
                        vec![],
                    )
                    .await
                    .unwrap();
            }
            fabric.checkpoint().await.unwrap();
        }

        // Recover
        {
            let config = FabricConfig {
                data_dir: Some(dir.path().to_path_buf()),
                ..FabricConfig::default()
            };
            let fabric = EventFabric::init(config).await.unwrap();

            let mut recovered = Vec::new();
            let count = fabric
                .recover(|seq, event| {
                    recovered.push((seq, event));
                    Ok(())
                })
                .await
                .unwrap();

            assert_eq!(count, 5);
            for (_, event) in &recovered {
                assert!(event.verify_integrity());
            }
        }
    }

    #[tokio::test]
    async fn fabric_verify_integrity() {
        let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();

        for i in 0..10 {
            fabric
                .emit(
                    test_worldline(),
                    ResonanceStage::Meaning,
                    EventPayload::MeaningFormed {
                        interpretation_count: i,
                        confidence: 0.5,
                        ambiguity_preserved: true,
                    },
                    vec![],
                )
                .await
                .unwrap();
        }

        let report = fabric.verify().await.unwrap();
        assert!(report.is_clean());
        assert_eq!(report.total_events, 10);
        assert_eq!(report.verified_events, 10);
    }

    #[tokio::test]
    async fn fabric_stage_subscription_routing() {
        let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();

        let mut commitment_rx = fabric
            .subscribe(Some(vec![ResonanceStage::Commitment]), None)
            .await;
        let mut all_rx = fabric.subscribe(None, None).await;

        // Emit a meaning event — should only go to all_rx
        fabric
            .emit(
                test_worldline(),
                ResonanceStage::Meaning,
                EventPayload::MeaningFormed {
                    interpretation_count: 1,
                    confidence: 0.5,
                    ambiguity_preserved: false,
                },
                vec![],
            )
            .await
            .unwrap();

        // all_rx should receive it
        let _received = all_rx.recv().await.unwrap();

        // commitment_rx should NOT have it
        assert!(commitment_rx.try_recv().is_err());

        // Emit a commitment event — should go to both
        fabric
            .emit(
                test_worldline(),
                ResonanceStage::Commitment,
                EventPayload::CommitmentDeclared {
                    commitment_id: maple_mwl_types::CommitmentId::new(),
                    scope: serde_json::json!({}),
                    parties: vec![],
                },
                vec![],
            )
            .await
            .unwrap();

        let _from_all = all_rx.recv().await.unwrap();
        let from_commitment = commitment_rx.recv().await.unwrap();
        assert_eq!(from_commitment.stage, ResonanceStage::Commitment);
    }

    #[tokio::test]
    async fn fabric_metrics() {
        let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();

        let _rx = fabric.subscribe(None, None).await;

        fabric
            .emit(
                test_worldline(),
                ResonanceStage::System,
                EventPayload::WorldlineCreated {
                    profile: "test".into(),
                },
                vec![],
            )
            .await
            .unwrap();

        let m = fabric.metrics().await;
        assert_eq!(m.events_total, 1);
        assert_eq!(m.latest_sequence, 1);
        assert_eq!(m.subscribers_active, 1);
    }

    #[tokio::test]
    async fn many_events_concurrent_emitters() {
        let fabric = Arc::new(EventFabric::init(FabricConfig::default()).await.unwrap());

        let mut handles = vec![];
        for i in 0..4 {
            let fabric = fabric.clone();
            let wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([i as u8; 32]));
            handles.push(tokio::spawn(async move {
                for j in 0..100 {
                    fabric
                        .emit(
                            wid.clone(),
                            ResonanceStage::Meaning,
                            EventPayload::MeaningFormed {
                                interpretation_count: j,
                                confidence: 0.5,
                                ambiguity_preserved: true,
                            },
                            vec![],
                        )
                        .await
                        .unwrap();
                }
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let report = fabric.verify().await.unwrap();
        assert!(report.is_clean());
        assert_eq!(report.total_events, 400);
    }
}
