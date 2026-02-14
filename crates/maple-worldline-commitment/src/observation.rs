//! Observation period tracker — enforces governance tier observation windows.
//!
//! Each substrate tier mandates a minimum observation period before an
//! intent can be committed:
//! - Tier 0 (config): 30 minutes
//! - Tier 1 (operator): 1 hour
//! - Tier 2 (kernel): 24 hours
//! - Tier 3 (architecture): 72 hours

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};

use maple_worldline_intent::intent::SelfRegenerationIntent;
use maple_worldline_intent::types::{IntentId, SubstrateTier};

// ── Observation Record ──────────────────────────────────────────────────

/// Record tracking the observation period for a single intent.
#[derive(Clone, Debug)]
pub struct ObservationRecord {
    /// The intent being observed.
    pub intent_id: IntentId,
    /// Governance tier determining the observation duration.
    pub governance_tier: SubstrateTier,
    /// When the observation period started.
    pub started_at: DateTime<Utc>,
    /// Required observation duration (seconds).
    pub required_secs: u64,
    /// Whether the observation has been marked complete.
    pub completed: bool,
}

impl ObservationRecord {
    /// Remaining time in the observation period.
    pub fn time_remaining(&self) -> Option<Duration> {
        if self.completed {
            return None;
        }
        let elapsed = (Utc::now() - self.started_at).num_seconds().max(0) as u64;
        if elapsed >= self.required_secs {
            None // Ready
        } else {
            Some(Duration::seconds((self.required_secs - elapsed) as i64))
        }
    }

    /// Whether the observation period has elapsed.
    pub fn is_elapsed(&self) -> bool {
        let elapsed = (Utc::now() - self.started_at).num_seconds().max(0) as u64;
        elapsed >= self.required_secs
    }
}

// ── Observation Period Tracker ──────────────────────────────────────────

/// Tracks observation periods for intents awaiting commitment.
pub struct ObservationPeriodTracker {
    records: HashMap<IntentId, ObservationRecord>,
}

impl Default for ObservationPeriodTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ObservationPeriodTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    /// Start the observation period for an intent.
    ///
    /// Uses the intent's governance tier to determine the required duration.
    pub fn start_observation(&mut self, intent: &SelfRegenerationIntent) {
        let record = ObservationRecord {
            intent_id: intent.id.clone(),
            governance_tier: intent.governance_tier.clone(),
            started_at: Utc::now(),
            required_secs: intent.governance_tier.min_observation_secs(),
            completed: false,
        };
        self.records.insert(intent.id.clone(), record);
    }

    /// Start observation with a custom start time (for testing).
    pub fn start_observation_at(
        &mut self,
        intent: &SelfRegenerationIntent,
        started_at: DateTime<Utc>,
    ) {
        let record = ObservationRecord {
            intent_id: intent.id.clone(),
            governance_tier: intent.governance_tier.clone(),
            started_at,
            required_secs: intent.governance_tier.min_observation_secs(),
            completed: false,
        };
        self.records.insert(intent.id.clone(), record);
    }

    /// Check if an intent's observation is complete and mark it.
    pub fn check_completion(&mut self, intent_id: &IntentId) -> bool {
        if let Some(record) = self.records.get_mut(intent_id) {
            if !record.completed && record.is_elapsed() {
                record.completed = true;
            }
            record.completed
        } else {
            false
        }
    }

    /// Whether an intent's observation is ready (completed).
    pub fn is_ready(&self, intent_id: &IntentId) -> bool {
        self.records
            .get(intent_id)
            .map(|r| r.completed || r.is_elapsed())
            .unwrap_or(false)
    }

    /// Get all pending (not yet complete) observations.
    pub fn pending_observations(&self) -> Vec<&ObservationRecord> {
        self.records
            .values()
            .filter(|r| !r.completed && !r.is_elapsed())
            .collect()
    }

    /// Get the remaining time for an intent's observation.
    pub fn time_remaining(&self, intent_id: &IntentId) -> Option<Duration> {
        self.records.get(intent_id).and_then(|r| r.time_remaining())
    }

    /// Get a record by intent ID.
    pub fn get(&self, intent_id: &IntentId) -> Option<&ObservationRecord> {
        self.records.get(intent_id)
    }

    /// Remove a record (after commitment or abandonment).
    pub fn remove(&mut self, intent_id: &IntentId) {
        self.records.remove(intent_id);
    }

    /// Number of tracked observations.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the tracker is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_intent::intent::{
        ImpactAssessment, ImprovementEstimate, IntentStatus,
    };
    use maple_worldline_intent::proposal::{
        RegenerationProposal, RollbackPlan, RollbackStrategy,
    };
    use maple_worldline_intent::types::{
        ChangeType, MeaningId, ProposalId, ReversibilityLevel,
    };

    fn make_intent(tier: SubstrateTier) -> SelfRegenerationIntent {
        SelfRegenerationIntent {
            id: IntentId::new(),
            derived_from: vec![MeaningId::new()],
            change_type: ChangeType::ConfigurationChange {
                parameter: "test".into(),
                current_value: "1".into(),
                proposed_value: "2".into(),
                rationale: "test".into(),
            },
            proposal: RegenerationProposal {
                id: ProposalId::new(),
                summary: "test".into(),
                rationale: "test".into(),
                affected_components: vec![],
                code_changes: vec![],
                required_tests: vec![],
                performance_gates: vec![],
                safety_checks: vec![],
                estimated_improvement: ImprovementEstimate {
                    metric: "test".into(),
                    current_value: 100.0,
                    projected_value: 80.0,
                    confidence: 0.9,
                    unit: "ms".into(),
                },
                risk_score: 0.1,
                rollback_plan: RollbackPlan {
                    strategy: RollbackStrategy::ConfigRestore,
                    steps: vec!["restore".into()],
                    estimated_duration_secs: 60,
                },
            },
            confidence: 0.9,
            reversibility: ReversibilityLevel::FullyReversible,
            impact: ImpactAssessment {
                affected_components: vec!["test".into()],
                risk_score: 0.1,
                risk_factors: vec![],
                blast_radius: "test".into(),
            },
            governance_tier: tier,
            estimated_improvement: ImprovementEstimate {
                metric: "test".into(),
                current_value: 100.0,
                projected_value: 80.0,
                confidence: 0.9,
                unit: "ms".into(),
            },
            stabilized_at: Utc::now(),
            status: IntentStatus::Validated,
        }
    }

    #[test]
    fn start_and_check_observation() {
        let mut tracker = ObservationPeriodTracker::new();
        let intent = make_intent(SubstrateTier::Tier0);
        tracker.start_observation(&intent);

        assert_eq!(tracker.len(), 1);
        // Just started — should not be ready yet (30 min window)
        assert!(!tracker.is_ready(&intent.id));
    }

    #[test]
    fn observation_with_past_start_time_is_ready() {
        let mut tracker = ObservationPeriodTracker::new();
        let intent = make_intent(SubstrateTier::Tier0);

        // Start 2 hours ago (well past 30 min observation)
        let two_hours_ago = Utc::now() - Duration::hours(2);
        tracker.start_observation_at(&intent, two_hours_ago);

        assert!(tracker.is_ready(&intent.id));
        assert!(tracker.check_completion(&intent.id));
    }

    #[test]
    fn pending_observations_excludes_completed() {
        let mut tracker = ObservationPeriodTracker::new();
        let intent1 = make_intent(SubstrateTier::Tier0);
        let intent2 = make_intent(SubstrateTier::Tier0);

        // intent1: started 2 hours ago (complete)
        let two_hours_ago = Utc::now() - Duration::hours(2);
        tracker.start_observation_at(&intent1, two_hours_ago);
        tracker.check_completion(&intent1.id);

        // intent2: just started (pending)
        tracker.start_observation(&intent2);

        assert_eq!(tracker.pending_observations().len(), 1);
    }

    #[test]
    fn time_remaining_computation() {
        let mut tracker = ObservationPeriodTracker::new();
        let intent = make_intent(SubstrateTier::Tier0);
        tracker.start_observation(&intent);

        let remaining = tracker.time_remaining(&intent.id);
        assert!(remaining.is_some());
        // Should be approximately 30 minutes
        let secs = remaining.unwrap().num_seconds();
        assert!(secs > 0 && secs <= 1800);
    }

    #[test]
    fn remove_observation() {
        let mut tracker = ObservationPeriodTracker::new();
        let intent = make_intent(SubstrateTier::Tier0);
        tracker.start_observation(&intent);
        assert_eq!(tracker.len(), 1);

        tracker.remove(&intent.id);
        assert!(tracker.is_empty());
    }

    #[test]
    fn different_tiers_different_durations() {
        let mut tracker = ObservationPeriodTracker::new();
        let t0 = make_intent(SubstrateTier::Tier0);
        let t3 = make_intent(SubstrateTier::Tier3);

        tracker.start_observation(&t0);
        tracker.start_observation(&t3);

        let r0 = tracker.get(&t0.id).unwrap();
        let r3 = tracker.get(&t3.id).unwrap();

        assert_eq!(r0.required_secs, 1800);    // 30 min
        assert_eq!(r3.required_secs, 259200);   // 72 hr
    }
}
