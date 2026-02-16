//! Self-commitment engine — orchestrates the intent→commitment pipeline.
//!
//! Manages the full lifecycle: observation tracking, intent→declaration
//! mapping, submission recording, and outcome tracking.

use chrono::{DateTime, Utc};

use maple_worldline_intent::intent::SelfRegenerationIntent;
use maple_worldline_intent::types::{IntentId, SubstrateTier};
use worldline_core::types::CommitmentId;
use worldline_runtime::gate::CommitmentDeclaration;

use crate::bridge::IntentCommitmentBridge;
use crate::error::{CommitmentError, CommitmentResult};
use crate::lifecycle::CommitmentLifecycleManager;
use crate::mapper::DeclarationMapper;
use crate::observation::ObservationPeriodTracker;
use crate::types::{CommitmentConfig, CommitmentRecord, CommitmentSummary, SelfCommitmentId};

// ── Self-Commitment Engine ──────────────────────────────────────────────

/// Orchestrates the self-commitment pipeline.
///
/// ```text
/// Intent (stabilized)
///   │
///   ▼
/// [Observation Period]  ← governance tier mandated wait
///   │
///   ▼
/// [Declaration Mapper]  ← translate intent → gate declaration
///   │
///   ▼
/// [Gate Submission]     ← caller submits to CommitmentGate
///   │
///   ▼
/// [Lifecycle Tracking]  ← approved/denied/fulfilled/failed
/// ```
pub struct SelfCommitmentEngine {
    mapper: DeclarationMapper,
    observation_tracker: ObservationPeriodTracker,
    lifecycle: CommitmentLifecycleManager,
    config: CommitmentConfig,
    /// Intents currently under observation (stored for bridge access).
    observing_intents: Vec<SelfRegenerationIntent>,
    /// Intents that completed observation and are ready.
    ready_intents: Vec<SelfRegenerationIntent>,
}

impl SelfCommitmentEngine {
    /// Create a new engine with the given mapper and config.
    pub fn new(mapper: DeclarationMapper, config: CommitmentConfig) -> Self {
        Self {
            mapper,
            observation_tracker: ObservationPeriodTracker::new(),
            lifecycle: CommitmentLifecycleManager::new(config.max_tracked_commitments),
            config,
            observing_intents: Vec::new(),
            ready_intents: Vec::new(),
        }
    }

    /// Begin the observation period for a stabilized intent.
    ///
    /// The intent will be tracked until its governance tier's observation
    /// window elapses, at which point it becomes ready for commitment.
    pub fn start_observation(&mut self, intent: SelfRegenerationIntent) {
        if self.config.require_observation_period {
            self.observation_tracker.start_observation(&intent);
            self.observing_intents.push(intent);
        } else {
            // No observation required — immediately ready
            self.ready_intents.push(intent);
        }
    }

    /// Start observation with a custom start time (for testing).
    pub fn start_observation_at(
        &mut self,
        intent: SelfRegenerationIntent,
        started_at: DateTime<Utc>,
    ) {
        self.observation_tracker
            .start_observation_at(&intent, started_at);
        self.observing_intents.push(intent);
    }

    /// Check for intents that have completed observation and promote them.
    ///
    /// Returns the IDs of newly ready intents.
    pub fn check_ready(&mut self) -> Vec<IntentId> {
        let mut newly_ready = Vec::new();
        let mut still_observing = Vec::new();

        for intent in self.observing_intents.drain(..) {
            if self.observation_tracker.check_completion(&intent.id) {
                newly_ready.push(intent.id.clone());
                self.observation_tracker.remove(&intent.id);
                self.ready_intents.push(intent);
            } else {
                still_observing.push(intent);
            }
        }

        self.observing_intents = still_observing;
        newly_ready
    }

    /// Prepare a `CommitmentDeclaration` for an intent that is ready.
    ///
    /// The caller is responsible for submitting this to the `CommitmentGate`.
    pub fn prepare_declaration(
        &self,
        intent: &SelfRegenerationIntent,
    ) -> CommitmentResult<CommitmentDeclaration> {
        // Validate readiness
        if self.config.require_observation_period
            && !self.observation_tracker.is_ready(&intent.id)
            && !self.ready_intents.iter().any(|i| i.id == intent.id)
        {
            return Err(CommitmentError::ObservationIncomplete(format!(
                "Intent {} observation period not complete",
                intent.id
            )));
        }

        if intent.confidence < self.config.min_confidence {
            return Err(CommitmentError::IntentNotReady(format!(
                "Confidence {:.2} below minimum {:.2}",
                intent.confidence, self.config.min_confidence
            )));
        }

        if self.config.require_rollback && !intent.proposal.has_rollback() {
            return Err(CommitmentError::IntentNotReady(
                "Rollback plan required but missing".into(),
            ));
        }

        self.mapper.map_intent(intent)
    }

    /// Record that a commitment was submitted to the gate.
    pub fn record_submission(
        &mut self,
        intent_id: &IntentId,
        commitment_id: CommitmentId,
        governance_tier: SubstrateTier,
    ) -> SelfCommitmentId {
        // Remove from ready list
        self.ready_intents.retain(|i| i.id != *intent_id);

        self.lifecycle
            .record_submission(intent_id.clone(), commitment_id, governance_tier)
    }

    /// Record the gate's adjudication result.
    pub fn record_gate_result(
        &mut self,
        self_commitment_id: &SelfCommitmentId,
        approved: bool,
        reason: Option<String>,
    ) {
        if approved {
            self.lifecycle.record_approval(self_commitment_id);
        } else {
            self.lifecycle.record_denial(
                self_commitment_id,
                reason.unwrap_or_else(|| "gate denied".into()),
            );
        }
    }

    /// Record the final outcome of a commitment.
    pub fn record_outcome(
        &mut self,
        self_commitment_id: &SelfCommitmentId,
        fulfilled: bool,
        reason: Option<String>,
    ) {
        if fulfilled {
            self.lifecycle.record_fulfilled(self_commitment_id);
        } else {
            self.lifecycle.record_failed(
                self_commitment_id,
                reason.unwrap_or_else(|| "execution failed".into()),
            );
        }
    }

    /// Get summary statistics.
    pub fn summary(&self) -> CommitmentSummary {
        self.lifecycle.summary()
    }

    /// Access the lifecycle manager directly.
    pub fn lifecycle(&self) -> &CommitmentLifecycleManager {
        &self.lifecycle
    }

    /// Access the observation tracker.
    pub fn observation_tracker(&self) -> &ObservationPeriodTracker {
        &self.observation_tracker
    }

    /// Number of intents currently under observation.
    pub fn observing_count(&self) -> usize {
        self.observing_intents.len()
    }

    /// Number of intents ready for commitment.
    pub fn ready_count(&self) -> usize {
        self.ready_intents.len()
    }

    /// Check concurrent commitment limit.
    pub fn can_accept_commitment(&self) -> bool {
        self.lifecycle.active_commitments().len() < self.config.max_concurrent_commitments
    }
}

impl IntentCommitmentBridge for SelfCommitmentEngine {
    fn ready_for_commitment(&self) -> Vec<&SelfRegenerationIntent> {
        self.ready_intents.iter().collect()
    }

    fn pending_observation(&self) -> Vec<&SelfRegenerationIntent> {
        self.observing_intents.iter().collect()
    }

    fn committed(&self) -> Vec<&CommitmentRecord> {
        self.lifecycle.all_records().iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use maple_worldline_intent::intent::{ImpactAssessment, ImprovementEstimate, IntentStatus};
    use maple_worldline_intent::proposal::{RegenerationProposal, RollbackPlan, RollbackStrategy};
    use maple_worldline_intent::types::{ChangeType, MeaningId, ProposalId, ReversibilityLevel};
    use worldline_core::types::IdentityMaterial;

    fn test_worldline() -> worldline_core::types::WorldlineId {
        worldline_core::types::WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn make_intent(tier: SubstrateTier, confidence: f64) -> SelfRegenerationIntent {
        SelfRegenerationIntent {
            id: IntentId::new(),
            derived_from: vec![MeaningId::new()],
            change_type: ChangeType::ConfigurationChange {
                parameter: "batch_size".into(),
                current_value: "32".into(),
                proposed_value: "64".into(),
                rationale: "improve throughput".into(),
            },
            proposal: RegenerationProposal {
                id: ProposalId::new(),
                summary: "test".into(),
                rationale: "test".into(),
                affected_components: vec!["test".into()],
                code_changes: vec![],
                required_tests: vec![],
                performance_gates: vec![],
                safety_checks: vec![],
                estimated_improvement: ImprovementEstimate {
                    metric: "latency".into(),
                    current_value: 100.0,
                    projected_value: 80.0,
                    confidence,
                    unit: "ms".into(),
                },
                risk_score: 0.1,
                rollback_plan: RollbackPlan {
                    strategy: RollbackStrategy::ConfigRestore,
                    steps: vec!["restore".into()],
                    estimated_duration_secs: 60,
                },
            },
            confidence,
            reversibility: ReversibilityLevel::FullyReversible,
            impact: ImpactAssessment {
                affected_components: vec!["test".into()],
                risk_score: 0.1,
                risk_factors: vec![],
                blast_radius: "test".into(),
            },
            governance_tier: tier,
            estimated_improvement: ImprovementEstimate {
                metric: "latency".into(),
                current_value: 100.0,
                projected_value: 80.0,
                confidence,
                unit: "ms".into(),
            },
            stabilized_at: Utc::now(),
            status: IntentStatus::Validated,
        }
    }

    fn make_engine() -> SelfCommitmentEngine {
        let mapper = DeclarationMapper::new(test_worldline());
        SelfCommitmentEngine::new(mapper, CommitmentConfig::default())
    }

    fn make_engine_no_observation() -> SelfCommitmentEngine {
        let mapper = DeclarationMapper::new(test_worldline());
        SelfCommitmentEngine::new(
            mapper,
            CommitmentConfig {
                require_observation_period: false,
                ..CommitmentConfig::default()
            },
        )
    }

    #[test]
    fn engine_starts_observation() {
        let mut engine = make_engine();
        let intent = make_intent(SubstrateTier::Tier0, 0.9);
        engine.start_observation(intent);

        assert_eq!(engine.observing_count(), 1);
        assert_eq!(engine.ready_count(), 0);
    }

    #[test]
    fn engine_no_observation_mode() {
        let mut engine = make_engine_no_observation();
        let intent = make_intent(SubstrateTier::Tier0, 0.9);
        engine.start_observation(intent);

        assert_eq!(engine.observing_count(), 0);
        assert_eq!(engine.ready_count(), 1);
    }

    #[test]
    fn engine_check_ready_promotes() {
        let mut engine = make_engine();
        let intent = make_intent(SubstrateTier::Tier0, 0.9);
        let intent_id = intent.id.clone();

        // Start observation 2 hours ago (past 30min Tier0 window)
        let two_hours_ago = Utc::now() - Duration::hours(2);
        engine.start_observation_at(intent, two_hours_ago);

        let ready = engine.check_ready();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], intent_id);
        assert_eq!(engine.ready_count(), 1);
        assert_eq!(engine.observing_count(), 0);
    }

    #[test]
    fn engine_prepare_declaration() {
        let mut engine = make_engine_no_observation();
        let intent = make_intent(SubstrateTier::Tier0, 0.9);
        engine.start_observation(intent.clone());

        let decl = engine.prepare_declaration(&intent);
        assert!(decl.is_ok());
    }

    #[test]
    fn engine_rejects_low_confidence() {
        let mut engine = make_engine_no_observation();
        let intent = make_intent(SubstrateTier::Tier0, 0.5); // below 0.8 threshold
        engine.start_observation(intent.clone());

        let result = engine.prepare_declaration(&intent);
        assert!(result.is_err());
    }

    #[test]
    fn engine_full_lifecycle() {
        let mut engine = make_engine_no_observation();
        let intent = make_intent(SubstrateTier::Tier0, 0.9);
        let intent_id = intent.id.clone();
        engine.start_observation(intent);

        // Submit
        let self_cmt_id =
            engine.record_submission(&intent_id, CommitmentId::new(), SubstrateTier::Tier0);
        assert_eq!(engine.ready_count(), 0); // removed from ready

        // Approve
        engine.record_gate_result(&self_cmt_id, true, None);

        // Fulfill
        engine.record_outcome(&self_cmt_id, true, None);

        let summary = engine.summary();
        assert_eq!(summary.fulfilled, 1);
    }

    #[test]
    fn engine_bridge_implementation() {
        let mut engine = make_engine_no_observation();
        let intent = make_intent(SubstrateTier::Tier0, 0.9);
        engine.start_observation(intent);

        assert_eq!(engine.ready_for_commitment().len(), 1);
        assert_eq!(engine.pending_observation().len(), 0);
        assert_eq!(engine.committed().len(), 0);
    }

    #[test]
    fn engine_concurrent_limit() {
        let mut engine = make_engine_no_observation();

        for _ in 0..3 {
            let intent = make_intent(SubstrateTier::Tier0, 0.9);
            let intent_id = intent.id.clone();
            engine.start_observation(intent);
            engine.record_submission(&intent_id, CommitmentId::new(), SubstrateTier::Tier0);
        }

        assert!(!engine.can_accept_commitment()); // at limit
    }
}
