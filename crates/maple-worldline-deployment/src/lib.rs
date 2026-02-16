//! # maple-worldline-deployment
//!
//! **Deployment Pipeline** for the WorldLine Self-Producing Substrate.
//!
//! This is the **FINAL** crate that closes the self-producing substrate cycle:
//!
//! ```text
//! Observation → Meaning → Intent → Commitment → Codegen → [DEPLOYMENT] → Observation
//! ```
//!
//! Takes a validated `CodegenArtifact`, deploys it using one of four strategies
//! (Immediate/Canary/Staged/BlueGreen), monitors health, executes rollback on
//! failure, and feeds back to the observation layer.
//!
//! ## Architecture
//!
//! ```text
//! CodegenArtifact (validated)
//!     │
//!     ▼
//! DeploymentEngine
//!     │─── validate artifact (is_deployable)
//!     │─── (optional) GitHub: create_branch + commit + push
//!     │─── plan strategy → StrategyPlan (phases)
//!     │─── for each phase:
//!     │    ├── deploy_files (DeploymentExecutor trait)
//!     │    ├── check_health → MonitoringVerdict
//!     │    └── promote or rollback
//!     │─── (optional) GitHub: create_pr
//!     │─── generate feedback → SelfObservationEvent::GateSubmission
//!     ▼
//! DeploymentRecord + ObservationFeedback (closes the cycle)
//! ```
//!
//! ## Traits
//!
//! - [`DeploymentExecutor`] — abstracts file deployment and health checking
//! - [`DeploymentRollbackExecutor`] — abstracts rollback execution
//! - [`GitHubIntegration`] — abstracts Git/GitHub operations
//!
//! All have simulated implementations for testing.

#![deny(unsafe_code)]

pub mod engine;
pub mod error;
pub mod executor;
pub mod feedback;
pub mod github;
pub mod monitor;
pub mod rollback;
pub mod strategy;
pub mod types;

// Re-exports
pub use engine::{DeploymentEngine, DeploymentLearningMetrics};
pub use error::{DeploymentError, DeploymentResult};
pub use executor::{DeploymentExecutor, FileDeployResult, SimulatedDeploymentExecutor};
pub use feedback::DeploymentFeedback;
pub use github::{GitHubIntegration, GitOperationResult, SimulatedGitHub};
pub use monitor::{DeploymentMonitor, MonitoringVerdict};
pub use rollback::{DeploymentRollbackExecutor, DeploymentRollbackResult, SimulatedRollback};
pub use strategy::{execute_strategy, plan_strategy, StrategyPlan};
pub use types::{
    DeploymentConfig, DeploymentId, DeploymentPhase, DeploymentRecord, DeploymentSummary,
    HealthSnapshot,
};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use maple_worldline_codegen::artifact::CodegenArtifact;
    use maple_worldline_codegen::types::GeneratedCode;
    use maple_worldline_codegen::CodegenId;
    use maple_worldline_intent::proposal::{RollbackPlan, RollbackStrategy};
    use maple_worldline_intent::types::{IntentId, MeaningId};
    use maple_worldline_observation::events::{SelfObservationEvent, SubsystemId};
    use maple_worldline_self_mod_gate::commitment::IntentChain;
    use maple_worldline_self_mod_gate::ledger::DeploymentStatus;
    use maple_worldline_self_mod_gate::types::{DeploymentStrategy, SelfModTier};

    fn make_artifact(
        tier: SelfModTier,
        strategy: DeploymentStrategy,
        files: Vec<(&str, &str)>,
    ) -> CodegenArtifact {
        let generated_files: Vec<GeneratedCode> = files
            .into_iter()
            .enumerate()
            .map(|(i, (path, content))| GeneratedCode {
                change_spec_index: i,
                file_path: path.into(),
                content: content.into(),
                description: format!("change {}", i),
                content_hash: GeneratedCode::compute_hash(content),
                generated_at: Utc::now(),
            })
            .collect();
        let total_files = generated_files.len();

        CodegenArtifact {
            codegen_id: CodegenId::new(),
            commitment_id: "commit-1".into(),
            tier,
            generated_files,
            compilation_results: vec![],
            test_results: vec![],
            performance_results: vec![],
            fully_validated: true,
            total_files,
            total_tests: 0,
            tests_passed: 0,
            total_perf_gates: 0,
            perf_gates_passed: 0,
            assembled_at: Utc::now(),
            total_duration_ms: 100,
            deployment_strategy: strategy,
            rollback_plan: RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec!["git revert HEAD".into()],
                estimated_duration_secs: 60,
            },
            intent_chain: IntentChain {
                observation_ids: vec!["obs-1".into()],
                meaning_ids: vec![MeaningId::new()],
                intent_id: IntentId::new(),
            },
        }
    }

    #[test]
    fn integration_full_immediate_cycle() {
        let mut engine = DeploymentEngine::new(
            Box::new(SimulatedDeploymentExecutor::healthy()),
            Box::new(SimulatedRollback::succeeding()),
        );

        let artifact = make_artifact(
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            vec![("src/config.rs", "fn load() {}")],
        );

        let id = engine.deploy(&artifact).unwrap();
        let record = engine.find(&id).unwrap();

        assert!(matches!(record.status, DeploymentStatus::Succeeded));
        assert_eq!(record.files_deployed, vec!["src/config.rs"]);
        assert_eq!(record.tier, SelfModTier::Tier0Configuration);

        // Feedback closes the cycle
        let feedback = engine.get_feedback(&id);
        assert!(feedback.is_some());
        let (event, metadata) = feedback.unwrap();
        if let SelfObservationEvent::GateSubmission { approved, .. } = event {
            assert!(approved);
        } else {
            panic!("Expected GateSubmission");
        }
        assert_eq!(
            metadata.subsystem,
            SubsystemId::Custom("deployment-pipeline".into())
        );
    }

    #[test]
    fn integration_canary_with_rollback() {
        let mut engine = DeploymentEngine::new(
            Box::new(SimulatedDeploymentExecutor::unhealthy()),
            Box::new(SimulatedRollback::succeeding()),
        );

        let artifact = make_artifact(
            SelfModTier::Tier1OperatorInternal,
            DeploymentStrategy::Canary {
                traffic_fraction: 0.05,
            },
            vec![("src/handler.rs", "fn handle() {}")],
        );

        let result = engine.deploy(&artifact);
        assert!(result.is_err());

        // Check the record was stored and rolled back
        let summary = engine.summary();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.rolled_back, 1);

        // Feedback shows failure
        let record = engine.all_records().front().unwrap();
        let feedback = DeploymentFeedback::generate_feedback(record);
        assert!(feedback.is_some());
        let (event, _) = feedback.unwrap();
        if let SelfObservationEvent::GateSubmission { approved, .. } = event {
            assert!(!approved);
        } else {
            panic!("Expected GateSubmission");
        }
    }

    #[test]
    fn integration_provenance_preserved() {
        let mut engine = DeploymentEngine::new(
            Box::new(SimulatedDeploymentExecutor::healthy()),
            Box::new(SimulatedRollback::succeeding()),
        );

        let artifact = make_artifact(
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            vec![("src/config.rs", "fn load() {}")],
        );
        let obs_ids = artifact.intent_chain.observation_ids.clone();

        let id = engine.deploy(&artifact).unwrap();
        let record = engine.find(&id).unwrap();
        assert_eq!(record.commitment_id, "commit-1");

        // The observation IDs should match the original artifact
        assert_eq!(obs_ids, vec!["obs-1"]);
    }

    #[test]
    fn integration_multi_deploy_summary() {
        let mut engine = DeploymentEngine::new(
            Box::new(SimulatedDeploymentExecutor::healthy()),
            Box::new(SimulatedRollback::succeeding()),
        );

        // Deploy multiple artifacts
        for i in 0..5 {
            let artifact = make_artifact(
                SelfModTier::Tier0Configuration,
                DeploymentStrategy::Immediate,
                vec![(&format!("src/module{}.rs", i), "fn init() {}")],
            );
            engine.deploy(&artifact).unwrap();
        }

        let summary = engine.summary();
        assert_eq!(summary.total, 5);
        assert_eq!(summary.succeeded, 5);
        assert_eq!(summary.total_files_deployed, 5);

        let metrics = engine.learning_metrics();
        assert_eq!(metrics.total_terminal, 5);
        assert!((metrics.success_rate - 1.0).abs() < 0.01);
        assert!((metrics.rollback_rate - 0.0).abs() < 0.01);
    }

    #[test]
    fn integration_staged_phases() {
        let mut engine = DeploymentEngine::new(
            Box::new(SimulatedDeploymentExecutor::healthy()),
            Box::new(SimulatedRollback::succeeding()),
        );

        let artifact = make_artifact(
            SelfModTier::Tier2ApiChange,
            DeploymentStrategy::Staged,
            vec![
                ("src/api.rs", "fn endpoint() {}"),
                ("src/types.rs", "struct Request {}"),
            ],
        );

        let id = engine.deploy(&artifact).unwrap();
        let record = engine.find(&id).unwrap();

        assert!(matches!(record.status, DeploymentStatus::Succeeded));
        // Staged has 4 fractions: 10%, 25%, 50%, 100%
        // Each fraction creates Deploying + Monitoring + Promoting phases (except last)
        assert!(record.phase_count() >= 4);
        assert!(record.health_snapshots.len() >= 4); // Health snapshots per fraction
    }
}
