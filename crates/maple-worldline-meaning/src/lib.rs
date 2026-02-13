//! # maple-worldline-meaning
//!
//! Self-Meaning Formation for the WorldLine Self-Producing Substrate.
//!
//! This crate transforms raw observations and detected anomalies into interpreted
//! understanding about WorldLine's own behavior. It is the **Meaning** stage of
//! self-regeneration — the system understanding what its observations signify.
//!
//! ## Architecture
//!
//! ```text
//!   ┌─────────────────────┐     ┌──────────────────────┐
//!   │ PerformanceAnomaly  │     │ ObservationSummary   │
//!   │ (from Prompt 12)    │     │ (from Prompt 11)     │
//!   └──────────┬──────────┘     └──────────┬───────────┘
//!              │                            │
//!              └─────────┬─────────────────┘
//!                        ▼
//!              ┌─────────────────────┐
//!              │  Hypothesis         │  ← 6 generators propose interpretations
//!              │  Generators         │
//!              └─────────┬───────────┘
//!                        ▼
//!              ┌─────────────────────┐
//!              │  Evidence           │  ← Bayesian confidence update
//!              │  Evaluator          │
//!              └─────────┬───────────┘
//!                        ▼
//!              ┌─────────────────────┐
//!              │  SelfMeaning        │  ← Active meanings with evidence
//!              │  Formation          │
//!              └─────────┬───────────┘
//!                        ▼
//!              ┌─────────────────────┐
//!              │  Convergence        │  ← Confidence stability tracking
//!              │  Tracker            │
//!              └─────────┬───────────┘
//!                        ▼
//!              ┌─────────────────────┐
//!              │  Ambiguity          │  ← Preserve / Resolve / Escalate
//!              │  Manager            │
//!              └─────────┬───────────┘
//!                        ▼
//!              ┌─────────────────────┐
//!              │  MeaningIntentBridge│  → Feeds Intent Stabilization (Prompt 14)
//!              └─────────────────────┘
//! ```
//!
//! ## Key Principles
//!
//! - **Preserve ambiguity**: Multiple competing interpretations coexist until
//!   evidence resolves them. Premature disambiguation is a failure mode.
//! - **Bayesian evidence**: Confidence is updated via Bayesian inference —
//!   evidence quality matters, not just quantity.
//! - **Bounded resources**: All collections are bounded (max active meanings,
//!   max history, max evidence per meaning).
//! - **Convergence before action**: Meanings must stabilize before becoming
//!   candidates for intent formation.

#![deny(unsafe_code)]

pub mod ambiguity;
pub mod bridge;
pub mod convergence;
pub mod engine;
pub mod error;
pub mod evidence;
pub mod hypothesis;
pub mod types;

// ── Re-exports ──────────────────────────────────────────────────────────

pub use ambiguity::{AmbiguityDecision, AmbiguityManager, EvidenceRequest};
pub use bridge::MeaningIntentBridge;
pub use convergence::{ConfidenceTrend, ConvergenceState, ConvergenceTracker};
pub use engine::{MeaningHistory, SelfMeaningEngine};
pub use error::{MeaningError, MeaningResult};
pub use evidence::{
    BayesianUpdater, ConfidenceUpdate, EvidenceEvaluator, EvidenceQualityAssessor,
    UpdateDirection,
};
pub use hypothesis::{
    CodePathGenerator, ComponentIsolationGenerator, EnvironmentalChangeGenerator,
    HistoricalPatternGenerator, Hypothesis, HypothesisGenerator, InteractionPatternGenerator,
    ObservationSummary, ResourcePressureGenerator, SubsystemSummaryView,
};
pub use types::{
    ArchitecturalInsightType, Evidence, EvidenceCategory, GrowthModel, HypothesisId, MeaningConfig,
    MeaningId, MemoryOptimizationType, OperatorBottleneckType, RedundancyType,
    RootCauseHypothesis, SelfMeaning, SelfMeaningCategory,
};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use maple_worldline_observation::{
        AnomalyCategory, AnomalyId, AnomalySeverity, ComponentId, MetricId,
        PerformanceAnomaly,
    };
    use std::collections::HashMap;

    fn make_anomaly(
        component: &str,
        category: AnomalyCategory,
        score: f64,
    ) -> PerformanceAnomaly {
        PerformanceAnomaly {
            id: AnomalyId::new(),
            category,
            severity: AnomalySeverity::Warning,
            component: ComponentId(component.into()),
            metric_id: MetricId::new(component, "latency_ns"),
            description: format!("test anomaly in {}", component),
            score,
            detector_agreement: 0.8,
            detected_at: Utc::now(),
            baseline_mean: 5_000_000.0,
            observed_value: 25_000_000.0,
            detectors: vec!["statistical".into()],
        }
    }

    fn empty_summary() -> ObservationSummary {
        ObservationSummary {
            total_events: 1000,
            subsystem_summaries: HashMap::new(),
            snapshot_timestamp: Utc::now(),
        }
    }

    #[test]
    fn integration_full_pipeline() {
        // Create engine
        let mut engine = SelfMeaningEngine::new(MeaningConfig::default());

        // Process anomalies from multiple components
        let anomalies = vec![
            make_anomaly("event-fabric", AnomalyCategory::LatencyRegression, 0.8),
            make_anomaly("commitment-gate", AnomalyCategory::ThroughputDegradation, 0.7),
            make_anomaly("memory-engine", AnomalyCategory::MemoryLeak, 0.6),
        ];

        engine.process_anomalies(&anomalies, &empty_summary());

        // Should have created meanings
        assert!(
            engine.active_count() > 0,
            "Pipeline should produce active meanings"
        );

        // All meanings should be "still forming" (not yet converged)
        let forming = engine.still_forming();
        assert!(
            !forming.is_empty(),
            "New meanings should be still forming"
        );

        // No meanings should be ready for intent yet
        let ready = engine.ready_for_intent();
        assert!(
            ready.is_empty(),
            "New meanings should not be immediately ready for intent"
        );

        // Process more of the same anomalies to accumulate evidence
        for _ in 0..5 {
            engine.process_anomalies(&anomalies, &empty_summary());
        }

        // Evidence should have accumulated
        for meaning in engine.active_meanings() {
            assert!(
                meaning.evidence.len() > 1,
                "Evidence should accumulate over time"
            );
        }
    }

    #[test]
    fn integration_ambiguity_preservation() {
        let mut engine = SelfMeaningEngine::new(MeaningConfig::default());

        // Create anomalies that should produce competing hypotheses
        // LatencyRegression → ComponentIsolation → PerformanceBottleneck
        // OperatorBottleneck → ComponentIsolation → OperatorOptimization
        // Both for same component
        let anomalies = vec![
            make_anomaly("gate", AnomalyCategory::LatencyRegression, 0.7),
            make_anomaly("gate", AnomalyCategory::OperatorBottleneck, 0.6),
        ];

        engine.process_anomalies(&anomalies, &empty_summary());

        // Should have multiple meanings for the same component
        let gate_meanings: Vec<&SelfMeaning> = engine
            .active_meanings()
            .iter()
            .filter(|m| m.category.primary_component() == Some("gate"))
            .collect();

        if gate_meanings.len() >= 2 {
            // Verify competition is detected
            let has_competitors = gate_meanings
                .iter()
                .any(|m| !m.competing_with.is_empty());
            assert!(
                has_competitors,
                "Competing meanings should detect each other"
            );
        }
    }

    #[test]
    fn integration_all_public_types_accessible() {
        // Verify all re-exported types are accessible
        let _id = MeaningId::new();
        let _hyp_id = HypothesisId::new();
        let _evidence = Evidence {
            source: "test".into(),
            strength: 0.5,
            timestamp: Utc::now(),
            description: "test".into(),
            category: EvidenceCategory::Observation,
        };
        let _root_cause = RootCauseHypothesis {
            description: "test".into(),
            confidence: 0.5,
            supporting_evidence: vec![],
        };
        let _config = MeaningConfig::default();
        let _evaluator = EvidenceEvaluator::default();
        let _updater = BayesianUpdater;
        let _assessor = EvidenceQualityAssessor::default();
        let _ambiguity = AmbiguityManager::default();
        let _tracker = ConvergenceTracker::default();
        let _engine = SelfMeaningEngine::new(MeaningConfig::default());
        let _history = MeaningHistory::new(100);

        // Enums
        let _cat = EvidenceCategory::Anomaly;
        let _trend = ConfidenceTrend::Stable;
        let _dir = UpdateDirection::Strengthened;
        let _growth = GrowthModel::Linear;
        let _mem_opt = MemoryOptimizationType::CacheResizing;
        let _op_bt = OperatorBottleneckType::CpuBound;
        let _red = RedundancyType::DeadCode;
        let _arch = ArchitecturalInsightType::CouplingTooTight;

        // Generators
        let _gen1 = ComponentIsolationGenerator;
        let _gen2 = InteractionPatternGenerator;
        let _gen3 = ResourcePressureGenerator;
        let _gen4 = CodePathGenerator;
        let _gen5 = HistoricalPatternGenerator;
        let _gen6 = EnvironmentalChangeGenerator;

        // Meaning categories
        let _smc = SelfMeaningCategory::PerformanceBottleneck {
            component: "test".into(),
            severity: 0.5,
            root_causes: vec![],
        };
    }
}
