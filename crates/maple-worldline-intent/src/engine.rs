//! Intent stabilization engine — orchestrates the full intent pipeline.
//!
//! The engine connects converged meanings to the commitment boundary.
//! It generates intents from meanings, validates them, prioritizes them,
//! evaluates deferral conditions, and manages the intent lifecycle.
//!
//! ## Pipeline
//!
//! ```text
//! converged meanings
//!   → intent generation (generators)
//!   → validation (validator)
//!   → prioritization (prioritizer)
//!   → deferral evaluation (deferral manager)
//!   → stabilized intents (ready for commitment)
//! ```

use std::collections::VecDeque;

use chrono::{DateTime, Utc};

use maple_worldline_meaning::bridge::MeaningIntentBridge;
use maple_worldline_meaning::types::SelfMeaning;

use crate::deferral::{DeferralDecision, DeferralManager};
use crate::error::IntentResult;
use crate::generator::{self, IntentGenerator};
use crate::intent::{IntentStatus, SelfRegenerationIntent};
use crate::prioritizer::IntentPrioritizer;
use crate::types::IntentConfig;
use crate::validator::{IntentValidationResult, IntentValidator};

// ── Engine ─────────────────────────────────────────────────────────────

/// The intent stabilization engine.
///
/// Manages the full lifecycle of self-regeneration intents from
/// formation through validation, prioritization, and deferral.
pub struct IntentStabilizationEngine {
    /// Intent generators (one per meaning category).
    generators: Vec<Box<dyn IntentGenerator>>,
    /// Validator for quality/safety checks.
    validator: IntentValidator,
    /// Prioritizer for ranking intents.
    prioritizer: IntentPrioritizer,
    /// Deferral manager.
    deferral: DeferralManager,
    /// Active intents (validated, not deferred).
    active_intents: Vec<SelfRegenerationIntent>,
    /// Deferred intents (waiting for conditions to improve).
    deferred_intents: Vec<SelfRegenerationIntent>,
    /// History of completed/abandoned intents (bounded).
    history: VecDeque<SelfRegenerationIntent>,
    /// Last modification timestamp (for cooldown tracking).
    last_modification: Option<DateTime<Utc>>,
    /// Configuration.
    config: IntentConfig,
}

impl IntentStabilizationEngine {
    /// Create a new engine with the given configuration.
    pub fn new(config: IntentConfig) -> Self {
        let validator = IntentValidator::from_config(&config);
        let prioritizer = IntentPrioritizer::from_config(&config);
        let deferral = DeferralManager::from_config(&config);

        Self {
            generators: generator::default_generators(),
            validator,
            prioritizer,
            deferral,
            active_intents: Vec::new(),
            deferred_intents: Vec::new(),
            history: VecDeque::with_capacity(256),
            last_modification: None,
            config,
        }
    }

    /// Create a new engine with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(IntentConfig::default())
    }

    /// Process converged meanings from a meaning-intent bridge.
    ///
    /// This is the main entry point. It pulls ready meanings from the
    /// bridge and runs them through the full intent pipeline.
    pub fn process_meanings(
        &mut self,
        bridge: &dyn MeaningIntentBridge,
        system_load: f64,
    ) -> IntentResult<Vec<SelfRegenerationIntent>> {
        let ready_refs = bridge.ready_for_intent();
        let ready_meanings: Vec<SelfMeaning> =
            ready_refs.into_iter().cloned().collect();
        self.process_meaning_list(&ready_meanings, system_load)
    }

    /// Process a list of converged meanings through the pipeline.
    pub fn process_meaning_list(
        &mut self,
        meanings: &[SelfMeaning],
        system_load: f64,
    ) -> IntentResult<Vec<SelfRegenerationIntent>> {
        let mut newly_stabilized = Vec::new();

        for meaning in meanings {
            // Step 1: Generate intents from this meaning
            let generated = self.generate_intents(meaning);

            for mut intent in generated {
                // Step 2: Validate
                let validation = self.validator.validate(&intent);
                match validation {
                    IntentValidationResult::Valid => {
                        intent.status = IntentStatus::Validated;
                    }
                    IntentValidationResult::Invalid { ref issues } => {
                        let has_errors = issues
                            .iter()
                            .any(|i| i.severity == crate::validator::IssueSeverity::Error);
                        if has_errors {
                            let reasons: Vec<String> =
                                issues.iter().map(|i| i.to_string()).collect();
                            intent.status = IntentStatus::Abandoned(reasons.join("; "));
                            self.push_history(intent);
                            continue;
                        }
                        // Warnings only — still valid
                        intent.status = IntentStatus::Validated;
                    }
                }

                // Step 3: Deferral check
                let decision = self.deferral.evaluate(
                    &intent,
                    self.active_intents.len(),
                    self.last_modification,
                    system_load,
                );
                match decision {
                    DeferralDecision::Proceed => {
                        intent.status = IntentStatus::Stabilized;
                        newly_stabilized.push(intent.clone());
                        self.add_active(intent);
                    }
                    DeferralDecision::Defer(reason) => {
                        intent.status = IntentStatus::Deferred(reason.to_string());
                        self.add_deferred(intent);
                    }
                }
            }
        }

        // Step 4: Re-prioritize active intents
        self.prioritizer.prioritize(&mut self.active_intents);

        Ok(newly_stabilized)
    }

    /// Generate intents from a single meaning using all generators.
    fn generate_intents(&self, meaning: &SelfMeaning) -> Vec<SelfRegenerationIntent> {
        let mut intents = Vec::new();
        for gen in &self.generators {
            if let Some(intent) = gen.generate(meaning) {
                intents.push(intent);
            }
        }
        intents
    }

    /// Add an intent to the active set (bounded).
    fn add_active(&mut self, intent: SelfRegenerationIntent) {
        if self.active_intents.len() >= self.config.max_active_intents {
            // Remove the lowest-priority intent
            if let Some(removed) = self.active_intents.pop() {
                self.push_history(removed);
            }
        }
        self.active_intents.push(intent);
    }

    /// Add an intent to the deferred set (bounded).
    fn add_deferred(&mut self, intent: SelfRegenerationIntent) {
        if self.deferred_intents.len() >= self.config.max_deferred_intents {
            // Remove oldest deferred intent
            if let Some(removed) = self.deferred_intents.first().cloned() {
                self.push_history(removed);
                self.deferred_intents.remove(0);
            }
        }
        self.deferred_intents.push(intent);
    }

    /// Add an intent to history (bounded).
    fn push_history(&mut self, intent: SelfRegenerationIntent) {
        if self.history.len() >= 256 {
            self.history.pop_front();
        }
        self.history.push_back(intent);
    }

    /// Record that a modification was applied (resets cooldown timer).
    pub fn record_modification(&mut self) {
        self.last_modification = Some(Utc::now());
    }

    /// Re-evaluate deferred intents under current conditions.
    ///
    /// Moves intents from deferred → active if conditions have improved.
    pub fn reevaluate_deferred(&mut self, system_load: f64) -> Vec<SelfRegenerationIntent> {
        let mut promoted = Vec::new();
        let mut still_deferred = Vec::new();

        for mut intent in self.deferred_intents.drain(..) {
            let decision = self.deferral.evaluate(
                &intent,
                self.active_intents.len() + promoted.len(),
                self.last_modification,
                system_load,
            );
            match decision {
                DeferralDecision::Proceed => {
                    intent.status = IntentStatus::Stabilized;
                    promoted.push(intent);
                }
                DeferralDecision::Defer(reason) => {
                    intent.status = IntentStatus::Deferred(reason.to_string());
                    still_deferred.push(intent);
                }
            }
        }

        self.deferred_intents = still_deferred;

        for intent in &promoted {
            self.active_intents.push(intent.clone());
        }

        self.prioritizer.prioritize(&mut self.active_intents);

        promoted
    }

    /// Get current active intents (sorted by priority).
    pub fn active_intents(&self) -> &[SelfRegenerationIntent] {
        &self.active_intents
    }

    /// Get current deferred intents.
    pub fn deferred_intents(&self) -> &[SelfRegenerationIntent] {
        &self.deferred_intents
    }

    /// Get intent history.
    pub fn history(&self) -> &VecDeque<SelfRegenerationIntent> {
        &self.history
    }

    /// Get the engine configuration.
    pub fn config(&self) -> &IntentConfig {
        &self.config
    }

    /// Total intents tracked (active + deferred + history).
    pub fn total_tracked(&self) -> usize {
        self.active_intents.len() + self.deferred_intents.len() + self.history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_meaning::types::{
        Evidence, EvidenceCategory, MeaningId, SelfMeaning, SelfMeaningCategory,
    };

    fn make_evidence(n: usize) -> Vec<Evidence> {
        (0..n)
            .map(|i| Evidence {
                source: format!("source-{}", i),
                strength: 0.7,
                timestamp: Utc::now(),
                description: format!("evidence {}", i),
                category: EvidenceCategory::Anomaly,
            })
            .collect()
    }

    fn make_perf_meaning(confidence: f64) -> SelfMeaning {
        SelfMeaning {
            id: MeaningId::new(),
            category: SelfMeaningCategory::PerformanceBottleneck {
                component: "gate".into(),
                severity: 0.8,
                root_causes: vec![],
            },
            evidence: make_evidence(15),
            confidence,
            ambiguity: 0.1,
            formed_at: Utc::now(),
            temporal_stability_secs: 7200.0,
            competing_with: vec![],
            converged: true,
        }
    }

    #[test]
    fn engine_creates_with_defaults() {
        let engine = IntentStabilizationEngine::with_defaults();
        assert!(engine.active_intents().is_empty());
        assert!(engine.deferred_intents().is_empty());
        assert_eq!(engine.total_tracked(), 0);
    }

    #[test]
    fn engine_processes_performance_meaning() {
        let mut engine = IntentStabilizationEngine::with_defaults();
        let meaning = make_perf_meaning(0.9);
        let result = engine
            .process_meaning_list(&[meaning], 0.3)
            .expect("should succeed");
        // Performance bottleneck → PerformanceIntentGenerator produces intent
        assert!(!result.is_empty());
        assert!(!engine.active_intents().is_empty());
    }

    #[test]
    fn engine_defers_when_overloaded() {
        let mut engine = IntentStabilizationEngine::with_defaults();
        let meaning = make_perf_meaning(0.9);
        let result = engine
            .process_meaning_list(&[meaning], 0.95) // high load
            .expect("should succeed");
        assert!(result.is_empty(), "should defer under high load");
        assert!(!engine.deferred_intents().is_empty());
    }

    #[test]
    fn engine_rejects_low_confidence() {
        let mut engine = IntentStabilizationEngine::with_defaults();
        let meaning = make_perf_meaning(0.3); // too low
        let result = engine
            .process_meaning_list(&[meaning], 0.3)
            .expect("should succeed");
        assert!(result.is_empty());
        // Should be abandoned (in history), not active or deferred
        assert!(engine.active_intents().is_empty());
    }

    #[test]
    fn engine_reevaluates_deferred() {
        let mut engine = IntentStabilizationEngine::with_defaults();
        let meaning = make_perf_meaning(0.9);

        // First pass: defer due to load
        engine
            .process_meaning_list(&[meaning], 0.95)
            .expect("should succeed");
        assert_eq!(engine.deferred_intents().len(), 1);
        assert!(engine.active_intents().is_empty());

        // Second pass: conditions improve
        let promoted = engine.reevaluate_deferred(0.3);
        assert!(!promoted.is_empty());
        assert!(engine.deferred_intents().is_empty());
        assert!(!engine.active_intents().is_empty());
    }

    #[test]
    fn engine_records_modification() {
        let mut engine = IntentStabilizationEngine::with_defaults();
        assert!(engine.last_modification.is_none());
        engine.record_modification();
        assert!(engine.last_modification.is_some());
    }

    #[test]
    fn engine_respects_cooldown() {
        let mut engine = IntentStabilizationEngine::with_defaults();
        engine.record_modification(); // just modified → cooldown active
        let meaning = make_perf_meaning(0.9);
        let result = engine
            .process_meaning_list(&[meaning], 0.3)
            .expect("should succeed");
        assert!(result.is_empty(), "should defer during cooldown");
        assert!(!engine.deferred_intents().is_empty());
    }

    #[test]
    fn engine_ignores_irrelevant_meanings() {
        let mut engine = IntentStabilizationEngine::with_defaults();
        let meaning = SelfMeaning {
            id: MeaningId::new(),
            category: SelfMeaningCategory::ApiDesignInsight {
                pattern: "rest".into(),
                improvement_direction: "versioning".into(),
            },
            evidence: make_evidence(15),
            confidence: 0.9,
            ambiguity: 0.1,
            formed_at: Utc::now(),
            temporal_stability_secs: 7200.0,
            competing_with: vec![],
            converged: true,
        };
        let result = engine
            .process_meaning_list(&[meaning], 0.3)
            .expect("should succeed");
        // ApiDesignInsight has no generator → no intents
        assert!(result.is_empty());
        assert!(engine.active_intents().is_empty());
    }

    #[test]
    fn engine_processes_multiple_meanings() {
        let mut engine = IntentStabilizationEngine::with_defaults();
        let meanings = vec![
            make_perf_meaning(0.9),
            make_perf_meaning(0.85),
        ];
        let result = engine
            .process_meaning_list(&meanings, 0.3)
            .expect("should succeed");
        assert_eq!(result.len(), 2);
        assert_eq!(engine.active_intents().len(), 2);
    }

    #[test]
    fn engine_bounds_active_intents() {
        let config = IntentConfig {
            max_active_intents: 2,
            ..IntentConfig::default()
        };
        let mut engine = IntentStabilizationEngine::new(config);
        let meanings: Vec<_> = (0..5).map(|_| make_perf_meaning(0.9)).collect();
        engine
            .process_meaning_list(&meanings, 0.3)
            .expect("should succeed");
        // At most max_active_intents remain active
        assert!(engine.active_intents().len() <= 2);
    }
}
