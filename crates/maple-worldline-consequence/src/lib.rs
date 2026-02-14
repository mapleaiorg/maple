//! # maple-worldline-consequence
//!
//! Self-Consequence Engine for the WorldLine Self-Producing Substrate.
//!
//! This crate is the **final stage** of the self-producing substrate cycle.
//! It executes approved self-modifications, records outcomes with cryptographic
//! receipts, and generates observation events that feed back into the
//! observation layer — closing the cycle.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                 SELF-PRODUCING SUBSTRATE CYCLE                      │
//! │                                                                     │
//! │  ┌──────────────┐   ┌──────────┐   ┌────────┐   ┌────────────┐    │
//! │  │ Observation   │──▶│ Meaning  │──▶│ Intent │──▶│ Commitment │    │
//! │  │ (Prompt 11)   │   │(Prompt 13│   │(P.14)  │   │ (P.15)     │    │
//! │  └───────▲───────┘   └──────────┘   └────────┘   └─────┬──────┘    │
//! │          │                                              │           │
//! │          │    ┌──────────────────────────────────┐      │ approved  │
//! │          │    │      CONSEQUENCE ENGINE          │      │           │
//! │          │    │         (This Crate)             │◀─────┘           │
//! │          │    │                                  │                  │
//! │          │    │  CommitmentRecord + Proposal     │                  │
//! │          │    │         │                        │                  │
//! │          │    │         ▼                        │                  │
//! │          │    │  ┌─────────────┐                 │                  │
//! │          │    │  │  Executor   │ apply changes   │                  │
//! │          │    │  └──────┬──────┘                 │                  │
//! │          │    │         │                        │                  │
//! │          │    │    success?──┐                   │                  │
//! │          │    │    │yes      │no                 │                  │
//! │          │    │    ▼         ▼                   │                  │
//! │          │    │  Receipt   Rollback              │                  │
//! │          │    │    │         │                   │                  │
//! │          │    │    ▼         ▼                   │                  │
//! │          │    │  ┌─────────────┐                 │                  │
//! │          │    │  │  Feedback   │ → observation   │                  │
//! │          │    │  └─────────────┘                 │                  │
//! │          │    └──────────────────────────────────┘                  │
//! │          │                     │                                    │
//! │          └─────────────────────┘ SelfObservationEvent               │
//! │                   (cycle closes)                                    │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Components
//!
//! - [`SelfConsequenceEngine`]: Main engine orchestrating execution, receipts, feedback
//! - [`ConsequenceExecutor`]: Trait for executing regeneration proposals
//! - [`ExecutionReceipt`]: Cryptographic receipt proving execution (SHA-256)
//! - [`ObservationFeedback`]: Generates observation events from outcomes
//! - [`RollbackExecutor`]: Trait for rolling back failed modifications
//! - [`CommitmentConsequenceBridge`]: Trait bridging commitment → consequence
//!
//! ## Invariants
//!
//! - **I.CSQ-1**: Only approved commitments may produce consequences
//! - **I.CSQ-2**: Every successful execution produces a verifiable receipt
//! - **I.CSQ-3**: Every terminal consequence generates observation feedback
//! - **I.CSQ-4**: Memory bounded (256 records default, FIFO eviction)

#![deny(unsafe_code)]

pub mod bridge;
pub mod engine;
pub mod error;
pub mod executor;
pub mod feedback;
pub mod receipt;
pub mod rollback;
pub mod types;

// Re-exports for convenience
pub use bridge::CommitmentConsequenceBridge;
pub use engine::SelfConsequenceEngine;
pub use error::{ConsequenceError, ConsequenceResult};
pub use executor::{ConsequenceExecutor, ExecutionResult, SimulatedExecutor};
pub use feedback::ObservationFeedback;
pub use receipt::ExecutionReceipt;
pub use rollback::{RollbackExecutor, RollbackResult, SimulatedRollbackExecutor};
pub use types::{
    ConsequenceConfig, ConsequenceRecord, ConsequenceStatus, ConsequenceSummary,
    SelfConsequenceId,
};

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_commitment::types::{
        CommitmentLifecycleStatus, CommitmentRecord as CmtRecord, SelfCommitmentId,
    };
    use maple_worldline_intent::intent::ImprovementEstimate;
    use maple_worldline_intent::proposal::*;
    use maple_worldline_intent::types::{CodeChangeType, IntentId, ProposalId, SubstrateTier};
    use maple_worldline_intent::types::MeaningId;
    use maple_worldline_observation::SelfObservationEvent;

    fn make_approved_commitment(tier: SubstrateTier) -> CmtRecord {
        let now = chrono::Utc::now();
        CmtRecord {
            id: SelfCommitmentId::new(),
            intent_id: IntentId::new(),
            commitment_id: None,
            governance_tier: tier.clone(),
            observation_start: now,
            observation_required_secs: tier.min_observation_secs(),
            status: CommitmentLifecycleStatus::Approved,
            created_at: now,
            resolved_at: None,
        }
    }

    fn make_proposal(summary: &str, num_tests: usize) -> RegenerationProposal {
        let tests: Vec<TestSpec> = (0..num_tests)
            .map(|i| TestSpec {
                name: format!("test_{}", i),
                description: format!("Test {}", i),
                test_type: TestType::Unit,
            })
            .collect();

        RegenerationProposal {
            id: ProposalId::new(),
            summary: summary.into(),
            rationale: "Automated regeneration".into(),
            affected_components: vec!["target".into()],
            code_changes: vec![CodeChangeSpec {
                file_path: "src/target.rs".into(),
                change_type: CodeChangeType::ModifyFunction {
                    function_name: "optimize".into(),
                },
                description: "Apply optimization".into(),
                affected_regions: vec!["optimize()".into()],
                provenance: vec![MeaningId::new()],
            }],
            required_tests: tests,
            performance_gates: vec![],
            safety_checks: vec![],
            estimated_improvement: ImprovementEstimate {
                metric: "throughput".into(),
                current_value: 100.0,
                projected_value: 150.0,
                confidence: 0.85,
                unit: "ops/s".into(),
            },
            risk_score: 0.15,
            rollback_plan: RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec!["git revert HEAD".into(), "cargo test".into()],
                estimated_duration_secs: 120,
            },
        }
    }

    #[test]
    fn integration_full_cycle_success() {
        // Complete cycle: approved commitment → execute → receipt → feedback
        let mut engine = SelfConsequenceEngine::new(
            Box::new(SimulatedExecutor::new(true)),
            Box::new(SimulatedRollbackExecutor::new(true)),
        );

        let commitment = make_approved_commitment(SubstrateTier::Tier0);
        let proposal = make_proposal("Optimize config caching", 3);

        // Execute
        let csq_id = engine.execute_approved(&commitment, &proposal).unwrap();

        // Verify record
        let record = engine.find(&csq_id).unwrap();
        assert!(record.status.is_success());
        assert_eq!(record.tests_passed, 3);
        assert_eq!(record.governance_tier, SubstrateTier::Tier0);

        // Verify receipt
        let receipt = record.receipt.as_ref().unwrap();
        assert!(receipt.verify());
        assert_eq!(receipt.governance_tier, SubstrateTier::Tier0);
        assert_eq!(receipt.tests_passed, 3);

        // Verify feedback (observation event)
        let feedback = engine.get_feedback(&csq_id);
        assert!(feedback.is_some());
        let (event, _metadata) = feedback.unwrap();
        match event {
            SelfObservationEvent::GateSubmission { approved, .. } => {
                assert!(approved, "Feedback should indicate success");
            }
            _ => panic!("Expected GateSubmission feedback event"),
        }

        // Verify bridge
        assert_eq!(engine.completed().len(), 1);
        assert_eq!(engine.pending_execution().len(), 0);
    }

    #[test]
    fn integration_failure_with_rollback_and_feedback() {
        // Failed execution → auto-rollback → failure feedback
        let mut engine = SelfConsequenceEngine::new(
            Box::new(SimulatedExecutor::new(false)),
            Box::new(SimulatedRollbackExecutor::new(true)),
        );

        let commitment = make_approved_commitment(SubstrateTier::Tier1);
        let proposal = make_proposal("Risky operator change", 4);

        let csq_id = engine.execute_approved(&commitment, &proposal).unwrap();

        // Verify record is rolled back
        let record = engine.find(&csq_id).unwrap();
        assert!(matches!(record.status, ConsequenceStatus::RolledBack(_)));
        assert!(record.rollback_attempted);
        assert!(record.receipt.is_none());

        // Verify feedback indicates failure
        let feedback = engine.get_feedback(&csq_id);
        assert!(feedback.is_some());
        let (event, _) = feedback.unwrap();
        match event {
            SelfObservationEvent::GateSubmission { approved, .. } => {
                assert!(!approved, "Feedback should indicate failure");
            }
            _ => panic!("Expected GateSubmission feedback event"),
        }

        // Summary reflects the failure
        let summary = engine.summary();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.rolled_back, 1);
        assert_eq!(summary.succeeded, 0);
    }

    #[test]
    fn integration_multiple_consequences_summary() {
        let mut engine = SelfConsequenceEngine::new(
            Box::new(SimulatedExecutor::new(true)),
            Box::new(SimulatedRollbackExecutor::new(true)),
        );

        // Execute 3 successful modifications at different tiers
        for (i, tier) in [SubstrateTier::Tier0, SubstrateTier::Tier1, SubstrateTier::Tier2]
            .iter()
            .enumerate()
        {
            let commitment = make_approved_commitment(tier.clone());
            let proposal = make_proposal(
                &format!("Optimization #{}", i + 1),
                i + 2,
            );
            engine.execute_approved(&commitment, &proposal).unwrap();
        }

        // Record one external failure
        engine.record_external_outcome(
            &SelfCommitmentId::new(),
            &IntentId::new(),
            SubstrateTier::Tier0,
            false,
            Some("Manual test regression".into()),
        );

        // Verify summary
        let summary = engine.summary();
        assert_eq!(summary.total, 4);
        assert_eq!(summary.succeeded, 3);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.rolled_back, 0);
        assert_eq!(summary.pending, 0);

        // Verify all records have feedback
        for record in engine.all_records() {
            let fb = engine.get_feedback(&record.id);
            assert!(fb.is_some(), "Terminal record should generate feedback");
        }
    }
}
