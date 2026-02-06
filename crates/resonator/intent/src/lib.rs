//! Intent Stabilization Engine for MAPLE Resonators
//!
//! This module implements intent stabilization for the Resonance Architecture.
//! Intent represents stabilized goals formed from sufficient meaning
//! (Invariant #2: Meaning precedes Intent) and must stabilize before
//! commitments can be made (Invariant #3: Intent precedes Commitment).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                  INTENT STABILIZATION ENGINE                    │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │   Meaning ──> IntentStabilizationEngine                        │
//! │                        │                                       │
//! │                        v                                       │
//! │              TemporalStabilityChecker                          │
//! │                        │                                       │
//! │                        v                                       │
//! │              ConsistencyValidator                              │
//! │                        │                                       │
//! │                        v                                       │
//! │                 IntentHistory                                  │
//! │                        │                                       │
//! │                        v                                       │
//! │               StabilizedIntent                                 │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Components
//!
//! - [`IntentStabilizationEngine`]: Main engine that orchestrates intent stabilization
//! - [`TemporalStabilityChecker`]: Ensures intent remains stable over time
//! - [`ConsistencyValidator`]: Checks intent consistency with history
//! - [`IntentHistory`]: Ring buffer of recent intents for comparison
//!
//! # Invariant Enforcement
//!
//! - Invariant #2: Meaning must be present before intent can form
//! - Invariant #3: Intent must stabilize before commitment can be created

#![deny(unsafe_code)]

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use rcf_intent::{Goal, RcfIntent};
use rcf_meaning::RcfMeaning;
use rcf_types::IdentityRef;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Unique identifier for an intent instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntentId(pub String);

impl IntentId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for IntentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for IntentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Configuration for the intent stabilization engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentStabilizationConfig {
    /// Minimum confidence required for intent stabilization.
    pub min_confidence: f64,
    /// Temporal stability window in milliseconds.
    /// Intent must remain consistent for this duration.
    pub stability_window_ms: u64,
    /// Maximum number of intents to keep in history.
    pub history_depth: usize,
    /// Consistency threshold (0.0 to 1.0).
    /// Higher values require more consistency with history.
    pub consistency_threshold: f64,
    /// Minimum meaning confidence required before intent can form.
    pub min_meaning_confidence: f64,
}

impl Default for IntentStabilizationConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.65,
            stability_window_ms: 1000, // 1 second
            history_depth: 50,
            consistency_threshold: 0.7,
            min_meaning_confidence: 0.5,
        }
    }
}

/// Result of intent stabilization attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StabilizationResult {
    /// Intent has stabilized and is ready for commitment.
    Stabilized {
        intent: StabilizedIntent,
        confidence: f64,
    },
    /// Intent is not yet stable.
    Unstable {
        reason: UnstableReason,
        progress: f64,
        current_intent: IntentCandidate,
    },
    /// Intent contradicts recent history.
    Contradictory {
        conflicting_intents: Vec<IntentId>,
        suggestion: String,
    },
}

/// Reasons why intent may be unstable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnstableReason {
    /// Confidence below threshold.
    InsufficientConfidence { actual: f64, required: f64 },
    /// Intent changed too recently.
    TemporalInstability { remaining_ms: u64 },
    /// Inconsistent with recent history.
    HistoryInconsistency { consistency_score: f64 },
    /// Meaning not yet sufficient.
    InsufficientMeaning { confidence: f64 },
    /// No goals defined.
    NoGoals,
}

/// A candidate intent that may or may not be stabilized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentCandidate {
    /// Unique identifier.
    pub id: IntentId,
    /// The objective/goal description.
    pub objective: String,
    /// Detailed steps if available.
    pub steps: Vec<String>,
    /// Current confidence level.
    pub confidence: f64,
    /// Reference to source meaning.
    pub meaning_ref: Option<String>,
    /// When this candidate was created.
    pub created_at: DateTime<Utc>,
    /// Whether blocking ambiguity exists.
    pub blocking_ambiguity: bool,
}

impl IntentCandidate {
    /// Create a new intent candidate.
    pub fn new(objective: impl Into<String>, confidence: f64) -> Self {
        Self {
            id: IntentId::new(),
            objective: objective.into(),
            steps: Vec::new(),
            confidence,
            meaning_ref: None,
            created_at: Utc::now(),
            blocking_ambiguity: false,
        }
    }

    /// Create from RCF meaning.
    pub fn from_meaning(meaning: &RcfMeaning, objective: impl Into<String>) -> Self {
        Self {
            id: IntentId::new(),
            objective: objective.into(),
            steps: Vec::new(),
            confidence: meaning.uncertainty.confidence,
            meaning_ref: Some(meaning.id.clone()),
            created_at: Utc::now(),
            blocking_ambiguity: meaning.uncertainty.confidence < 0.5,
        }
    }

    /// Add a step to the intent.
    pub fn with_step(mut self, step: impl Into<String>) -> Self {
        self.steps.push(step.into());
        self
    }

    /// Set blocking ambiguity flag.
    pub fn with_blocking_ambiguity(mut self, blocking: bool) -> Self {
        self.blocking_ambiguity = blocking;
        self
    }
}

/// A fully stabilized intent ready for commitment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilizedIntent {
    /// The underlying RCF intent.
    pub intent: RcfIntent,
    /// When stabilization was achieved.
    pub stabilized_at: DateTime<Utc>,
    /// How long the intent remained stable.
    pub stability_duration_ms: u64,
    /// Consistency score with history.
    pub consistency_score: f64,
    /// Source meaning references.
    pub meaning_refs: Vec<String>,
}

impl StabilizedIntent {
    /// Get the intent ID.
    pub fn id(&self) -> &str {
        &self.intent.id
    }

    /// Get the confidence level.
    pub fn confidence(&self) -> f64 {
        self.intent.confidence
    }

    /// Check if sufficient for commitment.
    pub fn is_sufficient_for_commitment(&self) -> bool {
        self.intent.is_sufficient_for_commitment()
    }

    /// Get the primary goal description.
    pub fn primary_goal(&self) -> Option<&str> {
        self.intent.goals.first().map(|g| g.description.as_str())
    }
}

/// History record for an intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentHistoryRecord {
    pub id: IntentId,
    pub objective: String,
    pub confidence: f64,
    pub timestamp: DateTime<Utc>,
    pub was_stabilized: bool,
    pub meaning_ref: Option<String>,
}

/// Ring buffer of recent intents for consistency checking.
#[derive(Debug, Clone)]
pub struct IntentHistory {
    /// Recent intents.
    records: VecDeque<IntentHistoryRecord>,
    /// Maximum history size.
    max_size: usize,
}

impl IntentHistory {
    /// Create a new intent history.
    pub fn new(max_size: usize) -> Self {
        Self {
            records: VecDeque::new(),
            max_size,
        }
    }

    /// Add a record to history.
    pub fn add(&mut self, record: IntentHistoryRecord) {
        self.records.push_back(record);
        while self.records.len() > self.max_size {
            self.records.pop_front();
        }
    }

    /// Get recent records.
    pub fn recent(&self, limit: usize) -> Vec<&IntentHistoryRecord> {
        self.records.iter().rev().take(limit).collect()
    }

    /// Get all records.
    pub fn all(&self) -> &VecDeque<IntentHistoryRecord> {
        &self.records
    }

    /// Query by time range.
    pub fn in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&IntentHistoryRecord> {
        self.records
            .iter()
            .filter(|r| r.timestamp >= start && r.timestamp <= end)
            .collect()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Get count.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Clear history.
    pub fn clear(&mut self) {
        self.records.clear();
    }
}

impl Default for IntentHistory {
    fn default() -> Self {
        Self::new(50)
    }
}

/// Checks temporal stability of intent.
#[derive(Debug, Clone)]
pub struct TemporalStabilityChecker {
    /// Required stability window.
    stability_window_ms: u64,
    /// When current intent was first seen.
    first_seen: Option<DateTime<Utc>>,
    /// Current intent objective (for change detection).
    current_objective: Option<String>,
    /// Last significant change timestamp.
    last_change: Option<DateTime<Utc>>,
}

impl TemporalStabilityChecker {
    /// Create a new temporal stability checker.
    pub fn new(stability_window_ms: u64) -> Self {
        Self {
            stability_window_ms,
            first_seen: None,
            current_objective: None,
            last_change: None,
        }
    }

    /// Update with a new intent candidate.
    pub fn update(&mut self, candidate: &IntentCandidate) {
        let now = Utc::now();

        match &self.current_objective {
            Some(current) if self.objectives_similar(current, &candidate.objective) => {
                // Same intent, no change needed
            }
            _ => {
                // New or changed intent
                self.first_seen = Some(now);
                self.current_objective = Some(candidate.objective.clone());
                self.last_change = Some(now);
            }
        }
    }

    /// Check if intent is temporally stable.
    pub fn check(&self) -> TemporalStabilityResult {
        let now = Utc::now();

        let Some(first_seen) = self.first_seen else {
            return TemporalStabilityResult::NoIntent;
        };

        let elapsed_ms = (now - first_seen).num_milliseconds() as u64;

        if elapsed_ms >= self.stability_window_ms {
            TemporalStabilityResult::Stable {
                duration_ms: elapsed_ms,
            }
        } else {
            TemporalStabilityResult::Unstable {
                elapsed_ms,
                required_ms: self.stability_window_ms,
                remaining_ms: self.stability_window_ms - elapsed_ms,
            }
        }
    }

    /// Reset the checker.
    pub fn reset(&mut self) {
        self.first_seen = None;
        self.current_objective = None;
        self.last_change = None;
    }

    fn objectives_similar(&self, a: &str, b: &str) -> bool {
        // Simple similarity check - in production, use semantic similarity
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();

        // Exact match
        if a_lower == b_lower {
            return true;
        }

        // Check if one contains the other (for minor variations)
        if a_lower.contains(&b_lower) || b_lower.contains(&a_lower) {
            return true;
        }

        // Word overlap check
        let a_words: std::collections::HashSet<_> = a_lower.split_whitespace().collect();
        let b_words: std::collections::HashSet<_> = b_lower.split_whitespace().collect();

        if a_words.is_empty() || b_words.is_empty() {
            return false;
        }

        let intersection = a_words.intersection(&b_words).count();
        let union = a_words.union(&b_words).count();

        // Jaccard similarity > 0.5
        (intersection as f64 / union as f64) > 0.5
    }
}

/// Result of temporal stability check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemporalStabilityResult {
    /// Intent is stable.
    Stable { duration_ms: u64 },
    /// Intent is not yet stable.
    Unstable {
        elapsed_ms: u64,
        required_ms: u64,
        remaining_ms: u64,
    },
    /// No intent to check.
    NoIntent,
}

/// Validates consistency of intent with history.
#[derive(Debug, Clone)]
pub struct ConsistencyValidator {
    /// Consistency threshold.
    threshold: f64,
}

impl ConsistencyValidator {
    /// Create a new consistency validator.
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    /// Validate intent against history.
    pub fn validate(
        &self,
        candidate: &IntentCandidate,
        history: &IntentHistory,
    ) -> ConsistencyResult {
        if history.is_empty() {
            return ConsistencyResult::Consistent {
                score: 1.0,
                reason: "No history to compare against".to_string(),
            };
        }

        let recent = history.recent(10);

        // Check for contradictions
        let mut contradictions = Vec::new();
        for record in &recent {
            if self.are_contradictory(&candidate.objective, &record.objective) {
                contradictions.push(record.id.clone());
            }
        }

        if !contradictions.is_empty() {
            return ConsistencyResult::Contradictory {
                conflicting_ids: contradictions,
                suggestion: "Recent intents appear to contradict the current intent. Consider clarifying.".to_string(),
            };
        }

        // Compute consistency score
        let mut total_similarity = 0.0;
        let mut count = 0;

        for record in &recent {
            let similarity = self.compute_similarity(&candidate.objective, &record.objective);
            total_similarity += similarity;
            count += 1;
        }

        let avg_similarity = if count > 0 {
            total_similarity / count as f64
        } else {
            1.0
        };

        // Consistency is high when either:
        // 1. Intent is similar to history (continuation)
        // 2. Intent is clearly different (fresh start)
        let consistency_score = if avg_similarity > 0.7 {
            // Similar to history - good consistency
            avg_similarity
        } else if avg_similarity < 0.3 {
            // Clearly different - acceptable fresh start
            0.8
        } else {
            // Ambiguous middle ground - lower consistency
            avg_similarity
        };

        if consistency_score >= self.threshold {
            ConsistencyResult::Consistent {
                score: consistency_score,
                reason: "Intent is consistent with recent history".to_string(),
            }
        } else {
            ConsistencyResult::Inconsistent {
                score: consistency_score,
                threshold: self.threshold,
                reason: "Intent shows moderate but unclear similarity to recent history".to_string(),
            }
        }
    }

    fn are_contradictory(&self, a: &str, b: &str) -> bool {
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();

        // Check for negation patterns
        let negation_pairs = [
            ("should", "should not"),
            ("will", "will not"),
            ("can", "cannot"),
            ("do", "do not"),
            ("allow", "deny"),
            ("accept", "reject"),
            ("approve", "reject"),
            ("enable", "disable"),
            ("start", "stop"),
            ("create", "delete"),
        ];

        for (positive, negative) in negation_pairs {
            if (a_lower.contains(positive) && b_lower.contains(negative))
                || (a_lower.contains(negative) && b_lower.contains(positive))
            {
                return true;
            }
        }

        false
    }

    fn compute_similarity(&self, a: &str, b: &str) -> f64 {
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();

        if a_lower == b_lower {
            return 1.0;
        }

        let a_words: std::collections::HashSet<_> = a_lower.split_whitespace().collect();
        let b_words: std::collections::HashSet<_> = b_lower.split_whitespace().collect();

        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }

        let intersection = a_words.intersection(&b_words).count();
        let union = a_words.union(&b_words).count();

        intersection as f64 / union as f64
    }
}

/// Result of consistency validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsistencyResult {
    /// Intent is consistent.
    Consistent { score: f64, reason: String },
    /// Intent is inconsistent.
    Inconsistent {
        score: f64,
        threshold: f64,
        reason: String,
    },
    /// Intent contradicts history.
    Contradictory {
        conflicting_ids: Vec<IntentId>,
        suggestion: String,
    },
}

/// The main intent stabilization engine.
///
/// This engine orchestrates intent stabilization by checking:
/// 1. Meaning sufficiency (Invariant #2)
/// 2. Confidence threshold
/// 3. Temporal stability
/// 4. Consistency with history
#[derive(Debug, Clone)]
pub struct IntentStabilizationEngine {
    /// Configuration.
    config: IntentStabilizationConfig,
    /// Intent history.
    history: IntentHistory,
    /// Temporal stability checker.
    temporal: TemporalStabilityChecker,
    /// Consistency validator.
    consistency: ConsistencyValidator,
    /// Identity of owning Resonator.
    identity: Option<IdentityRef>,
    /// Current candidate.
    current_candidate: Option<IntentCandidate>,
}

impl IntentStabilizationEngine {
    /// Create a new intent stabilization engine with default configuration.
    pub fn new() -> Self {
        Self::with_config(IntentStabilizationConfig::default())
    }

    /// Create a new engine with custom configuration.
    pub fn with_config(config: IntentStabilizationConfig) -> Self {
        Self {
            history: IntentHistory::new(config.history_depth),
            temporal: TemporalStabilityChecker::new(config.stability_window_ms),
            consistency: ConsistencyValidator::new(config.consistency_threshold),
            config,
            identity: None,
            current_candidate: None,
        }
    }

    /// Set the identity of the owning Resonator.
    pub fn set_identity(&mut self, identity: IdentityRef) {
        self.identity = Some(identity);
    }

    /// Attempt to stabilize an intent from meaning.
    ///
    /// This enforces Invariant #2 (Meaning precedes Intent) by requiring
    /// sufficient meaning confidence before intent can form.
    pub fn stabilize_from_meaning(
        &mut self,
        meaning: &RcfMeaning,
        objective: impl Into<String>,
    ) -> StabilizationResult {
        // Check Invariant #2: Meaning must be sufficient
        if meaning.uncertainty.confidence < self.config.min_meaning_confidence {
            return StabilizationResult::Unstable {
                reason: UnstableReason::InsufficientMeaning {
                    confidence: meaning.uncertainty.confidence,
                },
                progress: meaning.uncertainty.confidence / self.config.min_meaning_confidence,
                current_intent: IntentCandidate::from_meaning(meaning, objective),
            };
        }

        let candidate = IntentCandidate::from_meaning(meaning, objective);
        self.stabilize(candidate)
    }

    /// Attempt to stabilize an intent candidate.
    pub fn stabilize(&mut self, candidate: IntentCandidate) -> StabilizationResult {
        // Check for blocking ambiguity
        if candidate.blocking_ambiguity {
            return StabilizationResult::Unstable {
                reason: UnstableReason::InsufficientConfidence {
                    actual: candidate.confidence,
                    required: self.config.min_confidence,
                },
                progress: candidate.confidence / self.config.min_confidence,
                current_intent: candidate,
            };
        }

        // Check confidence threshold
        if candidate.confidence < self.config.min_confidence {
            return StabilizationResult::Unstable {
                reason: UnstableReason::InsufficientConfidence {
                    actual: candidate.confidence,
                    required: self.config.min_confidence,
                },
                progress: candidate.confidence / self.config.min_confidence,
                current_intent: candidate,
            };
        }

        // Update temporal checker
        self.temporal.update(&candidate);
        self.current_candidate = Some(candidate.clone());

        // Check temporal stability
        let temporal_result = self.temporal.check();
        let stability_duration_ms = match temporal_result {
            TemporalStabilityResult::Stable { duration_ms } => duration_ms,
            TemporalStabilityResult::Unstable { remaining_ms, .. } => {
                return StabilizationResult::Unstable {
                    reason: UnstableReason::TemporalInstability { remaining_ms },
                    progress: 1.0
                        - (remaining_ms as f64 / self.config.stability_window_ms as f64),
                    current_intent: candidate,
                };
            }
            TemporalStabilityResult::NoIntent => {
                return StabilizationResult::Unstable {
                    reason: UnstableReason::NoGoals,
                    progress: 0.0,
                    current_intent: candidate,
                };
            }
        };

        // Check consistency with history
        let consistency_result = self.consistency.validate(&candidate, &self.history);
        let consistency_score = match consistency_result {
            ConsistencyResult::Consistent { score, .. } => score,
            ConsistencyResult::Inconsistent { score, .. } => {
                return StabilizationResult::Unstable {
                    reason: UnstableReason::HistoryInconsistency {
                        consistency_score: score,
                    },
                    progress: score / self.config.consistency_threshold,
                    current_intent: candidate,
                };
            }
            ConsistencyResult::Contradictory {
                conflicting_ids,
                suggestion,
            } => {
                return StabilizationResult::Contradictory {
                    conflicting_intents: conflicting_ids,
                    suggestion,
                };
            }
        };

        // Intent is stabilized!
        let identity = self
            .identity
            .clone()
            .unwrap_or_else(|| IdentityRef::new("unknown"));

        let goals: Vec<Goal> = std::iter::once(Goal::new(&candidate.objective))
            .chain(candidate.steps.iter().map(|s| Goal::new(s)))
            .collect();

        let rcf_intent = RcfIntent::new(identity, goals).with_confidence(candidate.confidence);

        let stabilized = StabilizedIntent {
            intent: rcf_intent,
            stabilized_at: Utc::now(),
            stability_duration_ms,
            consistency_score,
            meaning_refs: candidate.meaning_ref.clone().into_iter().collect(),
        };

        // Record in history
        self.history.add(IntentHistoryRecord {
            id: candidate.id.clone(),
            objective: candidate.objective,
            confidence: candidate.confidence,
            timestamp: Utc::now(),
            was_stabilized: true,
            meaning_ref: candidate.meaning_ref,
        });

        StabilizationResult::Stabilized {
            intent: stabilized.clone(),
            confidence: stabilized.confidence(),
        }
    }

    /// Check if current state would allow commitment.
    ///
    /// This is a preview check for Invariant #3 (Intent precedes Commitment).
    pub fn can_commit(&self) -> bool {
        if let Some(candidate) = &self.current_candidate {
            candidate.confidence >= self.config.min_confidence
                && matches!(
                    self.temporal.check(),
                    TemporalStabilityResult::Stable { .. }
                )
        } else {
            false
        }
    }

    /// Get the intent history.
    pub fn history(&self) -> &IntentHistory {
        &self.history
    }

    /// Get mutable access to history.
    pub fn history_mut(&mut self) -> &mut IntentHistory {
        &mut self.history
    }

    /// Get the current candidate if any.
    pub fn current_candidate(&self) -> Option<&IntentCandidate> {
        self.current_candidate.as_ref()
    }

    /// Get stabilization progress (0.0 to 1.0).
    pub fn stabilization_progress(&self) -> f64 {
        match self.temporal.check() {
            TemporalStabilityResult::Stable { .. } => 1.0,
            TemporalStabilityResult::Unstable {
                elapsed_ms,
                required_ms,
                ..
            } => elapsed_ms as f64 / required_ms as f64,
            TemporalStabilityResult::NoIntent => 0.0,
        }
    }

    /// Reset the engine state.
    pub fn reset(&mut self) {
        self.temporal.reset();
        self.current_candidate = None;
    }

    /// Clear all state including history.
    pub fn clear(&mut self) {
        self.reset();
        self.history.clear();
    }
}

impl Default for IntentStabilizationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during intent stabilization.
#[derive(Debug, Error)]
pub enum IntentError {
    /// Meaning not sufficient (Invariant #2).
    #[error("Insufficient meaning for intent formation (Invariant #2): confidence {0:.2}")]
    InsufficientMeaning(f64),

    /// Intent not stabilized (Invariant #3).
    #[error("Intent not stabilized for commitment (Invariant #3): {0}")]
    NotStabilized(String),

    /// Confidence too low.
    #[error("Confidence too low: {actual:.2} < {required:.2}")]
    InsufficientConfidence { actual: f64, required: f64 },

    /// Contradictory intent detected.
    #[error("Contradictory intent detected")]
    Contradiction,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcf_meaning::Claim;

    fn make_meaning(confidence: f64) -> RcfMeaning {
        RcfMeaning::new(
            IdentityRef::new("test"),
            vec![Claim::observation("test observation")],
        )
        .with_confidence(confidence)
    }

    #[test]
    fn test_intent_stabilizes_after_window() {
        let mut config = IntentStabilizationConfig::default();
        config.stability_window_ms = 0; // Immediate for testing
        config.min_confidence = 0.5;

        let mut engine = IntentStabilizationEngine::with_config(config);

        let candidate = IntentCandidate::new("Complete the task", 0.8);

        let result = engine.stabilize(candidate);

        assert!(matches!(result, StabilizationResult::Stabilized { .. }));
    }

    #[test]
    fn test_fleeting_intent_rejected() {
        let mut config = IntentStabilizationConfig::default();
        config.stability_window_ms = 10000; // 10 seconds
        config.min_confidence = 0.5;

        let mut engine = IntentStabilizationEngine::with_config(config);

        let candidate = IntentCandidate::new("Complete the task", 0.8);

        let result = engine.stabilize(candidate);

        // Should be unstable due to temporal requirements
        assert!(matches!(
            result,
            StabilizationResult::Unstable {
                reason: UnstableReason::TemporalInstability { .. },
                ..
            }
        ));
    }

    #[test]
    fn test_contradictory_intent_detected() {
        let mut config = IntentStabilizationConfig::default();
        config.stability_window_ms = 0;
        config.min_confidence = 0.5;

        let mut engine = IntentStabilizationEngine::with_config(config);

        // First intent
        let candidate1 = IntentCandidate::new("Allow the action", 0.8);
        let _ = engine.stabilize(candidate1);

        // Contradictory intent
        let candidate2 = IntentCandidate::new("Deny the action", 0.8);
        let result = engine.stabilize(candidate2);

        assert!(matches!(result, StabilizationResult::Contradictory { .. }));
    }

    #[test]
    fn test_insufficient_meaning_rejected() {
        let mut engine = IntentStabilizationEngine::new();

        let meaning = make_meaning(0.3); // Low confidence

        let result = engine.stabilize_from_meaning(&meaning, "Do something");

        assert!(matches!(
            result,
            StabilizationResult::Unstable {
                reason: UnstableReason::InsufficientMeaning { .. },
                ..
            }
        ));
    }

    #[test]
    fn test_consistency_score_computation() {
        let validator = ConsistencyValidator::new(0.7);

        let candidate = IntentCandidate::new("Process the data file", 0.8);

        let mut history = IntentHistory::new(10);
        history.add(IntentHistoryRecord {
            id: IntentId::new(),
            objective: "Process the data".to_string(),
            confidence: 0.8,
            timestamp: Utc::now(),
            was_stabilized: true,
            meaning_ref: None,
        });

        let result = validator.validate(&candidate, &history);

        assert!(matches!(result, ConsistencyResult::Consistent { .. }));
    }

    #[test]
    fn test_history_ring_buffer() {
        let mut history = IntentHistory::new(5);

        for i in 0..10 {
            history.add(IntentHistoryRecord {
                id: IntentId::new(),
                objective: format!("Intent {}", i),
                confidence: 0.8,
                timestamp: Utc::now(),
                was_stabilized: true,
                meaning_ref: None,
            });
        }

        assert_eq!(history.len(), 5);
        // Should have intents 5-9
        let recent = history.recent(5);
        assert!(recent[0].objective.contains("9"));
    }

    #[test]
    fn test_stabilization_progress() {
        let mut config = IntentStabilizationConfig::default();
        config.stability_window_ms = 1000;
        config.min_confidence = 0.5;

        let mut engine = IntentStabilizationEngine::with_config(config);

        // No candidate yet
        assert_eq!(engine.stabilization_progress(), 0.0);

        // Add candidate
        let candidate = IntentCandidate::new("Test", 0.8);
        let _ = engine.stabilize(candidate);

        // Should have some progress (but not 100% immediately)
        let progress = engine.stabilization_progress();
        assert!(progress >= 0.0 && progress <= 1.0);
    }
}
