//! # maple-worldline-commitment
//!
//! Self-Commitment: bridges the gap between stabilized intents
//! and the commitment gate's 7-stage adjudication pipeline.
//!
//! ## Architecture
//!
//! ```text
//! maple-worldline-intent              maple-worldline-commitment
//! ┌──────────────────────┐            ┌──────────────────────────────────┐
//! │  IntentStabilization │            │  SelfCommitmentEngine            │
//! │  Engine              │            │  ┌────────────────────────────┐  │
//! │  ┌────────────────┐  │            │  │ ObservationPeriodTracker   │  │
//! │  │ stabilized     │──┼──bridge──▶ │  │ (tier-based wait windows) │  │
//! │  │ intents        │  │            │  └──────────┬─────────────────┘  │
//! │  └────────────────┘  │            │             ▼                    │
//! └──────────────────────┘            │  ┌────────────────────────────┐  │
//!                                     │  │ DeclarationMapper          │  │
//!                                     │  │ (intent → declaration)     │  │
//!                                     │  └──────────┬─────────────────┘  │
//!                                     │             ▼                    │
//!                                     │  ┌────────────────────────────┐  │
//!                                     │  │ CommitmentLifecycleManager │  │
//!                                     │  │ (submitted/approved/done)  │  │
//!                                     │  └──────────────────────────────┘│
//!                                     └──────────────┬───────────────────┘
//!                                                    ▼
//!                                     maple-kernel-gate
//!                                     ┌──────────────────────────────────┐
//!                                     │  CommitmentGate (7 stages)       │
//!                                     │  → AdjudicationResult            │
//!                                     └──────────────────────────────────┘
//! ```
//!
//! ## Governance Tiers & Observation Windows
//!
//! | Tier | Scope          | Observation | Min Confidence |
//! |------|----------------|-------------|----------------|
//! | 0    | Configuration  | 30 min      | 0.70           |
//! | 1    | Operator       | 1 hour      | 0.80           |
//! | 2    | Kernel module  | 24 hours    | 0.85           |
//! | 3    | Architecture   | 72 hours    | 0.90           |

#![deny(unsafe_code)]

pub mod bridge;
pub mod engine;
pub mod error;
pub mod lifecycle;
pub mod mapper;
pub mod observation;
pub mod types;

// ── Re-exports ─────────────────────────────────────────────────────────

pub use engine::SelfCommitmentEngine;
pub use error::{CommitmentError, CommitmentResult};
pub use lifecycle::CommitmentLifecycleManager;
pub use mapper::DeclarationMapper;
pub use observation::ObservationPeriodTracker;
pub use types::{
    CommitmentConfig, CommitmentLifecycleStatus, CommitmentRecord, CommitmentSummary,
    SelfCommitmentId,
};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use maple_worldline_intent::intent::{
        ImpactAssessment, ImprovementEstimate, IntentStatus, SelfRegenerationIntent,
    };
    use maple_worldline_intent::proposal::{RegenerationProposal, RollbackPlan, RollbackStrategy};
    use maple_worldline_intent::types::{
        ChangeType, IntentId, MeaningId, ProposalId, ReversibilityLevel, SubstrateTier,
    };
    use worldline_core::types::{CommitmentId, IdentityMaterial, WorldlineId};

    use crate::bridge::IntentCommitmentBridge;

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
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
                summary: "Increase batch size".into(),
                rationale: "Suboptimal batch size detected".into(),
                affected_components: vec!["scheduler".into()],
                code_changes: vec![],
                required_tests: vec![],
                performance_gates: vec![],
                safety_checks: vec![],
                estimated_improvement: ImprovementEstimate {
                    metric: "throughput".into(),
                    current_value: 100.0,
                    projected_value: 150.0,
                    confidence,
                    unit: "ops/s".into(),
                },
                risk_score: 0.15,
                rollback_plan: RollbackPlan {
                    strategy: RollbackStrategy::ConfigRestore,
                    steps: vec!["restore batch_size=32".into()],
                    estimated_duration_secs: 60,
                },
            },
            confidence,
            reversibility: ReversibilityLevel::FullyReversible,
            impact: ImpactAssessment {
                affected_components: vec!["scheduler".into()],
                risk_score: 0.15,
                risk_factors: vec!["minor risk".into()],
                blast_radius: "scheduler only".into(),
            },
            governance_tier: tier,
            estimated_improvement: ImprovementEstimate {
                metric: "throughput".into(),
                current_value: 100.0,
                projected_value: 150.0,
                confidence,
                unit: "ops/s".into(),
            },
            stabilized_at: Utc::now(),
            status: IntentStatus::Validated,
        }
    }

    #[test]
    fn integration_full_pipeline() {
        let mapper = DeclarationMapper::new(test_worldline());
        let mut engine = SelfCommitmentEngine::new(
            mapper,
            CommitmentConfig {
                require_observation_period: true,
                ..CommitmentConfig::default()
            },
        );

        let intent = make_intent(SubstrateTier::Tier0, 0.9);
        let intent_id = intent.id.clone();

        // Step 1: Start observation (2 hours ago to pass immediately)
        let two_hours_ago = Utc::now() - Duration::hours(2);
        engine.start_observation_at(intent.clone(), two_hours_ago);
        assert_eq!(engine.pending_observation().len(), 1);

        // Step 2: Check ready
        let ready = engine.check_ready();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], intent_id);
        assert_eq!(engine.ready_for_commitment().len(), 1);

        // Step 3: Prepare declaration
        let decl = engine.prepare_declaration(&intent).unwrap();
        assert!(decl.derived_from_intent.is_some());

        // Step 4: Record submission
        let cmt_id = CommitmentId::new();
        let self_cmt_id = engine.record_submission(&intent_id, cmt_id, SubstrateTier::Tier0);

        // Step 5: Record gate approval
        engine.record_gate_result(&self_cmt_id, true, None);

        // Step 6: Record fulfillment
        engine.record_outcome(&self_cmt_id, true, None);

        let summary = engine.summary();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.fulfilled, 1);
    }

    #[test]
    fn integration_observation_enforcement() {
        let mapper = DeclarationMapper::new(test_worldline());
        let mut engine = SelfCommitmentEngine::new(mapper, CommitmentConfig::default());

        let intent = make_intent(SubstrateTier::Tier3, 0.92);

        // Start observation now (Tier3 = 72 hours)
        engine.start_observation(intent.clone());

        // Should NOT be ready yet
        let ready = engine.check_ready();
        assert!(ready.is_empty());
        assert_eq!(engine.observing_count(), 1);
        assert_eq!(engine.ready_count(), 0);

        // Trying to prepare a declaration should fail
        let result = engine.prepare_declaration(&intent);
        assert!(result.is_err());
    }

    #[test]
    fn integration_multiple_commitments_lifecycle() {
        let mapper = DeclarationMapper::new(test_worldline());
        let mut engine = SelfCommitmentEngine::new(
            mapper,
            CommitmentConfig {
                require_observation_period: false,
                ..CommitmentConfig::default()
            },
        );

        // Submit 3 intents
        let mut ids = Vec::new();
        for _ in 0..3 {
            let intent = make_intent(SubstrateTier::Tier0, 0.9);
            let intent_id = intent.id.clone();
            engine.start_observation(intent);

            let self_cmt_id =
                engine.record_submission(&intent_id, CommitmentId::new(), SubstrateTier::Tier0);
            ids.push(self_cmt_id);
        }

        // Approve first, deny second, fulfill first, fail third
        engine.record_gate_result(&ids[0], true, None);
        engine.record_gate_result(&ids[1], false, Some("policy denied".into()));
        engine.record_gate_result(&ids[2], true, None);

        engine.record_outcome(&ids[0], true, None);
        engine.record_outcome(&ids[2], false, Some("rollback triggered".into()));

        let summary = engine.summary();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.fulfilled, 1);
        assert_eq!(summary.denied, 1);
        assert_eq!(summary.failed, 1);

        // Verify committed() returns all records
        assert_eq!(engine.committed().len(), 3);
    }
}
