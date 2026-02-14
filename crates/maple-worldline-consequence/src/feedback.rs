//! Observation feedback — closes the self-producing substrate cycle.
//!
//! Converts consequence outcomes into `SelfObservationEvent`s that feed back
//! into the observation layer, completing the cycle:
//!
//! ```text
//! Observation → Meaning → Intent → Commitment → Consequence
//!      ↑                                            │
//!      └────────── feedback (this module) ──────────┘
//! ```

use std::time::Duration;

use maple_mwl_types::CommitmentId;
use maple_worldline_observation::events::{ObservationMetadata, SubsystemId};
use maple_worldline_observation::SelfObservationEvent;

use crate::types::ConsequenceRecord;

/// Generates observation events from consequence outcomes.
///
/// This is the key component that closes the self-producing substrate cycle.
/// When a consequence completes (success or failure), it generates a
/// `SelfObservationEvent` that feeds back into the observation layer.
#[derive(Clone, Debug, Default)]
pub struct ObservationFeedback;

impl ObservationFeedback {
    /// Create a new observation feedback generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate a feedback observation event from a completed consequence.
    ///
    /// Returns `None` for non-terminal consequences (Pending, Executing)
    /// since they haven't produced an outcome yet.
    pub fn generate_feedback(
        &self,
        record: &ConsequenceRecord,
    ) -> Option<(SelfObservationEvent, ObservationMetadata)> {
        if !record.status.is_terminal() {
            return None;
        }

        let approved = record.status.is_success();
        let total_tests = record.tests_passed + record.tests_failed;
        let duration = record
            .duration_ms
            .map(|ms| Duration::from_millis(ms.max(0) as u64))
            .unwrap_or(Duration::from_millis(0));

        // Map consequence outcome to a GateSubmission observation event.
        // We reuse GateSubmission because it captures the key dimensions:
        // - commitment_id: provenance tracking
        // - stages_evaluated: maps to tests run
        // - total_latency: execution duration
        // - approved: success or failure
        let event = SelfObservationEvent::GateSubmission {
            commitment_id: CommitmentId::new(),
            stages_evaluated: total_tests as u8,
            total_latency: duration,
            approved,
        };

        let metadata = ObservationMetadata::now(SubsystemId::CommitmentGate);

        Some((event, metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::receipt::ExecutionReceipt;
    use crate::types::ConsequenceRecord;
    use maple_worldline_commitment::types::SelfCommitmentId;
    use maple_worldline_intent::types::{IntentId, SubstrateTier};

    #[test]
    fn feedback_for_succeeded_consequence() {
        let feedback = ObservationFeedback::new();
        let mut record = ConsequenceRecord::new(
            SelfCommitmentId::new(),
            IntentId::new(),
            SubstrateTier::Tier0,
        );
        record.mark_executing();
        let receipt = ExecutionReceipt::new(
            record.id.clone(),
            record.self_commitment_id.clone(),
            record.intent_id.clone(),
            SubstrateTier::Tier0,
            5,
            "test",
        );
        record.mark_succeeded(receipt, 5);

        let result = feedback.generate_feedback(&record);
        assert!(result.is_some());

        let (event, metadata) = result.unwrap();
        match event {
            SelfObservationEvent::GateSubmission { approved, stages_evaluated, .. } => {
                assert!(approved);
                assert_eq!(stages_evaluated, 5);
            }
            _ => panic!("Expected GateSubmission event"),
        }
        assert_eq!(metadata.subsystem, SubsystemId::CommitmentGate);
    }

    #[test]
    fn feedback_for_failed_consequence() {
        let feedback = ObservationFeedback::new();
        let mut record = ConsequenceRecord::new(
            SelfCommitmentId::new(),
            IntentId::new(),
            SubstrateTier::Tier1,
        );
        record.mark_executing();
        record.mark_failed("compilation error".into(), 3, 2);

        let result = feedback.generate_feedback(&record);
        assert!(result.is_some());

        let (event, _) = result.unwrap();
        match event {
            SelfObservationEvent::GateSubmission { approved, stages_evaluated, .. } => {
                assert!(!approved);
                assert_eq!(stages_evaluated, 5); // 3 passed + 2 failed
            }
            _ => panic!("Expected GateSubmission event"),
        }
    }

    #[test]
    fn no_feedback_for_pending_consequence() {
        let feedback = ObservationFeedback::new();
        let record = ConsequenceRecord::new(
            SelfCommitmentId::new(),
            IntentId::new(),
            SubstrateTier::Tier0,
        );
        assert!(feedback.generate_feedback(&record).is_none());
    }

    #[test]
    fn no_feedback_for_executing_consequence() {
        let feedback = ObservationFeedback::new();
        let mut record = ConsequenceRecord::new(
            SelfCommitmentId::new(),
            IntentId::new(),
            SubstrateTier::Tier0,
        );
        record.mark_executing();
        assert!(feedback.generate_feedback(&record).is_none());
    }
}
