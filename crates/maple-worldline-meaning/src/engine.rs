//! Central meaning formation engine.
//!
//! The `SelfMeaningEngine` orchestrates the full pipeline:
//! anomalies → hypothesis generation → evidence evaluation → convergence tracking
//! → ambiguity management → meaning formation.

use std::collections::VecDeque;

use chrono::Utc;
use tracing::debug;

use maple_worldline_observation::PerformanceAnomaly;

use crate::ambiguity::{AmbiguityDecision, AmbiguityManager};
use crate::bridge::MeaningIntentBridge;
use crate::convergence::ConvergenceTracker;
use crate::evidence::EvidenceEvaluator;
use crate::hypothesis::{
    CodePathGenerator, ComponentIsolationGenerator, EnvironmentalChangeGenerator,
    HistoricalPatternGenerator, HypothesisGenerator, InteractionPatternGenerator,
    ObservationSummary, ResourcePressureGenerator,
};
use crate::types::{
    Evidence, EvidenceCategory, MeaningConfig, MeaningId, SelfMeaning,
    SelfMeaningCategory,
};

// ── Meaning History ─────────────────────────────────────────────────────

/// Bounded history of past meanings for pattern learning.
#[derive(Clone, Debug)]
pub struct MeaningHistory {
    /// Past meanings (oldest first).
    pub meanings: VecDeque<SelfMeaning>,
    /// Maximum number of entries.
    max_entries: usize,
}

impl MeaningHistory {
    /// Create a new bounded history.
    pub fn new(max_entries: usize) -> Self {
        Self {
            meanings: VecDeque::with_capacity(max_entries.min(1024)),
            max_entries,
        }
    }

    /// Add a meaning to history.
    pub fn push(&mut self, meaning: SelfMeaning) {
        if self.meanings.len() >= self.max_entries {
            self.meanings.pop_front();
        }
        self.meanings.push_back(meaning);
    }

    /// Get all historical meanings as a slice-compatible view.
    pub fn as_slice(&self) -> Vec<&SelfMeaning> {
        self.meanings.iter().collect()
    }

    /// Number of entries in history.
    pub fn len(&self) -> usize {
        self.meanings.len()
    }

    /// Whether history is empty.
    pub fn is_empty(&self) -> bool {
        self.meanings.is_empty()
    }
}

// ── Self-Meaning Engine ─────────────────────────────────────────────────

/// Central engine for meaning formation.
///
/// Orchestrates hypothesis generators, evidence evaluator, ambiguity manager,
/// and convergence tracker to transform anomalies into interpreted meanings
/// about WorldLine's own behavior.
pub struct SelfMeaningEngine {
    /// Hypothesis generators (produce possible interpretations).
    generators: Vec<Box<dyn HypothesisGenerator>>,
    /// Evidence evaluator (scores hypotheses against evidence).
    evaluator: EvidenceEvaluator,
    /// Ambiguity manager (tracks competing interpretations).
    ambiguity_manager: AmbiguityManager,
    /// Convergence tracker (when is meaning stable enough?).
    convergence_tracker: ConvergenceTracker,
    /// Currently active meanings (may be competing).
    active_meanings: Vec<SelfMeaning>,
    /// Meanings that were abandoned (evidence collapsed).
    abandoned_meanings: Vec<SelfMeaning>,
    /// Meaning history (for pattern learning).
    history: MeaningHistory,
    /// Configuration.
    config: MeaningConfig,
}

impl SelfMeaningEngine {
    /// Create a new engine with all 6 default generators.
    pub fn new(config: MeaningConfig) -> Self {
        let generators: Vec<Box<dyn HypothesisGenerator>> = vec![
            Box::new(ComponentIsolationGenerator),
            Box::new(InteractionPatternGenerator),
            Box::new(ResourcePressureGenerator),
            Box::new(CodePathGenerator),
            Box::new(HistoricalPatternGenerator),
            Box::new(EnvironmentalChangeGenerator),
        ];

        Self::with_generators(config, generators)
    }

    /// Create a new engine with custom generators.
    pub fn with_generators(
        config: MeaningConfig,
        generators: Vec<Box<dyn HypothesisGenerator>>,
    ) -> Self {
        let ambiguity_manager = AmbiguityManager::from_config(&config);
        let convergence_tracker = ConvergenceTracker::from_config(&config);
        let max_history = config.max_history;

        Self {
            generators,
            evaluator: EvidenceEvaluator::default(),
            ambiguity_manager,
            convergence_tracker,
            active_meanings: Vec::new(),
            abandoned_meanings: Vec::new(),
            history: MeaningHistory::new(max_history),
            config,
        }
    }

    /// Process a batch of anomalies and update meanings.
    ///
    /// This is the main entry point — called periodically with newly detected
    /// anomalies and a current observation summary.
    ///
    /// The pipeline:
    /// 1. Run all generators → hypotheses
    /// 2. Match hypotheses to existing meanings or create new ones
    /// 3. Evaluate evidence → update confidence
    /// 4. Update convergence tracker
    /// 5. Run ambiguity manager → decisions
    /// 6. Apply decisions (archive ready meanings, abandon collapsed ones)
    pub fn process_anomalies(
        &mut self,
        anomalies: &[PerformanceAnomaly],
        summary: &ObservationSummary,
    ) {
        if anomalies.is_empty() {
            return;
        }

        // Collect history as owned Vec for generators
        let history_vec: Vec<SelfMeaning> =
            self.history.meanings.iter().cloned().collect();

        // Step 1: Generate hypotheses
        let mut all_hypotheses = Vec::new();
        for gen in &self.generators {
            let hypotheses = gen.generate(anomalies, summary, &history_vec);
            debug!(
                generator = gen.name(),
                count = hypotheses.len(),
                "Generated hypotheses"
            );
            all_hypotheses.extend(hypotheses);
        }

        // Step 2: Match hypotheses to existing meanings or create new ones
        for hypothesis in all_hypotheses {
            let evidence = Evidence {
                source: hypothesis.generator_name.clone(),
                strength: hypothesis.confidence,
                timestamp: hypothesis.created_at,
                description: hypothesis.description.clone(),
                category: EvidenceCategory::Anomaly,
            };

            if let Some(existing) = self.find_matching_meaning(&hypothesis.meaning_category) {
                // Add evidence to existing meaning
                existing.evidence.push(evidence);
            } else if self.active_meanings.len() < self.config.max_active_meanings {
                // Create new meaning
                let meaning = SelfMeaning {
                    id: MeaningId::new(),
                    category: hypothesis.meaning_category,
                    evidence: hypothesis.evidence,
                    confidence: hypothesis.confidence,
                    ambiguity: 1.0 - hypothesis.confidence,
                    formed_at: Utc::now(),
                    temporal_stability_secs: 0.0,
                    competing_with: vec![],
                    converged: false,
                };
                self.active_meanings.push(meaning);
            }
        }

        // Step 3: Update confidence via evidence evaluator
        for meaning in &mut self.active_meanings {
            if !meaning.evidence.is_empty() {
                let new_confidence = self.evaluator.evaluate_evidence_set(
                    meaning.confidence,
                    &meaning.evidence,
                );
                meaning.confidence = new_confidence;
                meaning.ambiguity = 1.0 - new_confidence;
            }
        }

        // Step 4: Update convergence tracker
        for meaning in &self.active_meanings {
            self.convergence_tracker.record(
                &meaning.id,
                meaning.confidence,
                meaning.evidence.len(),
            );
        }

        // Apply convergence state back to meanings
        for meaning in &mut self.active_meanings {
            meaning.converged = self.convergence_tracker.is_converged(&meaning.id);
        }

        // Step 5: Detect competing meanings (same component, different category)
        self.detect_competition();

        // Step 6: Run ambiguity manager
        let decisions = self
            .ambiguity_manager
            .evaluate(&self.active_meanings, &self.config);

        // Step 7: Apply decisions
        let mut to_archive: Vec<MeaningId> = Vec::new();
        let mut to_abandon: Vec<MeaningId> = Vec::new();

        for (meaning_id, decision) in &decisions {
            match decision {
                AmbiguityDecision::ReadyForIntent { .. } => {
                    to_archive.push(meaning_id.clone());
                }
                AmbiguityDecision::Preserve { .. } => {
                    // Keep active
                }
                AmbiguityDecision::Escalated { .. } => {
                    // Keep active but log escalation
                    debug!(meaning_id = %meaning_id, "Meaning escalated for safety review");
                }
                AmbiguityDecision::GatherMore { .. } => {
                    // Keep active
                }
            }
        }

        // Abandon meanings with very low confidence
        for meaning in &self.active_meanings {
            if meaning.confidence < 0.05 && meaning.evidence.len() > 5 {
                to_abandon.push(meaning.id.clone());
            }
        }

        // Archive ready-for-intent meanings
        for id in &to_archive {
            if let Some(pos) = self.active_meanings.iter().position(|m| &m.id == id) {
                let meaning = self.active_meanings.remove(pos);
                self.convergence_tracker.remove(id);
                self.history.push(meaning);
            }
        }

        // Archive abandoned meanings
        for id in &to_abandon {
            if let Some(pos) = self.active_meanings.iter().position(|m| &m.id == id) {
                let meaning = self.active_meanings.remove(pos);
                self.convergence_tracker.remove(id);
                self.abandoned_meanings.push(meaning);
                // Bound abandoned list
                if self.abandoned_meanings.len() > 100 {
                    self.abandoned_meanings.remove(0);
                }
            }
        }
    }

    /// Get all currently active meanings.
    pub fn active_meanings(&self) -> &[SelfMeaning] {
        &self.active_meanings
    }

    /// Get a specific meaning by ID.
    pub fn get_meaning(&self, id: &MeaningId) -> Option<&SelfMeaning> {
        self.active_meanings.iter().find(|m| &m.id == id)
    }

    /// Get the meaning history.
    pub fn history(&self) -> &MeaningHistory {
        &self.history
    }

    /// Get the configuration.
    pub fn config(&self) -> &MeaningConfig {
        &self.config
    }

    /// Number of active meanings.
    pub fn active_count(&self) -> usize {
        self.active_meanings.len()
    }

    /// Find an existing meaning that matches the given category.
    fn find_matching_meaning(
        &mut self,
        category: &SelfMeaningCategory,
    ) -> Option<&mut SelfMeaning> {
        let target_label = category.label();
        let target_component = category.primary_component();

        self.active_meanings.iter_mut().find(|m| {
            m.category.label() == target_label
                && m.category.primary_component() == target_component
        })
    }

    /// Detect competing meanings (same component, different interpretations).
    fn detect_competition(&mut self) {
        let ids_and_components: Vec<(MeaningId, Option<String>)> = self
            .active_meanings
            .iter()
            .map(|m| {
                (
                    m.id.clone(),
                    m.category.primary_component().map(String::from),
                )
            })
            .collect();

        for meaning in &mut self.active_meanings {
            let my_component = meaning.category.primary_component().map(String::from);
            if my_component.is_none() {
                continue;
            }

            meaning.competing_with = ids_and_components
                .iter()
                .filter(|(id, comp)| {
                    id != &meaning.id && *comp == my_component
                })
                .map(|(id, _)| id.clone())
                .collect();
        }
    }
}

impl MeaningIntentBridge for SelfMeaningEngine {
    fn ready_for_intent(&self) -> Vec<&SelfMeaning> {
        self.active_meanings
            .iter()
            .filter(|m| {
                m.converged
                    && m.confidence > (1.0 - self.config.resolution_threshold)
                    && m.competing_with.is_empty()
            })
            .collect()
    }

    fn still_forming(&self) -> Vec<&SelfMeaning> {
        self.active_meanings
            .iter()
            .filter(|m| {
                !(m.converged
                    && m.confidence > (1.0 - self.config.resolution_threshold)
                    && m.competing_with.is_empty())
            })
            .collect()
    }

    fn abandoned(&self) -> Vec<&SelfMeaning> {
        self.abandoned_meanings.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_observation::{
        AnomalyCategory, AnomalyId, AnomalySeverity, ComponentId, MetricId,
    };
    use std::collections::HashMap;

    fn make_anomaly(
        component: &str,
        category: AnomalyCategory,
        severity: AnomalySeverity,
        score: f64,
    ) -> PerformanceAnomaly {
        PerformanceAnomaly {
            id: AnomalyId::new(),
            category,
            severity,
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
    fn engine_creates_with_default_generators() {
        let engine = SelfMeaningEngine::new(MeaningConfig::default());
        assert_eq!(engine.generators.len(), 6);
        assert_eq!(engine.active_count(), 0);
    }

    #[test]
    fn engine_processes_anomalies_creates_meanings() {
        let mut engine = SelfMeaningEngine::new(MeaningConfig::default());

        let anomalies = vec![
            make_anomaly(
                "event-fabric",
                AnomalyCategory::LatencyRegression,
                AnomalySeverity::Warning,
                0.8,
            ),
        ];

        engine.process_anomalies(&anomalies, &empty_summary());

        assert!(
            engine.active_count() > 0,
            "Should create at least one meaning from anomaly"
        );
    }

    #[test]
    fn engine_accumulates_evidence_on_repeated_anomalies() {
        let mut engine = SelfMeaningEngine::new(MeaningConfig::default());

        let anomaly = make_anomaly(
            "gate",
            AnomalyCategory::LatencyRegression,
            AnomalySeverity::Warning,
            0.7,
        );

        engine.process_anomalies(&[anomaly.clone()], &empty_summary());
        let initial_count = engine.active_count();

        // Process same type of anomaly again
        let anomaly2 = make_anomaly(
            "gate",
            AnomalyCategory::LatencyRegression,
            AnomalySeverity::Warning,
            0.75,
        );
        engine.process_anomalies(&[anomaly2], &empty_summary());

        // Should NOT create a new meaning — should add evidence to existing
        assert_eq!(engine.active_count(), initial_count);

        // Evidence should have accumulated
        let meaning = &engine.active_meanings()[0];
        assert!(
            meaning.evidence.len() > 1,
            "Evidence should accumulate: got {}",
            meaning.evidence.len()
        );
    }

    #[test]
    fn engine_bounds_active_meanings() {
        let config = MeaningConfig {
            max_active_meanings: 3,
            ..MeaningConfig::default()
        };
        let mut engine = SelfMeaningEngine::new(config);

        // Create anomalies for different components
        for i in 0..10 {
            let anomaly = make_anomaly(
                &format!("component-{}", i),
                AnomalyCategory::LatencyRegression,
                AnomalySeverity::Warning,
                0.7,
            );
            engine.process_anomalies(&[anomaly], &empty_summary());
        }

        assert!(
            engine.active_count() <= 3,
            "Should be bounded: got {}",
            engine.active_count()
        );
    }

    #[test]
    fn engine_detects_competition() {
        let mut engine = SelfMeaningEngine::new(MeaningConfig::default());

        // Create a latency regression anomaly
        let anomaly1 = make_anomaly(
            "gate",
            AnomalyCategory::LatencyRegression,
            AnomalySeverity::Warning,
            0.7,
        );
        // Create an operator bottleneck for same component
        let anomaly2 = make_anomaly(
            "gate",
            AnomalyCategory::OperatorBottleneck,
            AnomalySeverity::Warning,
            0.6,
        );

        engine.process_anomalies(&[anomaly1, anomaly2], &empty_summary());

        // Check if competition is detected
        let with_competitors = engine
            .active_meanings()
            .iter()
            .filter(|m| !m.competing_with.is_empty())
            .count();

        // Both meanings about "gate" should detect each other as competitors
        assert!(
            with_competitors > 0,
            "Should detect competing meanings for same component"
        );
    }

    #[test]
    fn engine_abandons_low_confidence() {
        let mut engine = SelfMeaningEngine::new(MeaningConfig::default());

        // Manually inject a meaning with very low confidence
        let weak_meaning = SelfMeaning {
            id: MeaningId::new(),
            category: SelfMeaningCategory::ApiDesignInsight {
                pattern: "test".into(),
                improvement_direction: "test".into(),
            },
            evidence: (0..10)
                .map(|i| Evidence {
                    source: "test".into(),
                    strength: 0.01,
                    timestamp: Utc::now(),
                    description: format!("weak evidence {}", i),
                    category: EvidenceCategory::Absence,
                })
                .collect(),
            confidence: 0.02,
            ambiguity: 0.98,
            formed_at: Utc::now(),
            temporal_stability_secs: 0.0,
            competing_with: vec![],
            converged: false,
        };
        engine.active_meanings.push(weak_meaning);

        // Process to trigger cleanup
        engine.process_anomalies(
            &[make_anomaly(
                "other",
                AnomalyCategory::LatencyRegression,
                AnomalySeverity::Info,
                0.5,
            )],
            &empty_summary(),
        );

        // The weak meaning should have been abandoned
        assert!(
            engine.abandoned().len() > 0,
            "Low confidence meaning should be abandoned"
        );
    }

    #[test]
    fn engine_history_grows_on_archive() {
        let mut engine = SelfMeaningEngine::new(MeaningConfig::default());
        assert!(engine.history().is_empty());

        // Manually push a converged meaning to history
        let meaning = SelfMeaning {
            id: MeaningId::new(),
            category: SelfMeaningCategory::PerformanceBottleneck {
                component: "test".into(),
                severity: 0.8,
                root_causes: vec![],
            },
            evidence: vec![],
            confidence: 0.9,
            ambiguity: 0.1,
            formed_at: Utc::now(),
            temporal_stability_secs: 7200.0,
            competing_with: vec![],
            converged: true,
        };
        engine.history.push(meaning);

        assert_eq!(engine.history().len(), 1);
    }

    #[test]
    fn engine_empty_anomalies_noop() {
        let mut engine = SelfMeaningEngine::new(MeaningConfig::default());
        engine.process_anomalies(&[], &empty_summary());
        assert_eq!(engine.active_count(), 0);
    }

    #[test]
    fn bridge_still_forming_returns_non_converged() {
        let mut engine = SelfMeaningEngine::new(MeaningConfig::default());

        let anomalies = vec![make_anomaly(
            "fabric",
            AnomalyCategory::ThroughputDegradation,
            AnomalySeverity::Warning,
            0.7,
        )];
        engine.process_anomalies(&anomalies, &empty_summary());

        // Newly created meanings should be "still forming"
        let forming = engine.still_forming();
        assert!(
            !forming.is_empty(),
            "Newly created meanings should be still forming"
        );
    }

    #[test]
    fn meaning_history_bounded() {
        let mut history = MeaningHistory::new(3);
        for i in 0..10 {
            history.push(SelfMeaning {
                id: MeaningId::new(),
                category: SelfMeaningCategory::PerformanceBottleneck {
                    component: format!("comp-{}", i),
                    severity: 0.5,
                    root_causes: vec![],
                },
                evidence: vec![],
                confidence: 0.5,
                ambiguity: 0.5,
                formed_at: Utc::now(),
                temporal_stability_secs: 0.0,
                competing_with: vec![],
                converged: false,
            });
        }
        assert_eq!(history.len(), 3, "History should be bounded to 3");
    }
}
