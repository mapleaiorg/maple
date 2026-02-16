//! Self-consequence engine — orchestrates execution of approved self-modifications.
//!
//! The engine coordinates:
//! 1. Validation that the commitment is approved
//! 2. Execution of the regeneration proposal
//! 3. Receipt generation for successful executions
//! 4. Rollback on failure (if configured)
//! 5. Observation feedback to close the self-producing cycle

use std::collections::VecDeque;

use maple_worldline_commitment::types::{
    CommitmentLifecycleStatus, CommitmentRecord, SelfCommitmentId,
};
use maple_worldline_intent::proposal::RegenerationProposal;
use maple_worldline_intent::types::{IntentId, SubstrateTier};
use maple_worldline_observation::events::ObservationMetadata;
use maple_worldline_observation::SelfObservationEvent;

use crate::bridge::CommitmentConsequenceBridge;
use crate::error::{ConsequenceError, ConsequenceResult};
use crate::executor::ConsequenceExecutor;
use crate::feedback::ObservationFeedback;
use crate::receipt::ExecutionReceipt;
use crate::rollback::RollbackExecutor;
use crate::types::{
    ConsequenceConfig, ConsequenceRecord, ConsequenceStatus, ConsequenceSummary, SelfConsequenceId,
};

/// The self-consequence engine.
///
/// Orchestrates the full lifecycle of consequence execution:
/// validate → execute → receipt → feedback → (rollback on failure).
pub struct SelfConsequenceEngine {
    /// Executor for applying modifications.
    executor: Box<dyn ConsequenceExecutor>,
    /// Executor for rolling back failed modifications.
    rollback_executor: Box<dyn RollbackExecutor>,
    /// Feedback generator for observation events.
    feedback: ObservationFeedback,
    /// Bounded FIFO queue of consequence records.
    records: VecDeque<ConsequenceRecord>,
    /// Engine configuration.
    config: ConsequenceConfig,
}

impl SelfConsequenceEngine {
    /// Create a new self-consequence engine.
    pub fn new(
        executor: Box<dyn ConsequenceExecutor>,
        rollback_executor: Box<dyn RollbackExecutor>,
    ) -> Self {
        Self {
            executor,
            rollback_executor,
            feedback: ObservationFeedback::new(),
            records: VecDeque::new(),
            config: ConsequenceConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(mut self, config: ConsequenceConfig) -> Self {
        self.config = config;
        self
    }

    /// Execute an approved commitment's regeneration proposal.
    ///
    /// Full pipeline:
    /// 1. Validate that commitment is approved
    /// 2. Create consequence record
    /// 3. Execute the proposal
    /// 4. On success: generate receipt, mark succeeded
    /// 5. On failure: attempt rollback (if configured), mark failed/rolled-back
    /// 6. Store record
    pub fn execute_approved(
        &mut self,
        commitment_record: &CommitmentRecord,
        proposal: &RegenerationProposal,
    ) -> ConsequenceResult<SelfConsequenceId> {
        // 1. Validate that commitment is approved
        if !matches!(
            commitment_record.status,
            CommitmentLifecycleStatus::Approved
        ) {
            return Err(ConsequenceError::CommitmentNotApproved(format!(
                "commitment {} is in state '{}', expected 'approved'",
                commitment_record.id, commitment_record.status
            )));
        }

        // 2. Create consequence record
        let mut record = ConsequenceRecord::new(
            commitment_record.id.clone(),
            commitment_record.intent_id.clone(),
            commitment_record.governance_tier.clone(),
        );
        let consequence_id = record.id.clone();

        // 3. Mark as executing
        record.mark_executing();

        // 4. Execute the proposal
        match self.executor.execute(proposal) {
            Ok(result) if result.success => {
                // Generate receipt
                let receipt = ExecutionReceipt::new(
                    consequence_id.clone(),
                    commitment_record.id.clone(),
                    commitment_record.intent_id.clone(),
                    commitment_record.governance_tier.clone(),
                    result.tests_passed,
                    format!(
                        "Executed '{}': {} tests passed in {}ms",
                        proposal.summary, result.tests_passed, result.duration_ms
                    ),
                );
                record.mark_succeeded(receipt, result.tests_passed);
            }
            Ok(result) => {
                // Execution returned but marked as not successful
                record.mark_failed(
                    format!("Execution completed but not successful: {}", result.output),
                    result.tests_passed,
                    result.tests_failed,
                );
                self.attempt_rollback(&mut record, proposal);
            }
            Err(ConsequenceError::ExecutionFailed(reason)) => {
                record.mark_failed(reason, 0, 0);
                self.attempt_rollback(&mut record, proposal);
            }
            Err(e) => {
                record.mark_failed(e.to_string(), 0, 0);
                self.attempt_rollback(&mut record, proposal);
            }
        }

        // 5. Store record (with FIFO eviction)
        self.store_record(record);

        Ok(consequence_id)
    }

    /// Record an external/manual outcome (not executed by this engine).
    pub fn record_external_outcome(
        &mut self,
        commitment_id: &SelfCommitmentId,
        intent_id: &IntentId,
        tier: SubstrateTier,
        succeeded: bool,
        reason: Option<String>,
    ) -> SelfConsequenceId {
        let mut record =
            ConsequenceRecord::new(commitment_id.clone(), intent_id.clone(), tier.clone());
        let consequence_id = record.id.clone();

        record.mark_executing();
        if succeeded {
            let receipt = ExecutionReceipt::new(
                consequence_id.clone(),
                commitment_id.clone(),
                intent_id.clone(),
                tier,
                0,
                reason.unwrap_or_else(|| "External execution succeeded".into()),
            );
            record.mark_succeeded(receipt, 0);
        } else {
            record.mark_failed(
                reason.unwrap_or_else(|| "External execution failed".into()),
                0,
                0,
            );
        }

        self.store_record(record);
        consequence_id
    }

    /// Get the observation feedback event for a consequence.
    pub fn get_feedback(
        &self,
        consequence_id: &SelfConsequenceId,
    ) -> Option<(SelfObservationEvent, ObservationMetadata)> {
        self.find(consequence_id)
            .and_then(|record| self.feedback.generate_feedback(record))
    }

    /// Find a consequence record by ID.
    pub fn find(&self, id: &SelfConsequenceId) -> Option<&ConsequenceRecord> {
        self.records.iter().find(|r| r.id == *id)
    }

    /// Find a consequence record by its commitment ID.
    pub fn find_by_commitment(&self, id: &SelfCommitmentId) -> Option<&ConsequenceRecord> {
        self.records.iter().find(|r| r.self_commitment_id == *id)
    }

    /// Get summary statistics for all tracked consequences.
    pub fn summary(&self) -> ConsequenceSummary {
        let mut summary = ConsequenceSummary::default();
        summary.total = self.records.len();
        for record in &self.records {
            match &record.status {
                ConsequenceStatus::Pending => summary.pending += 1,
                ConsequenceStatus::Executing => summary.pending += 1,
                ConsequenceStatus::Succeeded => summary.succeeded += 1,
                ConsequenceStatus::Failed(_) => summary.failed += 1,
                ConsequenceStatus::RolledBack(_) => summary.rolled_back += 1,
            }
        }
        summary
    }

    /// Get all tracked consequence records.
    pub fn all_records(&self) -> &VecDeque<ConsequenceRecord> {
        &self.records
    }

    /// Attempt rollback if configured for auto-rollback.
    fn attempt_rollback(&self, record: &mut ConsequenceRecord, proposal: &RegenerationProposal) {
        if !self.config.auto_rollback_on_failure {
            return;
        }

        if !proposal.has_rollback() {
            return;
        }

        match self.rollback_executor.rollback(&proposal.rollback_plan) {
            Ok(result) if result.success => {
                record.mark_rolled_back(format!(
                    "Rolled back via {}: {}/{} steps",
                    proposal.rollback_plan.strategy, result.steps_executed, result.total_steps,
                ));
            }
            Ok(result) => {
                record.rollback_attempted = true;
                tracing::warn!("Rollback completed but not successful: {}", result.output);
            }
            Err(e) => {
                record.rollback_attempted = true;
                tracing::error!("Rollback failed: {}", e);
            }
        }
    }

    /// Store a record with FIFO eviction if at capacity.
    fn store_record(&mut self, record: ConsequenceRecord) {
        if self.records.len() >= self.config.max_tracked_consequences {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }
}

impl CommitmentConsequenceBridge for SelfConsequenceEngine {
    fn approved_for_execution(&self) -> Vec<&CommitmentRecord> {
        // The engine doesn't store commitment records directly;
        // the commitment engine owns those. Return empty.
        vec![]
    }

    fn pending_execution(&self) -> Vec<&ConsequenceRecord> {
        self.records
            .iter()
            .filter(|r| !r.status.is_terminal())
            .collect()
    }

    fn completed(&self) -> Vec<&ConsequenceRecord> {
        self.records
            .iter()
            .filter(|r| r.status.is_terminal())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::SimulatedExecutor;
    use crate::rollback::SimulatedRollbackExecutor;
    use maple_worldline_commitment::types::{
        CommitmentLifecycleStatus, CommitmentRecord as CmtRecord, SelfCommitmentId,
    };
    use maple_worldline_intent::intent::ImprovementEstimate;
    use maple_worldline_intent::proposal::*;
    use maple_worldline_intent::types::MeaningId;
    use maple_worldline_intent::types::{CodeChangeType, IntentId, ProposalId, SubstrateTier};

    fn make_approved_commitment() -> CmtRecord {
        let now = chrono::Utc::now();
        CmtRecord {
            id: SelfCommitmentId::new(),
            intent_id: IntentId::new(),
            commitment_id: None,
            governance_tier: SubstrateTier::Tier0,
            observation_start: now,
            observation_required_secs: 1800,
            status: CommitmentLifecycleStatus::Approved,
            created_at: now,
            resolved_at: None,
        }
    }

    fn make_test_proposal() -> RegenerationProposal {
        RegenerationProposal {
            id: ProposalId::new(),
            summary: "Optimize config loading".into(),
            rationale: "Reduce startup time".into(),
            affected_components: vec!["config".into()],
            code_changes: vec![CodeChangeSpec {
                file_path: "src/config.rs".into(),
                change_type: CodeChangeType::ModifyFunction {
                    function_name: "load".into(),
                },
                description: "Cache config".into(),
                affected_regions: vec!["load()".into()],
                provenance: vec![MeaningId::new()],
            }],
            required_tests: vec![
                TestSpec {
                    name: "test_load".into(),
                    description: "Verify loading".into(),
                    test_type: TestType::Unit,
                },
                TestSpec {
                    name: "test_cache".into(),
                    description: "Verify caching".into(),
                    test_type: TestType::Integration,
                },
            ],
            performance_gates: vec![],
            safety_checks: vec![],
            estimated_improvement: ImprovementEstimate {
                metric: "startup_time".into(),
                current_value: 500.0,
                projected_value: 200.0,
                confidence: 0.9,
                unit: "ms".into(),
            },
            risk_score: 0.1,
            rollback_plan: RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec!["git revert HEAD".into()],
                estimated_duration_secs: 60,
            },
        }
    }

    #[test]
    fn execute_approved_success() {
        let mut engine = SelfConsequenceEngine::new(
            Box::new(SimulatedExecutor::new(true)),
            Box::new(SimulatedRollbackExecutor::new(true)),
        );

        let commitment = make_approved_commitment();
        let proposal = make_test_proposal();
        let id = engine.execute_approved(&commitment, &proposal).unwrap();

        let record = engine.find(&id).unwrap();
        assert!(record.status.is_success());
        assert!(record.receipt.is_some());
        assert_eq!(record.tests_passed, 2);
        assert!(record.receipt.as_ref().unwrap().verify());
    }

    #[test]
    fn execute_approved_failure_with_rollback() {
        let mut engine = SelfConsequenceEngine::new(
            Box::new(SimulatedExecutor::new(false)),
            Box::new(SimulatedRollbackExecutor::new(true)),
        );

        let commitment = make_approved_commitment();
        let proposal = make_test_proposal();
        let id = engine.execute_approved(&commitment, &proposal).unwrap();

        let record = engine.find(&id).unwrap();
        assert!(matches!(record.status, ConsequenceStatus::RolledBack(_)));
        assert!(record.rollback_attempted);
    }

    #[test]
    fn reject_non_approved_commitment() {
        let mut engine = SelfConsequenceEngine::new(
            Box::new(SimulatedExecutor::new(true)),
            Box::new(SimulatedRollbackExecutor::new(true)),
        );

        let mut commitment = make_approved_commitment();
        commitment.status = CommitmentLifecycleStatus::PendingObservation;
        let proposal = make_test_proposal();

        let result = engine.execute_approved(&commitment, &proposal);
        assert!(matches!(
            result,
            Err(ConsequenceError::CommitmentNotApproved(_))
        ));
    }

    #[test]
    fn record_external_outcome_success() {
        let mut engine = SelfConsequenceEngine::new(
            Box::new(SimulatedExecutor::new(true)),
            Box::new(SimulatedRollbackExecutor::new(true)),
        );

        let cmt_id = SelfCommitmentId::new();
        let intent_id = IntentId::new();
        let id = engine.record_external_outcome(
            &cmt_id,
            &intent_id,
            SubstrateTier::Tier1,
            true,
            Some("Manual verification passed".into()),
        );

        let record = engine.find(&id).unwrap();
        assert!(record.status.is_success());
        assert!(record.receipt.is_some());
    }

    #[test]
    fn record_external_outcome_failure() {
        let mut engine = SelfConsequenceEngine::new(
            Box::new(SimulatedExecutor::new(true)),
            Box::new(SimulatedRollbackExecutor::new(true)),
        );

        let id = engine.record_external_outcome(
            &SelfCommitmentId::new(),
            &IntentId::new(),
            SubstrateTier::Tier0,
            false,
            Some("Manual test failed".into()),
        );

        let record = engine.find(&id).unwrap();
        assert!(matches!(record.status, ConsequenceStatus::Failed(_)));
    }

    #[test]
    fn feedback_generation() {
        let mut engine = SelfConsequenceEngine::new(
            Box::new(SimulatedExecutor::new(true)),
            Box::new(SimulatedRollbackExecutor::new(true)),
        );

        let commitment = make_approved_commitment();
        let proposal = make_test_proposal();
        let id = engine.execute_approved(&commitment, &proposal).unwrap();

        let feedback = engine.get_feedback(&id);
        assert!(feedback.is_some());

        let (event, _metadata) = feedback.unwrap();
        match event {
            SelfObservationEvent::GateSubmission { approved, .. } => {
                assert!(approved);
            }
            _ => panic!("Expected GateSubmission"),
        }
    }

    #[test]
    fn summary_statistics() {
        let mut engine = SelfConsequenceEngine::new(
            Box::new(SimulatedExecutor::new(true)),
            Box::new(SimulatedRollbackExecutor::new(true)),
        );

        // Execute two successful consequences
        for _ in 0..2 {
            let commitment = make_approved_commitment();
            let proposal = make_test_proposal();
            engine.execute_approved(&commitment, &proposal).unwrap();
        }

        // Record one external failure
        engine.record_external_outcome(
            &SelfCommitmentId::new(),
            &IntentId::new(),
            SubstrateTier::Tier0,
            false,
            None,
        );

        let summary = engine.summary();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn fifo_eviction_at_capacity() {
        let config = ConsequenceConfig {
            max_tracked_consequences: 3,
            ..ConsequenceConfig::default()
        };
        let mut engine = SelfConsequenceEngine::new(
            Box::new(SimulatedExecutor::new(true)),
            Box::new(SimulatedRollbackExecutor::new(true)),
        )
        .with_config(config);

        // Fill to capacity
        let mut ids = Vec::new();
        for _ in 0..4 {
            let commitment = make_approved_commitment();
            let proposal = make_test_proposal();
            let id = engine.execute_approved(&commitment, &proposal).unwrap();
            ids.push(id);
        }

        // First record should have been evicted
        assert!(engine.find(&ids[0]).is_none());
        // Last three should still be present
        assert!(engine.find(&ids[1]).is_some());
        assert!(engine.find(&ids[2]).is_some());
        assert!(engine.find(&ids[3]).is_some());
        assert_eq!(engine.all_records().len(), 3);
    }
}
