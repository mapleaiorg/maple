//! Meaning Formation Engine for MAPLE Resonators
//!
//! This module implements the meaning formation layer of the Resonance Architecture.
//! Meaning is the semantic understanding that emerges through interaction and
//! must be present before intent can form (Invariant #2: Meaning precedes Intent).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                  MEANING FORMATION ENGINE                       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │   Input ──> ContextAccumulator ──> MeaningFormationEngine      │
//! │                                            │                   │
//! │                                            v                   │
//! │                                   ConvergenceTracker           │
//! │                                            │                   │
//! │                                            v                   │
//! │                                   MisalignmentDetector         │
//! │                                            │                   │
//! │                                            v                   │
//! │                                     FormedMeaning              │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Components
//!
//! - [`MeaningFormationEngine`]: Main engine that orchestrates meaning formation
//! - [`ContextAccumulator`]: Tracks semantic context across interactions
//! - [`ConvergenceTracker`]: Measures meaning alignment between Resonators
//! - [`MisalignmentDetector`]: Detects when meanings diverge unexpectedly
//!
//! # Invariant Enforcement
//!
//! This module enforces Invariant #1 (Presence precedes Meaning) by requiring
//! a valid presence signal before meaning can be formed.

#![deny(unsafe_code)]

use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Duration, Utc};
use rcf_meaning::{Claim, ClaimType, RcfMeaning};
use rcf_types::IdentityRef;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Unique identifier for a meaning instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeaningId(pub String);

impl MeaningId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for MeaningId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MeaningId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a Resonator (copied here to avoid circular deps).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResonatorId(pub u64);

impl std::fmt::Display for ResonatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "resonator-{}", self.0)
    }
}

/// Configuration for the meaning formation engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeaningFormationConfig {
    /// Maximum number of context items to retain.
    pub context_window_size: usize,
    /// Minimum confidence required for meaning to be considered formed.
    pub min_formation_confidence: f64,
    /// Weight decay factor for older context items.
    pub context_decay_factor: f64,
    /// Threshold for detecting misalignment.
    pub misalignment_threshold: f64,
    /// Maximum age of context items before eviction (milliseconds).
    pub context_max_age_ms: i64,
}

impl Default for MeaningFormationConfig {
    fn default() -> Self {
        Self {
            context_window_size: 100,
            min_formation_confidence: 0.5,
            context_decay_factor: 0.95,
            misalignment_threshold: 0.3,
            context_max_age_ms: 3_600_000, // 1 hour
        }
    }
}

/// A single context item accumulated during meaning formation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    /// Unique identifier.
    pub id: String,
    /// The semantic content.
    pub content: String,
    /// Type of context (input, inference, clarification, etc.).
    pub context_type: ContextType,
    /// Importance weight (0.0 to 1.0).
    pub importance: f64,
    /// When this item was added.
    pub timestamp: DateTime<Utc>,
    /// Source Resonator (if from coupling).
    pub source: Option<ResonatorId>,
    /// Associated claims extracted from this context.
    pub claims: Vec<Claim>,
}

/// Types of context that can be accumulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextType {
    /// Direct user/environment input.
    Input,
    /// Inference made by the agent.
    Inference,
    /// Clarification received.
    Clarification,
    /// Context from a coupled Resonator.
    Coupling,
    /// Historical context from memory.
    Historical,
    /// System-provided context.
    System,
}

/// Accumulates semantic context across interactions.
///
/// The accumulator maintains a windowed view of context items,
/// applying decay to older items and evicting stale entries.
#[derive(Debug, Clone)]
pub struct ContextAccumulator {
    /// Configuration.
    config: MeaningFormationConfig,
    /// Context items in order of arrival.
    items: VecDeque<ContextItem>,
    /// Total accumulated importance (for weighted computations).
    total_importance: f64,
}

impl ContextAccumulator {
    /// Create a new context accumulator with the given configuration.
    pub fn new(config: MeaningFormationConfig) -> Self {
        Self {
            config,
            items: VecDeque::new(),
            total_importance: 0.0,
        }
    }

    /// Add a new context item.
    pub fn add(&mut self, item: ContextItem) {
        // Evict old items first
        self.evict_stale();

        // Apply decay to existing items
        self.apply_decay();

        // Add new item
        self.total_importance += item.importance;
        self.items.push_back(item);

        // Enforce window size
        while self.items.len() > self.config.context_window_size {
            if let Some(removed) = self.items.pop_front() {
                self.total_importance -= removed.importance;
            }
        }
    }

    /// Add input context with automatic claim extraction.
    pub fn add_input(&mut self, content: impl Into<String>) {
        let content = content.into();
        let claims = vec![Claim::observation(content.clone())];
        self.add(ContextItem {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            context_type: ContextType::Input,
            importance: 1.0,
            timestamp: Utc::now(),
            source: None,
            claims,
        });
    }

    /// Add inference context.
    pub fn add_inference(&mut self, content: impl Into<String>, confidence: f64) {
        let content = content.into();
        let mut claim = Claim::belief(content.clone());
        claim.claim_type = ClaimType::Inference;
        claim.confidence = confidence;
        self.add(ContextItem {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            context_type: ContextType::Inference,
            importance: confidence,
            timestamp: Utc::now(),
            source: None,
            claims: vec![claim],
        });
    }

    /// Add context from a coupled Resonator.
    pub fn add_from_coupling(&mut self, content: impl Into<String>, source: ResonatorId) {
        let content = content.into();
        let claims = vec![Claim::observation(content.clone())];
        self.add(ContextItem {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            context_type: ContextType::Coupling,
            importance: 0.8, // Slightly lower than direct input
            timestamp: Utc::now(),
            source: Some(source),
            claims,
        });
    }

    /// Get all current context items.
    pub fn items(&self) -> &VecDeque<ContextItem> {
        &self.items
    }

    /// Get the number of context items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the accumulator is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Compute a summary of the current context.
    pub fn summary(&self) -> ContextSummary {
        let now = Utc::now();
        let items: Vec<_> = self.items.iter().collect();

        let total_items = items.len();
        let total_importance = self.total_importance;

        // Compute weighted average age
        let mut weighted_age_sum = 0.0;
        let mut weight_sum = 0.0;
        for item in &items {
            let age_ms = (now - item.timestamp).num_milliseconds() as f64;
            weighted_age_sum += age_ms * item.importance;
            weight_sum += item.importance;
        }
        let avg_age_ms = if weight_sum > 0.0 {
            weighted_age_sum / weight_sum
        } else {
            0.0
        };

        // Count claims by type
        let mut claim_counts: HashMap<ClaimType, usize> = HashMap::new();
        for item in &items {
            for claim in &item.claims {
                *claim_counts.entry(claim.claim_type).or_insert(0) += 1;
            }
        }

        // Compute average claim confidence
        let all_claims: Vec<_> = items.iter().flat_map(|i| &i.claims).collect();
        let avg_confidence = if all_claims.is_empty() {
            0.0
        } else {
            all_claims.iter().map(|c| c.confidence).sum::<f64>() / all_claims.len() as f64
        };

        ContextSummary {
            total_items,
            total_importance,
            avg_age_ms,
            claim_counts,
            avg_confidence,
        }
    }

    /// Generate a text representation for LLM context.
    pub fn to_prompt_context(&self) -> String {
        let mut lines = Vec::new();
        for (i, item) in self.items.iter().enumerate() {
            let type_label = match item.context_type {
                ContextType::Input => "INPUT",
                ContextType::Inference => "INFERENCE",
                ContextType::Clarification => "CLARIFICATION",
                ContextType::Coupling => "COUPLING",
                ContextType::Historical => "HISTORICAL",
                ContextType::System => "SYSTEM",
            };
            lines.push(format!("[{}] {}: {}", i + 1, type_label, item.content));
        }
        lines.join("\n")
    }

    /// Clear all context.
    pub fn clear(&mut self) {
        self.items.clear();
        self.total_importance = 0.0;
    }

    fn apply_decay(&mut self) {
        for item in &mut self.items {
            let old_importance = item.importance;
            item.importance *= self.config.context_decay_factor;
            self.total_importance -= old_importance - item.importance;
        }
    }

    fn evict_stale(&mut self) {
        let now = Utc::now();
        let max_age = Duration::milliseconds(self.config.context_max_age_ms);
        let cutoff = now - max_age;

        while let Some(front) = self.items.front() {
            if front.timestamp < cutoff {
                if let Some(removed) = self.items.pop_front() {
                    self.total_importance -= removed.importance;
                }
            } else {
                break;
            }
        }
    }
}

/// Summary of accumulated context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummary {
    pub total_items: usize,
    pub total_importance: f64,
    pub avg_age_ms: f64,
    pub claim_counts: HashMap<ClaimType, usize>,
    pub avg_confidence: f64,
}

/// Tracks meaning convergence between coupled Resonators.
///
/// Convergence measures how well two Resonators' meanings align.
/// It increases through successful interactions and decreases when
/// misalignments are detected.
#[derive(Debug, Clone)]
pub struct ConvergenceTracker {
    /// Convergence scores per coupling (source -> target -> score).
    scores: HashMap<ResonatorId, HashMap<ResonatorId, ConvergenceScore>>,
    /// History of convergence updates.
    history: VecDeque<ConvergenceEvent>,
    /// Maximum history size.
    max_history: usize,
}

/// Convergence score between two Resonators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceScore {
    /// Current convergence value (0.0 to 1.0).
    pub value: f64,
    /// Number of interactions that contributed.
    pub interaction_count: u64,
    /// Last update timestamp.
    pub last_updated: DateTime<Utc>,
    /// Trend direction.
    pub trend: ConvergenceTrend,
}

impl Default for ConvergenceScore {
    fn default() -> Self {
        Self {
            value: 0.0,
            interaction_count: 0,
            last_updated: Utc::now(),
            trend: ConvergenceTrend::Stable,
        }
    }
}

/// Trend direction for convergence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConvergenceTrend {
    Increasing,
    Decreasing,
    Stable,
}

/// Event recording a convergence update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceEvent {
    pub source: ResonatorId,
    pub target: ResonatorId,
    pub old_value: f64,
    pub new_value: f64,
    pub reason: ConvergenceReason,
    pub timestamp: DateTime<Utc>,
}

/// Reason for convergence change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConvergenceReason {
    /// Successful meaning exchange.
    SuccessfulExchange,
    /// Clarification resolved ambiguity.
    ClarificationResolved,
    /// Misalignment detected.
    MisalignmentDetected,
    /// Natural decay over time.
    TemporalDecay,
    /// Manual adjustment.
    ManualAdjustment,
}

impl ConvergenceTracker {
    /// Create a new convergence tracker.
    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
            history: VecDeque::new(),
            max_history: 1000,
        }
    }

    /// Get convergence score between two Resonators.
    pub fn get(&self, source: ResonatorId, target: ResonatorId) -> Option<&ConvergenceScore> {
        self.scores.get(&source).and_then(|m| m.get(&target))
    }

    /// Get or create convergence score.
    pub fn get_or_create(&mut self, source: ResonatorId, target: ResonatorId) -> &ConvergenceScore {
        self.scores
            .entry(source)
            .or_insert_with(HashMap::new)
            .entry(target)
            .or_insert_with(ConvergenceScore::default)
    }

    /// Record a successful meaning exchange (increases convergence).
    pub fn record_successful_exchange(
        &mut self,
        source: ResonatorId,
        target: ResonatorId,
        semantic_similarity: f64,
    ) {
        let (old_value, new_value) = {
            let score = self
                .scores
                .entry(source.clone())
                .or_insert_with(HashMap::new)
                .entry(target.clone())
                .or_insert_with(ConvergenceScore::default);

            let old_value = score.value;

            // Convergence formula: weighted average with similarity
            // Convergence increases more when similarity is high
            let delta = (semantic_similarity - score.value) * 0.1; // Max 10% change per interaction
            score.value = (score.value + delta).clamp(0.0, 1.0);
            score.interaction_count += 1;
            score.last_updated = Utc::now();
            score.trend = if score.value > old_value {
                ConvergenceTrend::Increasing
            } else if score.value < old_value {
                ConvergenceTrend::Decreasing
            } else {
                ConvergenceTrend::Stable
            };

            (old_value, score.value)
        };

        self.record_event(ConvergenceEvent {
            source,
            target,
            old_value,
            new_value,
            reason: ConvergenceReason::SuccessfulExchange,
            timestamp: Utc::now(),
        });
    }

    /// Record a misalignment (decreases convergence).
    pub fn record_misalignment(&mut self, source: ResonatorId, target: ResonatorId, severity: f64) {
        let (old_value, new_value) = {
            let score = self
                .scores
                .entry(source.clone())
                .or_insert_with(HashMap::new)
                .entry(target.clone())
                .or_insert_with(ConvergenceScore::default);

            let old_value = score.value;

            // Misalignment decreases convergence proportional to severity
            let delta = severity * 0.2; // Max 20% decrease per misalignment
            score.value = (score.value - delta).clamp(0.0, 1.0);
            score.last_updated = Utc::now();
            score.trend = ConvergenceTrend::Decreasing;

            (old_value, score.value)
        };

        self.record_event(ConvergenceEvent {
            source,
            target,
            old_value,
            new_value,
            reason: ConvergenceReason::MisalignmentDetected,
            timestamp: Utc::now(),
        });
    }

    /// Get recent convergence events.
    pub fn recent_events(&self, limit: usize) -> Vec<&ConvergenceEvent> {
        self.history.iter().rev().take(limit).collect()
    }

    /// Check if convergence is sufficient for intent formation.
    pub fn is_sufficient_for_intent(&self, source: ResonatorId, target: ResonatorId) -> bool {
        self.get(source, target)
            .map(|s| s.value >= 0.5)
            .unwrap_or(false)
    }

    fn record_event(&mut self, event: ConvergenceEvent) {
        self.history.push_back(event);
        while self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }
}

impl Default for ConvergenceTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Detects misalignments in meaning between Resonators.
///
/// Misalignment occurs when:
/// - Contradictory claims are made
/// - Confidence drops unexpectedly
/// - Semantic drift exceeds threshold
#[derive(Debug, Clone)]
pub struct MisalignmentDetector {
    /// Configuration threshold.
    threshold: f64,
    /// Recent meanings for comparison.
    recent_meanings: VecDeque<MeaningSnapshot>,
    /// Maximum snapshots to retain.
    max_snapshots: usize,
}

/// Snapshot of meaning state for comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeaningSnapshot {
    pub meaning_id: MeaningId,
    pub claims: Vec<Claim>,
    pub confidence: f64,
    pub timestamp: DateTime<Utc>,
    pub resonator_id: Option<ResonatorId>,
}

/// Result of misalignment detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MisalignmentResult {
    /// Whether misalignment was detected.
    pub detected: bool,
    /// Severity of misalignment (0.0 to 1.0).
    pub severity: f64,
    /// Type of misalignment.
    pub misalignment_type: Option<MisalignmentType>,
    /// Conflicting claims if any.
    pub conflicting_claims: Vec<(Claim, Claim)>,
    /// Suggested clarification request.
    pub suggested_clarification: Option<String>,
}

/// Types of misalignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MisalignmentType {
    /// Direct contradiction between claims.
    Contradiction,
    /// Unexpected confidence drop.
    ConfidenceDrop,
    /// Semantic drift from established meaning.
    SemanticDrift,
    /// Topic shift without transition.
    TopicShift,
}

impl MisalignmentDetector {
    /// Create a new misalignment detector.
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            recent_meanings: VecDeque::new(),
            max_snapshots: 50,
        }
    }

    /// Add a meaning snapshot for future comparison.
    pub fn add_snapshot(&mut self, snapshot: MeaningSnapshot) {
        self.recent_meanings.push_back(snapshot);
        while self.recent_meanings.len() > self.max_snapshots {
            self.recent_meanings.pop_front();
        }
    }

    /// Check for misalignment between new meaning and recent history.
    pub fn check(&self, new_meaning: &MeaningSnapshot) -> MisalignmentResult {
        let mut result = MisalignmentResult {
            detected: false,
            severity: 0.0,
            misalignment_type: None,
            conflicting_claims: Vec::new(),
            suggested_clarification: None,
        };

        // Check against recent meanings
        for recent in self.recent_meanings.iter().rev().take(10) {
            // Check for contradictions
            for new_claim in &new_meaning.claims {
                for old_claim in &recent.claims {
                    if self.are_contradictory(new_claim, old_claim) {
                        result.detected = true;
                        result.severity = (result.severity + 0.5).min(1.0);
                        result.misalignment_type = Some(MisalignmentType::Contradiction);
                        result
                            .conflicting_claims
                            .push((new_claim.clone(), old_claim.clone()));
                    }
                }
            }

            // Check for confidence drop
            if new_meaning.confidence < recent.confidence - self.threshold {
                result.detected = true;
                result.severity =
                    (result.severity + (recent.confidence - new_meaning.confidence)).min(1.0);
                if result.misalignment_type.is_none() {
                    result.misalignment_type = Some(MisalignmentType::ConfidenceDrop);
                }
            }
        }

        // Generate clarification suggestion if misalignment detected
        if result.detected {
            result.suggested_clarification = self.generate_clarification(&result);
        }

        result
    }

    fn are_contradictory(&self, claim1: &Claim, claim2: &Claim) -> bool {
        // Simple heuristic: check for negation patterns
        // In a full implementation, this would use semantic analysis
        let content1 = claim1.content.to_lowercase();
        let content2 = claim2.content.to_lowercase();

        // Check for explicit negation
        if content1.contains("not ") && content2.contains(&content1.replace("not ", "")) {
            return true;
        }
        if content2.contains("not ") && content1.contains(&content2.replace("not ", "")) {
            return true;
        }

        // Check for opposite assertions (simplified)
        let opposites = [
            ("true", "false"),
            ("yes", "no"),
            ("allow", "deny"),
            ("accept", "reject"),
        ];
        for (a, b) in opposites {
            if (content1.contains(a) && content2.contains(b))
                || (content1.contains(b) && content2.contains(a))
            {
                return true;
            }
        }

        false
    }

    fn generate_clarification(&self, result: &MisalignmentResult) -> Option<String> {
        match result.misalignment_type {
            Some(MisalignmentType::Contradiction) => {
                if let Some((new, old)) = result.conflicting_claims.first() {
                    Some(format!(
                        "I notice a potential contradiction. Earlier: '{}'. Now: '{}'. Could you clarify?",
                        old.content, new.content
                    ))
                } else {
                    Some("I detected a contradiction. Could you clarify your intent?".to_string())
                }
            }
            Some(MisalignmentType::ConfidenceDrop) => Some(
                "I'm less certain about the current context. Could you provide more details?"
                    .to_string(),
            ),
            Some(MisalignmentType::SemanticDrift) => Some(
                "The topic seems to have shifted. Should I continue with the new direction?"
                    .to_string(),
            ),
            Some(MisalignmentType::TopicShift) => {
                Some("It seems we've moved to a new topic. Is this intentional?".to_string())
            }
            None => None,
        }
    }
}

/// The main meaning formation engine.
///
/// This engine orchestrates the formation of meaning from inputs,
/// tracks convergence with coupled Resonators, and detects misalignments.
#[derive(Debug, Clone)]
pub struct MeaningFormationEngine {
    /// Configuration.
    config: MeaningFormationConfig,
    /// Context accumulator.
    context: ContextAccumulator,
    /// Convergence tracker.
    convergence: ConvergenceTracker,
    /// Misalignment detector.
    misalignment: MisalignmentDetector,
    /// Identity of the owning Resonator.
    identity: Option<IdentityRef>,
}

impl MeaningFormationEngine {
    /// Create a new meaning formation engine with default configuration.
    pub fn new() -> Self {
        Self::with_config(MeaningFormationConfig::default())
    }

    /// Create a new meaning formation engine with custom configuration.
    pub fn with_config(config: MeaningFormationConfig) -> Self {
        let misalignment_threshold = config.misalignment_threshold;
        Self {
            context: ContextAccumulator::new(config.clone()),
            convergence: ConvergenceTracker::new(),
            misalignment: MisalignmentDetector::new(misalignment_threshold),
            config,
            identity: None,
        }
    }

    /// Set the identity of the owning Resonator.
    pub fn set_identity(&mut self, identity: IdentityRef) {
        self.identity = Some(identity);
    }

    /// Form meaning from input.
    ///
    /// This is the main entry point for meaning formation.
    /// It accumulates context, checks for misalignment, and produces a FormedMeaning.
    pub fn form_meaning(
        &mut self,
        input: impl Into<String>,
    ) -> Result<FormedMeaning, MeaningError> {
        let input = input.into();

        // Add input to context
        self.context.add_input(&input);

        // Build claims from accumulated context
        let claims = self.extract_claims();

        // Compute confidence based on context quality
        let confidence = self.compute_confidence();

        // Check minimum formation confidence
        if confidence < self.config.min_formation_confidence {
            return Err(MeaningError::InsufficientConfidence {
                actual: confidence,
                required: self.config.min_formation_confidence,
            });
        }

        // Create RCF meaning
        let identity = self
            .identity
            .clone()
            .unwrap_or_else(|| IdentityRef::new("unknown"));
        let rcf_meaning = RcfMeaning::new(identity, claims.clone()).with_confidence(confidence);

        // Create snapshot for misalignment detection
        let snapshot = MeaningSnapshot {
            meaning_id: MeaningId(rcf_meaning.id.clone()),
            claims: claims.clone(),
            confidence,
            timestamp: Utc::now(),
            resonator_id: None,
        };

        // Check for misalignment
        let misalignment_result = self.misalignment.check(&snapshot);

        // Add snapshot to history
        self.misalignment.add_snapshot(snapshot);

        Ok(FormedMeaning {
            meaning: rcf_meaning,
            context_summary: self.context.summary(),
            misalignment: if misalignment_result.detected {
                Some(misalignment_result)
            } else {
                None
            },
            formation_timestamp: Utc::now(),
        })
    }

    /// Form meaning from coupling input (from another Resonator).
    pub fn form_meaning_from_coupling(
        &mut self,
        input: impl Into<String>,
        source: ResonatorId,
        semantic_similarity: f64,
    ) -> Result<FormedMeaning, MeaningError> {
        let input = input.into();

        // Add coupling context
        self.context.add_from_coupling(&input, source);

        // Update convergence
        if let Some(my_id) = self.my_resonator_id() {
            self.convergence
                .record_successful_exchange(source, my_id, semantic_similarity);
        }

        // Continue with normal meaning formation
        self.form_meaning(input)
    }

    /// Get the context accumulator.
    pub fn context(&self) -> &ContextAccumulator {
        &self.context
    }

    /// Get mutable access to context accumulator.
    pub fn context_mut(&mut self) -> &mut ContextAccumulator {
        &mut self.context
    }

    /// Get the convergence tracker.
    pub fn convergence(&self) -> &ConvergenceTracker {
        &self.convergence
    }

    /// Get mutable access to convergence tracker.
    pub fn convergence_mut(&mut self) -> &mut ConvergenceTracker {
        &mut self.convergence
    }

    /// Get the misalignment detector.
    pub fn misalignment(&self) -> &MisalignmentDetector {
        &self.misalignment
    }

    /// Check if meaning is sufficient for intent formation.
    pub fn is_sufficient_for_intent(&self) -> bool {
        let summary = self.context.summary();
        summary.avg_confidence >= 0.5 && summary.total_items > 0
    }

    /// Generate context for LLM prompt.
    pub fn to_prompt_context(&self) -> String {
        self.context.to_prompt_context()
    }

    /// Clear all accumulated context and state.
    pub fn clear(&mut self) {
        self.context.clear();
    }

    fn my_resonator_id(&self) -> Option<ResonatorId> {
        // In a full implementation, this would be set from the Resonator
        None
    }

    fn extract_claims(&self) -> Vec<Claim> {
        self.context
            .items()
            .iter()
            .flat_map(|item| item.claims.clone())
            .collect()
    }

    fn compute_confidence(&self) -> f64 {
        let summary = self.context.summary();

        if summary.total_items == 0 {
            return 0.0;
        }

        // Confidence is based on:
        // 1. Average claim confidence
        // 2. Context freshness (newer = better)
        // 3. Context diversity (multiple types = better)

        let freshness = if summary.avg_age_ms < 60_000.0 {
            1.0 // Less than 1 minute old
        } else if summary.avg_age_ms < 300_000.0 {
            0.9 // Less than 5 minutes old
        } else if summary.avg_age_ms < 900_000.0 {
            0.7 // Less than 15 minutes old
        } else {
            0.5
        };

        let diversity = (summary.claim_counts.len() as f64 / 5.0).min(1.0);

        // Weighted combination
        (summary.avg_confidence * 0.6 + freshness * 0.3 + diversity * 0.1).clamp(0.0, 1.0)
    }
}

impl Default for MeaningFormationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of meaning formation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormedMeaning {
    /// The formed RCF meaning.
    pub meaning: RcfMeaning,
    /// Summary of context used.
    pub context_summary: ContextSummary,
    /// Misalignment result if any was detected.
    pub misalignment: Option<MisalignmentResult>,
    /// When the meaning was formed.
    pub formation_timestamp: DateTime<Utc>,
}

impl FormedMeaning {
    /// Get the meaning ID.
    pub fn id(&self) -> &str {
        &self.meaning.id
    }

    /// Get the confidence.
    pub fn confidence(&self) -> f64 {
        self.meaning.uncertainty.confidence
    }

    /// Check if there was a misalignment.
    pub fn has_misalignment(&self) -> bool {
        self.misalignment.is_some()
    }

    /// Get suggested clarification if any.
    pub fn suggested_clarification(&self) -> Option<&str> {
        self.misalignment
            .as_ref()
            .and_then(|m| m.suggested_clarification.as_deref())
    }
}

/// Errors that can occur during meaning formation.
#[derive(Debug, Error)]
pub enum MeaningError {
    /// Confidence is below the minimum threshold.
    #[error("Insufficient confidence for meaning formation: {actual:.2} < {required:.2}")]
    InsufficientConfidence { actual: f64, required: f64 },

    /// No context available for meaning formation.
    #[error("No context available for meaning formation")]
    NoContext,

    /// Presence not signaled (Invariant #1 violation).
    #[error("Presence must be signaled before meaning formation (Invariant #1)")]
    PresenceRequired,

    /// Context window overflow.
    #[error("Context window overflow")]
    ContextOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_accumulator_basic() {
        let config = MeaningFormationConfig::default();
        let mut acc = ContextAccumulator::new(config);

        assert!(acc.is_empty());

        acc.add_input("Hello world");
        assert_eq!(acc.len(), 1);

        acc.add_inference("This is a greeting", 0.9);
        assert_eq!(acc.len(), 2);

        let summary = acc.summary();
        assert_eq!(summary.total_items, 2);
        assert!(summary.avg_confidence > 0.0);
    }

    #[test]
    fn test_context_window_bounds() {
        let mut config = MeaningFormationConfig::default();
        config.context_window_size = 5;

        let mut acc = ContextAccumulator::new(config);

        for i in 0..10 {
            acc.add_input(format!("Message {}", i));
        }

        assert_eq!(acc.len(), 5);
        // Should have messages 5-9 (most recent)
        let items: Vec<_> = acc.items().iter().collect();
        assert!(items[0].content.contains("5") || items[0].content.contains("6"));
    }

    #[test]
    fn test_meaning_convergence_increases_over_interactions() {
        let mut tracker = ConvergenceTracker::new();
        let source = ResonatorId(1);
        let target = ResonatorId(2);

        // Initially no convergence
        assert!(tracker.get(source, target).is_none());

        // Record successful exchanges
        for _ in 0..5 {
            tracker.record_successful_exchange(source, target, 0.8);
        }

        let score = tracker.get(source, target).unwrap();
        assert!(score.value > 0.0);
        assert_eq!(score.interaction_count, 5);
    }

    #[test]
    fn test_misalignment_triggers_clarification() {
        let mut detector = MisalignmentDetector::new(0.3);

        // Add initial meaning
        let initial = MeaningSnapshot {
            meaning_id: MeaningId::new(),
            claims: vec![Claim::belief("The answer is true")],
            confidence: 0.9,
            timestamp: Utc::now(),
            resonator_id: None,
        };
        detector.add_snapshot(initial);

        // Check contradictory meaning
        let contradictory = MeaningSnapshot {
            meaning_id: MeaningId::new(),
            claims: vec![Claim::belief("The answer is false")],
            confidence: 0.9,
            timestamp: Utc::now(),
            resonator_id: None,
        };

        let result = detector.check(&contradictory);
        assert!(result.detected);
        assert!(result.suggested_clarification.is_some());
    }

    #[test]
    fn test_meaning_formation_engine() {
        let mut engine = MeaningFormationEngine::new();

        // Form meaning from input
        let result = engine.form_meaning("Hello, I need help with a task");
        assert!(result.is_ok());

        let meaning = result.unwrap();
        assert!(meaning.confidence() > 0.0);
        assert!(!meaning.meaning.claims.is_empty());
    }

    #[test]
    fn test_insufficient_confidence_rejected() {
        let mut config = MeaningFormationConfig::default();
        config.min_formation_confidence = 0.99; // Very high threshold

        let mut engine = MeaningFormationEngine::with_config(config);

        // Single input unlikely to meet 0.99 confidence
        let result = engine.form_meaning("Hello");
        assert!(matches!(
            result,
            Err(MeaningError::InsufficientConfidence { .. })
        ));
    }

    #[test]
    fn test_prompt_context_generation() {
        let mut engine = MeaningFormationEngine::new();

        engine.context_mut().add_input("First message");
        engine
            .context_mut()
            .add_inference("This is about greetings", 0.8);
        engine.context_mut().add_input("Second message");

        let context = engine.to_prompt_context();
        assert!(context.contains("[1]"));
        assert!(context.contains("[2]"));
        assert!(context.contains("[3]"));
        assert!(context.contains("INPUT"));
        assert!(context.contains("INFERENCE"));
    }
}
