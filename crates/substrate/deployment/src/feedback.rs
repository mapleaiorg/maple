//! Deployment feedback — SelfObservationEvent generation.
//!
//! Generates observation feedback events that close the WorldLine
//! self-producing substrate cycle. Terminal deployments (succeeded,
//! failed, rolled back) produce `SelfObservationEvent::GateSubmission`
//! events that feed back into the observation layer.

use std::time::Duration;

use maple_worldline_observation::events::{ObservationMetadata, SelfObservationEvent, SubsystemId};
use worldline_core::types::CommitmentId;

use crate::types::DeploymentRecord;

// ── Deployment Feedback ────────────────────────────────────────────────

/// Generates observation feedback from deployment records.
///
/// This is the critical link that closes the WorldLine cycle:
/// Observation → Meaning → Intent → Commitment → Codegen → Deployment → **Observation**
pub struct DeploymentFeedback;

impl DeploymentFeedback {
    /// Generate feedback for a terminal deployment.
    ///
    /// Returns `None` if the deployment is not yet terminal.
    /// Returns `Some((event, metadata))` for terminal deployments.
    ///
    /// Maps deployment state to observation event:
    /// - phases_completed → stages_evaluated
    /// - Succeeded → approved = true
    /// - Failed/RolledBack → approved = false
    pub fn generate_feedback(
        record: &DeploymentRecord,
    ) -> Option<(SelfObservationEvent, ObservationMetadata)> {
        if !record.status.is_terminal() {
            return None;
        }

        let duration_ms = record.duration_ms().unwrap_or(0);
        let stages_evaluated = record.phase_count() as u8;
        let approved = record.status.is_success();

        let event = SelfObservationEvent::GateSubmission {
            commitment_id: CommitmentId::new(), // Fresh ID for the observation event
            stages_evaluated,
            total_latency: Duration::from_millis(duration_ms as u64),
            approved,
        };

        let metadata = ObservationMetadata::now(SubsystemId::Custom("deployment-pipeline".into()));

        Some((event, metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DeploymentRecord;
    use maple_worldline_self_mod_gate::types::{DeploymentStrategy, SelfModTier};

    fn make_record() -> DeploymentRecord {
        DeploymentRecord::new(
            "codegen-1".into(),
            "commit-1".into(),
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            vec!["src/config.rs".into()],
        )
    }

    #[test]
    fn feedback_none_for_pending_deployment() {
        let record = make_record();
        assert!(DeploymentFeedback::generate_feedback(&record).is_none());
    }

    #[test]
    fn feedback_generated_for_succeeded_deployment() {
        let mut record = make_record();
        record.mark_in_progress();
        record.mark_succeeded();

        let feedback = DeploymentFeedback::generate_feedback(&record);
        assert!(feedback.is_some());

        let (event, metadata) = feedback.unwrap();
        if let SelfObservationEvent::GateSubmission { approved, .. } = event {
            assert!(approved);
        } else {
            panic!("Expected GateSubmission event");
        }
        assert_eq!(
            metadata.subsystem,
            SubsystemId::Custom("deployment-pipeline".into())
        );
    }

    #[test]
    fn feedback_generated_for_failed_deployment() {
        let mut record = make_record();
        record.mark_in_progress();
        record.mark_failed("compilation error".into());

        let feedback = DeploymentFeedback::generate_feedback(&record);
        assert!(feedback.is_some());

        let (event, _) = feedback.unwrap();
        if let SelfObservationEvent::GateSubmission { approved, .. } = event {
            assert!(!approved);
        } else {
            panic!("Expected GateSubmission event");
        }
    }

    #[test]
    fn feedback_generated_for_rolled_back_deployment() {
        let mut record = make_record();
        record.mark_in_progress();
        record.mark_rolled_back("regression detected".into());

        let feedback = DeploymentFeedback::generate_feedback(&record);
        assert!(feedback.is_some());

        let (event, _) = feedback.unwrap();
        if let SelfObservationEvent::GateSubmission { approved, .. } = event {
            assert!(!approved);
        } else {
            panic!("Expected GateSubmission event");
        }
    }
}
