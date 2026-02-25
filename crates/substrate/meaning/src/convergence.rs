//! Convergence tracker — monitors whether meanings are stabilizing.
//!
//! Tracks confidence trajectories over time and determines when a meaning
//! has converged (stabilized enough for intent formation).

use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{MeaningConfig, MeaningId};

// ── Constants ───────────────────────────────────────────────────────────

/// Maximum number of confidence history entries per meaning.
const MAX_HISTORY_ENTRIES: usize = 50;

/// Variance threshold for convergence (below this → stable).
const DEFAULT_VARIANCE_THRESHOLD: f64 = 0.005;

// ── Confidence Trend ────────────────────────────────────────────────────

/// Direction of the confidence trend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceTrend {
    /// Confidence is increasing.
    Rising,
    /// Confidence is decreasing.
    Falling,
    /// Confidence is stable.
    Stable,
    /// Confidence is oscillating without clear direction.
    Oscillating,
}

impl std::fmt::Display for ConfidenceTrend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rising => write!(f, "rising"),
            Self::Falling => write!(f, "falling"),
            Self::Stable => write!(f, "stable"),
            Self::Oscillating => write!(f, "oscillating"),
        }
    }
}

// ── Convergence State ───────────────────────────────────────────────────

/// Convergence state for a single meaning.
#[derive(Clone, Debug)]
pub struct ConvergenceState {
    /// Recent confidence values with timestamps.
    pub confidence_history: VecDeque<(DateTime<Utc>, f64)>,
    /// Rolling variance of recent confidence values.
    pub confidence_variance: f64,
    /// Detected trend direction.
    pub trend: ConfidenceTrend,
    /// Whether this meaning has converged.
    pub converged: bool,
    /// When this meaning was first tracked.
    pub first_seen: DateTime<Utc>,
    /// Number of evidence items received.
    pub evidence_count: usize,
}

impl ConvergenceState {
    fn new() -> Self {
        Self {
            confidence_history: VecDeque::with_capacity(MAX_HISTORY_ENTRIES),
            confidence_variance: 1.0,
            trend: ConfidenceTrend::Oscillating,
            converged: false,
            first_seen: Utc::now(),
            evidence_count: 0,
        }
    }
}

// ── Convergence Tracker ─────────────────────────────────────────────────

/// Tracks convergence state for all active meanings.
///
/// A meaning is considered converged when:
/// 1. Confidence variance is below the threshold
/// 2. Sufficient evidence has been accumulated
/// 3. Sufficient time has elapsed since first observation
pub struct ConvergenceTracker {
    /// Per-meaning convergence state.
    states: HashMap<MeaningId, ConvergenceState>,
    /// Variance threshold for convergence.
    variance_threshold: f64,
    /// Minimum evidence count for convergence.
    min_evidence_count: usize,
    /// Minimum observation period (seconds) for convergence.
    min_observation_secs: u64,
}

impl Default for ConvergenceTracker {
    fn default() -> Self {
        Self {
            states: HashMap::new(),
            variance_threshold: DEFAULT_VARIANCE_THRESHOLD,
            min_evidence_count: 10,
            min_observation_secs: 3600,
        }
    }
}

impl ConvergenceTracker {
    /// Create a convergence tracker from a meaning configuration.
    pub fn from_config(config: &MeaningConfig) -> Self {
        Self {
            states: HashMap::new(),
            variance_threshold: 1.0 - config.convergence_threshold,
            min_evidence_count: config.min_evidence_count,
            min_observation_secs: config.min_observation_secs,
        }
    }

    /// Record a new confidence observation for a meaning.
    pub fn record(&mut self, meaning_id: &MeaningId, confidence: f64, evidence_count: usize) {
        let variance_threshold = self.variance_threshold;
        let min_evidence_count = self.min_evidence_count;
        let min_observation_secs = self.min_observation_secs;

        let state = self
            .states
            .entry(meaning_id.clone())
            .or_insert_with(ConvergenceState::new);

        state.evidence_count = evidence_count;

        // Append to history, bounded
        if state.confidence_history.len() >= MAX_HISTORY_ENTRIES {
            state.confidence_history.pop_front();
        }
        state.confidence_history.push_back((Utc::now(), confidence));

        // Update variance
        state.confidence_variance = compute_variance(&state.confidence_history);

        // Update trend
        state.trend = detect_trend(&state.confidence_history);

        // Check convergence
        state.converged = check_converged(
            state,
            variance_threshold,
            min_evidence_count,
            min_observation_secs,
        );
    }

    /// Check if a meaning has converged.
    pub fn is_converged(&self, meaning_id: &MeaningId) -> bool {
        self.states
            .get(meaning_id)
            .map(|s| s.converged)
            .unwrap_or(false)
    }

    /// Get convergence state for a meaning.
    pub fn get_state(&self, meaning_id: &MeaningId) -> Option<&ConvergenceState> {
        self.states.get(meaning_id)
    }

    /// Remove tracking for a meaning (cleanup on abandon/resolve).
    pub fn remove(&mut self, meaning_id: &MeaningId) {
        self.states.remove(meaning_id);
    }

    /// Number of meanings being tracked.
    pub fn tracked_count(&self) -> usize {
        self.states.len()
    }
}

/// Compute rolling variance of confidence values.
fn compute_variance(history: &VecDeque<(DateTime<Utc>, f64)>) -> f64 {
    if history.len() < 2 {
        return 1.0; // High variance when insufficient data
    }

    let values: Vec<f64> = history.iter().map(|(_, c)| *c).collect();
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;

    values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n
}

/// Detect trend direction from confidence history.
fn detect_trend(history: &VecDeque<(DateTime<Utc>, f64)>) -> ConfidenceTrend {
    if history.len() < 4 {
        return ConfidenceTrend::Oscillating;
    }

    let values: Vec<f64> = history.iter().map(|(_, c)| *c).collect();
    let mid = values.len() / 2;

    let first_half_mean = values[..mid].iter().sum::<f64>() / mid as f64;
    let second_half_mean = values[mid..].iter().sum::<f64>() / (values.len() - mid) as f64;
    let diff = second_half_mean - first_half_mean;

    if diff.abs() < 0.02 {
        ConfidenceTrend::Stable
    } else if diff > 0.0 {
        let second_half_var =
            compute_variance(&history.iter().skip(mid).cloned().collect::<VecDeque<_>>());
        if second_half_var > 0.01 {
            ConfidenceTrend::Oscillating
        } else {
            ConfidenceTrend::Rising
        }
    } else {
        let second_half_var =
            compute_variance(&history.iter().skip(mid).cloned().collect::<VecDeque<_>>());
        if second_half_var > 0.01 {
            ConfidenceTrend::Oscillating
        } else {
            ConfidenceTrend::Falling
        }
    }
}

/// Check all convergence criteria.
fn check_converged(
    state: &ConvergenceState,
    variance_threshold: f64,
    min_evidence_count: usize,
    min_observation_secs: u64,
) -> bool {
    // Criterion 1: Low variance
    if state.confidence_variance > variance_threshold {
        return false;
    }

    // Criterion 2: Sufficient evidence
    if state.evidence_count < min_evidence_count {
        return false;
    }

    // Criterion 3: Sufficient elapsed time
    let elapsed_secs = (Utc::now() - state.first_seen).num_seconds().max(0) as u64;
    if elapsed_secs < min_observation_secs {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_meaning_is_not_converged() {
        let tracker = ConvergenceTracker::default();
        let id = MeaningId::new();
        assert!(!tracker.is_converged(&id));
    }

    #[test]
    fn single_observation_not_converged() {
        let mut tracker = ConvergenceTracker::default();
        let id = MeaningId::new();
        tracker.record(&id, 0.8, 15);
        assert!(!tracker.is_converged(&id));
        // Variance should be high with single observation
        let state = tracker.get_state(&id).unwrap();
        assert!(state.confidence_variance > 0.0);
    }

    #[test]
    fn stable_observations_reduce_variance() {
        let mut tracker = ConvergenceTracker {
            min_observation_secs: 0, // disable time check for test
            min_evidence_count: 5,
            ..ConvergenceTracker::default()
        };
        let id = MeaningId::new();

        // Record many similar observations
        for _ in 0..20 {
            tracker.record(&id, 0.8, 15);
        }

        let state = tracker.get_state(&id).unwrap();
        assert!(
            state.confidence_variance < 0.01,
            "Stable observations should have low variance: {}",
            state.confidence_variance
        );
    }

    #[test]
    fn oscillating_observations_have_high_variance() {
        let mut tracker = ConvergenceTracker {
            min_observation_secs: 0,
            min_evidence_count: 5,
            ..ConvergenceTracker::default()
        };
        let id = MeaningId::new();

        // Record oscillating observations
        for i in 0..20 {
            let confidence = if i % 2 == 0 { 0.3 } else { 0.9 };
            tracker.record(&id, confidence, 15);
        }

        let state = tracker.get_state(&id).unwrap();
        assert!(
            state.confidence_variance > 0.05,
            "Oscillating should have high variance: {}",
            state.confidence_variance
        );
        assert!(!tracker.is_converged(&id));
    }

    #[test]
    fn convergence_requires_evidence_count() {
        let mut tracker = ConvergenceTracker {
            min_observation_secs: 0,
            min_evidence_count: 10,
            ..ConvergenceTracker::default()
        };
        let id = MeaningId::new();

        for _ in 0..20 {
            tracker.record(&id, 0.8, 5); // only 5 evidence items
        }

        assert!(
            !tracker.is_converged(&id),
            "Should not converge with insufficient evidence"
        );
    }

    #[test]
    fn remove_clears_state() {
        let mut tracker = ConvergenceTracker::default();
        let id = MeaningId::new();
        tracker.record(&id, 0.8, 15);
        assert_eq!(tracker.tracked_count(), 1);

        tracker.remove(&id);
        assert_eq!(tracker.tracked_count(), 0);
        assert!(tracker.get_state(&id).is_none());
    }

    #[test]
    fn trend_detection_rising() {
        let mut tracker = ConvergenceTracker {
            min_observation_secs: 0,
            min_evidence_count: 5,
            ..ConvergenceTracker::default()
        };
        let id = MeaningId::new();

        // Gradually rising confidence
        for i in 0..10 {
            tracker.record(&id, 0.3 + 0.05 * i as f64, 15);
        }

        let state = tracker.get_state(&id).unwrap();
        assert_eq!(state.trend, ConfidenceTrend::Rising);
    }

    #[test]
    fn from_config_applies_settings() {
        let config = MeaningConfig {
            convergence_threshold: 0.9,
            min_evidence_count: 20,
            min_observation_secs: 7200,
            ..MeaningConfig::default()
        };
        let tracker = ConvergenceTracker::from_config(&config);
        assert!((tracker.variance_threshold - 0.1).abs() < f64::EPSILON);
        assert_eq!(tracker.min_evidence_count, 20);
        assert_eq!(tracker.min_observation_secs, 7200);
    }
}
