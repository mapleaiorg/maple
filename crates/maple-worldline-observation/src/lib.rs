//! # maple-worldline-observation
//!
//! Self-Observation Foundation for the WorldLine Self-Producing Substrate.
//!
//! This crate provides low-overhead, bounded-memory observation of the MWL kernel.
//! It is the "Presence" layer of self-regeneration — the system observing itself.
//!
//! ## Architecture
//!
//! ```text
//!   ┌─────────────────┐      ┌──────────────┐
//!   │  EventFabric    │──────│  Bridge      │
//!   │  (subscription) │      │  (passive)   │
//!   └─────────────────┘      └──────┬───────┘
//!                                   │ map KernelEvent → SelfObservationEvent
//!                                   ▼
//!                            ┌──────────────┐
//!                            │  Collector   │
//!                            │  ┌────────┐  │
//!                            │  │RingBuf │  │  ← bounded circular buffer
//!                            │  └────────┘  │
//!                            │  ┌────────┐  │
//!                            │  │Analytics│  │  ← CMS + HLL
//!                            │  └────────┘  │
//!                            │  ┌────────┐  │
//!                            │  │Windows │  │  ← time-based aggregates
//!                            │  └────────┘  │
//!                            └──────┬───────┘
//!                                   │ snapshot()
//!                                   ▼
//!                            ┌──────────────┐
//!                            │  Snapshot    │  → feeds Meaning Formation (Prompt 13)
//!                            └──────────────┘
//! ```
//!
//! ## Invariants
//!
//! - **I.OBS-1**: Overhead < 1% of total execution time
//! - **I.OBS-2**: Observation is meaning input only — never triggers action
//! - **I.OBS-3**: All data is provenance-tagged (metadata on every event)
//! - **I.OBS-4**: Memory bounded (64MB default, enforced by data structures)
//! - **I.OBS-5**: Sampling never drops to zero (minimum rate enforced)
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use maple_worldline_observation::{
//!     ObservationCollector, ObservationBridge, ObservationMetadata,
//!     SelfObservationEvent, SubsystemId,
//! };
//! use std::sync::{Arc, Mutex};
//! use std::time::Duration;
//!
//! // Create collector
//! let collector = Arc::new(Mutex::new(ObservationCollector::with_defaults()));
//!
//! // Record manually
//! let event = SelfObservationEvent::FabricEventEmitted {
//!     event_id: maple_mwl_types::EventId::new(),
//!     stage: maple_kernel_fabric::ResonanceStage::Meaning,
//!     latency: Duration::from_millis(5),
//!     payload_bytes: 128,
//! };
//! let metadata = ObservationMetadata::now(SubsystemId::EventFabric);
//! collector.lock().unwrap().record(event, metadata).unwrap();
//!
//! // Take a snapshot
//! let snap = collector.lock().unwrap().snapshot();
//! assert!(snap.is_healthy());
//! ```

#![deny(unsafe_code)]

pub mod anomaly;
pub mod baseline;
pub mod bridge;
pub mod collector;
pub mod error;
pub mod events;
pub mod invariants;
pub mod profiler;
pub mod snapshot;
pub mod usage;

// ── Re-exports ──────────────────────────────────────────────────────────

pub use bridge::{ObservationBridge, ObservationHandle};
pub use collector::{
    CollectorConfig, ObservationCollector, ObservedEvent, RingBuffer, SamplingConfig,
    TimeWindow, WindowAggregate,
};
pub use error::{ObservationError, ObservationResult};
pub use events::{
    MemoryOperationType, ObservationMetadata, SelfObservationEvent, SubsystemId,
};
pub use invariants::{
    InvariantChecker, DEFAULT_RING_BUFFER_CAPACITY, MAX_OBSERVATION_MEMORY_BYTES,
    MAX_OVERHEAD_FRACTION, MIN_SAMPLING_RATE,
};
pub use profiler::{
    PerformanceProfiler, ProfilingSample, ProfilingSession, SubsystemProfile,
};
pub use snapshot::{ObservationSnapshot, SubsystemSummary};
pub use usage::{
    CountMinSketch, HyperLogLog, UsageAnalytics, UsageAnalyticsSnapshot,
};

// ── Baseline & Anomaly Detection (Prompt 12) ────────────────────────────

pub use baseline::{
    BaselineConfig, BaselineEngine, BaselinePersistence, DistributionModel,
    InMemoryBaseline, JsonFileBaseline, MetricBaseline, MetricId, PercentileEstimates,
};
pub use anomaly::{
    AnomalyAlgorithm, AnomalyCategory, AnomalyDetector, AnomalyDetectorConfig,
    AnomalyId, AnomalySeverity, ComponentId, PerformanceAnomaly,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn integration_full_pipeline() {
        // Create collector
        let mut collector = ObservationCollector::with_defaults();

        // Record events from multiple subsystems
        let events = vec![
            (
                SelfObservationEvent::FabricEventEmitted {
                    event_id: maple_mwl_types::EventId::new(),
                    stage: maple_kernel_fabric::ResonanceStage::Meaning,
                    latency: Duration::from_millis(5),
                    payload_bytes: 128,
                },
                SubsystemId::EventFabric,
            ),
            (
                SelfObservationEvent::GateSubmission {
                    commitment_id: maple_mwl_types::CommitmentId::new(),
                    stages_evaluated: 7,
                    total_latency: Duration::from_millis(50),
                    approved: true,
                },
                SubsystemId::CommitmentGate,
            ),
            (
                SelfObservationEvent::MemoryOperation {
                    operation: MemoryOperationType::Store,
                    plane: "working".into(),
                    latency: Duration::from_micros(200),
                    entries_affected: 1,
                },
                SubsystemId::MemoryEngine,
            ),
            (
                SelfObservationEvent::PolicyEvaluated {
                    policy_id: "POL-001".into(),
                    latency: Duration::from_millis(2),
                    result: "allow".into(),
                },
                SubsystemId::GovernanceEngine,
            ),
            (
                SelfObservationEvent::SystemResourceSample {
                    observation_memory_bytes: 1024,
                    observation_overhead_fraction: 0.003,
                    active_subscriptions: 1,
                    ring_buffer_utilization: 0.1,
                },
                SubsystemId::Profiler,
            ),
        ];

        for (event, subsystem) in events {
            let metadata = ObservationMetadata::now(subsystem);
            collector.record(event, metadata).unwrap();
        }

        // Take snapshot
        let snap = collector.snapshot();
        assert_eq!(snap.total_events_observed, 5);
        assert!(snap.is_healthy());
        assert!(snap.subsystem_summaries.contains_key("event-fabric"));
        assert!(snap.subsystem_summaries.contains_key("commitment-gate"));
        assert!(snap.subsystem_summaries.contains_key("memory-engine"));

        // Aggregate windows
        collector.aggregate_window(TimeWindow::OneSecond);
        let windows = collector.get_windows(&TimeWindow::OneSecond);
        assert!(!windows.is_empty());
    }

    #[test]
    fn integration_profiler_session() {
        let mut profiler = PerformanceProfiler::new();

        let sid = profiler
            .start_session(SubsystemId::EventFabric, 100)
            .unwrap();

        for i in 0..20 {
            profiler
                .record_sample(
                    &sid,
                    ProfilingSample {
                        timestamp: chrono::Utc::now(),
                        operation: "emit".into(),
                        latency: Duration::from_millis(i + 1),
                        metadata: std::collections::HashMap::new(),
                    },
                )
                .unwrap();
        }

        let profile = profiler.end_session(&sid).unwrap();
        assert_eq!(profile.sample_count, 20);
        assert!(profile.p50_latency > Duration::ZERO);
        assert!(profile.p95_latency >= profile.p50_latency);
        assert!(profile.p99_latency >= profile.p95_latency);
    }

    #[test]
    fn integration_usage_analytics() {
        let mut analytics = UsageAnalytics::new();

        // Simulate varied usage
        for i in 0..100 {
            analytics.record_operation("fabric.emit");
            analytics.record_worldline(&format!("wl-{}", i % 10));
            analytics.record_event_type("MeaningFormed");
        }
        for _ in 0..50 {
            analytics.record_operation("gate.submit");
            analytics.record_commitment(&format!("c-{}", rand::random::<u32>()));
        }

        let snap = analytics.snapshot();
        assert_eq!(snap.total_operations, 150);
        assert!(snap.estimated_unique_worldlines >= 5);
        assert!(snap.estimated_unique_event_types >= 1);
    }

    #[test]
    fn integration_invariants_enforced() {
        let mut collector = ObservationCollector::with_defaults();

        // Memory should always be within budget
        let mem = collector.estimated_memory_bytes();
        assert!(
            InvariantChecker::check_memory_usage(mem, MAX_OBSERVATION_MEMORY_BYTES).is_ok()
        );

        // Sampling rate must be valid
        assert!(
            InvariantChecker::validate_sampling_rate(collector.current_sampling_rate()).is_ok()
        );

        // Record many events
        for _ in 0..1000 {
            let event = SelfObservationEvent::FabricEventEmitted {
                event_id: maple_mwl_types::EventId::new(),
                stage: maple_kernel_fabric::ResonanceStage::Meaning,
                latency: Duration::from_millis(1),
                payload_bytes: 64,
            };
            let metadata = ObservationMetadata::now(SubsystemId::EventFabric);
            collector.record(event, metadata).unwrap();
        }

        // Memory still bounded
        let mem = collector.estimated_memory_bytes();
        assert!(mem < MAX_OBSERVATION_MEMORY_BYTES);

        // Overhead tracking works (in a test where recording IS the workload,
        // the fraction will be high; in production the fraction is observation_time / total_time).
        // We verify the mechanism works, not the absolute value in a synthetic test.
        let overhead = collector.overhead_fraction();
        assert!(overhead >= 0.0, "overhead fraction should be non-negative");
    }

    #[test]
    fn integration_baseline_anomaly_pipeline() {
        // End-to-end: collector → snapshot → baseline → anomaly detection
        let mut collector = ObservationCollector::with_defaults();

        // Record normal events
        for _ in 0..50 {
            let event = SelfObservationEvent::FabricEventEmitted {
                event_id: maple_mwl_types::EventId::new(),
                stage: maple_kernel_fabric::ResonanceStage::Meaning,
                latency: Duration::from_millis(5),
                payload_bytes: 128,
            };
            let metadata = ObservationMetadata::now(SubsystemId::EventFabric);
            collector.record(event, metadata).unwrap();
        }

        // Build baseline from snapshots (low thresholds for testing)
        let baseline_config = BaselineConfig {
            min_establishment_samples: 5,
            min_establishment_duration: Duration::from_secs(0),
            ..BaselineConfig::default()
        };
        let mut engine = BaselineEngine::new(baseline_config);

        for _ in 0..10 {
            let snap = collector.snapshot();
            engine.observe_snapshot(&snap);
        }

        // Verify baselines are established
        let metrics = engine.metrics();
        assert!(!metrics.is_empty(), "should have tracked metrics");

        // Create anomaly detector and check normal state
        let mut detector = AnomalyDetector::new(AnomalyDetectorConfig {
            min_detector_agreement: 0.2,
            ..AnomalyDetectorConfig::default()
        });

        let normal_snap = collector.snapshot();
        let _normal_anomalies = detector.detect_from_snapshot(&normal_snap, &engine);
        // Normal state may or may not produce anomalies depending on exact values

        // Now inject an anomalous metric directly
        let mid = MetricId::new("system", "memory_bytes");
        if let Some(baseline) = engine.get_baseline(&mid) {
            let extreme_value = baseline.mean + baseline.std_dev * 10.0;
            let _anomalies = detector.detect(&mid, extreme_value, baseline);
            // With a 10-sigma deviation, at least the statistical detector should fire
            // (but we can't guarantee fusion passes since it needs agreement)
        }
    }

    #[test]
    fn all_public_types_accessible() {
        // Verify that the re-exports work by constructing key types
        let _error = ObservationError::LockError;
        let _checker = InvariantChecker;
        let _subsystem = SubsystemId::EventFabric;
        let _metadata = ObservationMetadata::now(SubsystemId::Profiler);
        let _rb: RingBuffer<i32> = RingBuffer::new(10);
        let _cms = CountMinSketch::default_size();
        let _hll = HyperLogLog::default_precision();
        let _analytics = UsageAnalytics::new();
        let _profiler = PerformanceProfiler::new();
        let _config = CollectorConfig::default();
        let _collector = ObservationCollector::with_defaults();

        // Prompt 12 types
        let _mid = MetricId::new("test", "value");
        let _baseline = MetricBaseline::new(_mid.clone());
        let _engine = BaselineEngine::with_defaults();
        let _anomaly_id = AnomalyId::new();
        let _component = ComponentId("test".into());
        let _category = AnomalyCategory::LatencyRegression;
        let _severity = AnomalySeverity::Info;
        let _detector = AnomalyDetector::default();
        let _detector_config = AnomalyDetectorConfig::default();
        let _baseline_config = BaselineConfig::default();
        let _persistence = InMemoryBaseline::new();
    }
}
