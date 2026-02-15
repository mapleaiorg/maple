//! Deployment engine — orchestrator for the deployment pipeline.
//!
//! The `DeploymentEngine` orchestrates the full deployment lifecycle:
//! 1. Validate the codegen artifact
//! 2. (Optional) Create a GitHub branch and commit
//! 3. Execute the deployment strategy
//! 4. On success: mark succeeded, collect performance deltas
//! 5. On failure: attempt rollback, mark rolled back or failed
//! 6. (Optional) Create a GitHub PR
//! 7. Store the deployment record
//! 8. Generate observation feedback (closing the cycle)

use std::collections::VecDeque;

use maple_worldline_codegen::CodegenArtifact;
use maple_worldline_observation::events::{ObservationMetadata, SelfObservationEvent};
use maple_worldline_self_mod_gate::ledger::DeploymentStatus;
use maple_worldline_self_mod_gate::types::SelfModTier;

use crate::error::{DeploymentError, DeploymentResult};
use crate::executor::DeploymentExecutor;
use crate::feedback::DeploymentFeedback;
use crate::github::GitHubIntegration;
use crate::rollback::DeploymentRollbackExecutor;
use crate::strategy::{execute_strategy, plan_strategy};
use crate::types::{DeploymentConfig, DeploymentId, DeploymentPhase, DeploymentRecord, DeploymentSummary};

// ── Learning Metrics ───────────────────────────────────────────────────

/// Aggregate metrics for learning and observation.
///
/// Tracks success/failure rates, rollback rates, and average durations
/// across all tracked deployments. Feeds into the observation layer
/// to improve future deployment decisions.
#[derive(Clone, Debug)]
pub struct DeploymentLearningMetrics {
    /// Total terminal deployments tracked.
    pub total_terminal: usize,
    /// Deployment success rate (0.0–1.0).
    pub success_rate: f64,
    /// Rollback rate (0.0–1.0).
    pub rollback_rate: f64,
    /// Average deployment duration in milliseconds (for completed deployments).
    pub avg_duration_ms: f64,
    /// Success rate per tier.
    pub per_tier_success: Vec<(SelfModTier, f64)>,
    /// Success rate per strategy.
    pub per_strategy_success: Vec<(String, f64)>,
}

// ── Deployment Engine ──────────────────────────────────────────────────

/// The main deployment engine orchestrator.
///
/// Coordinates executor, rollback, GitHub integration, monitoring, and
/// feedback generation to produce complete deployment records.
pub struct DeploymentEngine {
    executor: Box<dyn DeploymentExecutor>,
    rollback_executor: Box<dyn DeploymentRollbackExecutor>,
    github: Option<Box<dyn GitHubIntegration>>,
    config: DeploymentConfig,
    records: VecDeque<DeploymentRecord>,
}

impl DeploymentEngine {
    /// Create a new deployment engine.
    pub fn new(
        executor: Box<dyn DeploymentExecutor>,
        rollback_executor: Box<dyn DeploymentRollbackExecutor>,
    ) -> Self {
        Self {
            executor,
            rollback_executor,
            github: None,
            config: DeploymentConfig::default(),
            records: VecDeque::new(),
        }
    }

    /// Create a deployment engine with configuration.
    pub fn with_config(
        executor: Box<dyn DeploymentExecutor>,
        rollback_executor: Box<dyn DeploymentRollbackExecutor>,
        config: DeploymentConfig,
    ) -> Self {
        Self {
            executor,
            rollback_executor,
            github: None,
            config,
            records: VecDeque::new(),
        }
    }

    /// Set the GitHub integration (optional).
    pub fn with_github(mut self, github: Box<dyn GitHubIntegration>) -> Self {
        self.github = Some(github);
        self
    }

    /// Deploy a codegen artifact.
    ///
    /// Returns the deployment ID on success or an error on failure.
    /// The deployment record is always stored regardless of outcome.
    pub fn deploy(&mut self, artifact: &CodegenArtifact) -> DeploymentResult<DeploymentId> {
        // 1. Validate artifact
        if !artifact.is_deployable() {
            let mut record = DeploymentRecord::new(
                artifact.codegen_id.to_string(),
                artifact.commitment_id.clone(),
                artifact.tier.clone(),
                artifact.deployment_strategy.clone(),
                artifact.affected_files().into_iter().map(String::from).collect(),
            );
            record.mark_failed("Artifact not deployable".into());
            self.store_record(record);
            return Err(DeploymentError::ArtifactNotDeployable(
                "Artifact is not validated or has no generated files".into(),
            ));
        }

        // 2. Create record
        let files: Vec<String> = artifact
            .affected_files()
            .into_iter()
            .map(String::from)
            .collect();

        let mut record = DeploymentRecord::new(
            artifact.codegen_id.to_string(),
            artifact.commitment_id.clone(),
            artifact.tier.clone(),
            artifact.deployment_strategy.clone(),
            files,
        );
        record.mark_in_progress();
        let deploy_id = record.id.clone();

        // 3. (Optional) GitHub: create branch + commit + push
        let branch_name = format!("deploy/{}", deploy_id.0);
        if let Some(ref github) = self.github {
            let _ = github.create_branch(&branch_name, "main");
            let _ = github.commit_files(
                &branch_name,
                &record.files_deployed,
                &format!("deploy: {}", artifact.commitment_id),
            );
            let _ = github.push_branch(&branch_name);
        }

        // 4. Execute strategy
        let plan = plan_strategy(&artifact.deployment_strategy, &self.config);
        let strategy_result =
            execute_strategy(&*self.executor, artifact, &plan, &self.config, &mut record);

        match strategy_result {
            Ok(()) => {
                // 5. Success path
                record.mark_succeeded();

                // (Optional) GitHub: create PR
                if let Some(ref github) = self.github {
                    let _ = github.create_pr(
                        &branch_name,
                        &format!("[deploy] {}", artifact.commitment_id),
                        &format!(
                            "Automated deployment for commitment {}.\n\nFiles: {:?}",
                            artifact.commitment_id,
                            record.files_deployed,
                        ),
                    );
                }
            }
            Err(strategy_err) => {
                // 6. Failure path: attempt rollback
                if self.config.auto_rollback {
                    record.advance_phase(DeploymentPhase::RollingBack);

                    let rollback_result = self.rollback_executor.rollback(
                        &artifact.rollback_plan,
                        &record.files_deployed,
                    );

                    match rollback_result {
                        Ok(rb) if rb.success => {
                            record.mark_rolled_back(format!("Strategy failed: {}", strategy_err));
                        }
                        Ok(_) => {
                            record.mark_failed(format!(
                                "Strategy failed ({}), rollback also failed",
                                strategy_err,
                            ));
                        }
                        Err(rb_err) => {
                            record.mark_failed(format!(
                                "Strategy failed ({}), rollback error: {}",
                                strategy_err, rb_err,
                            ));
                        }
                    }
                } else {
                    record.mark_failed(format!("Strategy failed: {}", strategy_err));
                }
            }
        }

        // 7. Store record
        let id = record.id.clone();
        self.store_record(record);

        // Return the deploy ID (even on failure, the record is stored)
        if let Some(rec) = self.find(&id) {
            if rec.status.is_success() {
                Ok(id)
            } else {
                match &rec.status {
                    DeploymentStatus::Failed(reason) => {
                        Err(DeploymentError::StrategyExecutionFailed(reason.clone()))
                    }
                    DeploymentStatus::RolledBack(reason) => {
                        Err(DeploymentError::HealthCheckFailed(reason.clone()))
                    }
                    _ => Ok(id),
                }
            }
        } else {
            Ok(id)
        }
    }

    /// Get observation feedback for a deployment.
    pub fn get_feedback(
        &self,
        id: &DeploymentId,
    ) -> Option<(SelfObservationEvent, ObservationMetadata)> {
        self.find(id)
            .and_then(|record| DeploymentFeedback::generate_feedback(record))
    }

    /// Find a deployment record by ID.
    pub fn find(&self, id: &DeploymentId) -> Option<&DeploymentRecord> {
        self.records.iter().find(|r| r.id == *id)
    }

    /// Find a deployment record by commitment ID.
    pub fn find_by_commitment(&self, commitment_id: &str) -> Option<&DeploymentRecord> {
        self.records
            .iter()
            .find(|r| r.commitment_id == commitment_id)
    }

    /// Get all deployment records.
    pub fn all_records(&self) -> &VecDeque<DeploymentRecord> {
        &self.records
    }

    /// Compute summary statistics.
    pub fn summary(&self) -> DeploymentSummary {
        let mut summary = DeploymentSummary::default();
        for record in &self.records {
            summary.total += 1;
            summary.total_files_deployed += record.files_deployed.len();
            match &record.status {
                DeploymentStatus::Pending => summary.pending += 1,
                DeploymentStatus::InProgress => summary.in_progress += 1,
                DeploymentStatus::Succeeded => summary.succeeded += 1,
                DeploymentStatus::Failed(_) => summary.failed += 1,
                DeploymentStatus::RolledBack(_) => summary.rolled_back += 1,
            }
        }
        summary
    }

    /// Compute learning metrics from all tracked deployments.
    pub fn learning_metrics(&self) -> DeploymentLearningMetrics {
        let terminal: Vec<&DeploymentRecord> = self
            .records
            .iter()
            .filter(|r| r.status.is_terminal())
            .collect();

        let total_terminal = terminal.len();
        if total_terminal == 0 {
            return DeploymentLearningMetrics {
                total_terminal: 0,
                success_rate: 0.0,
                rollback_rate: 0.0,
                avg_duration_ms: 0.0,
                per_tier_success: vec![],
                per_strategy_success: vec![],
            };
        }

        let succeeded = terminal.iter().filter(|r| r.status.is_success()).count();
        let rolled_back = terminal.iter().filter(|r| r.rollback_triggered).count();

        let total_duration: i64 = terminal
            .iter()
            .filter_map(|r| r.duration_ms())
            .sum();
        let duration_count = terminal.iter().filter(|r| r.duration_ms().is_some()).count();

        // Per-tier breakdown
        let tiers = [
            SelfModTier::Tier0Configuration,
            SelfModTier::Tier1OperatorInternal,
            SelfModTier::Tier2ApiChange,
            SelfModTier::Tier3KernelChange,
            SelfModTier::Tier4SubstrateChange,
            SelfModTier::Tier5ArchitecturalChange,
        ];
        let per_tier_success: Vec<(SelfModTier, f64)> = tiers
            .iter()
            .filter_map(|tier| {
                let tier_records: Vec<_> = terminal
                    .iter()
                    .filter(|r| r.tier == *tier)
                    .collect();
                if tier_records.is_empty() {
                    None
                } else {
                    let tier_succeeded = tier_records
                        .iter()
                        .filter(|r| r.status.is_success())
                        .count();
                    Some((
                        tier.clone(),
                        tier_succeeded as f64 / tier_records.len() as f64,
                    ))
                }
            })
            .collect();

        // Per-strategy breakdown
        let strategies = ["immediate", "canary", "staged", "blue-green"];
        let per_strategy_success: Vec<(String, f64)> = strategies
            .iter()
            .filter_map(|strategy_name| {
                let strategy_records: Vec<_> = terminal
                    .iter()
                    .filter(|r| r.strategy.to_string().starts_with(strategy_name))
                    .collect();
                if strategy_records.is_empty() {
                    None
                } else {
                    let strategy_succeeded = strategy_records
                        .iter()
                        .filter(|r| r.status.is_success())
                        .count();
                    Some((
                        strategy_name.to_string(),
                        strategy_succeeded as f64 / strategy_records.len() as f64,
                    ))
                }
            })
            .collect();

        DeploymentLearningMetrics {
            total_terminal,
            success_rate: succeeded as f64 / total_terminal as f64,
            rollback_rate: rolled_back as f64 / total_terminal as f64,
            avg_duration_ms: if duration_count > 0 {
                total_duration as f64 / duration_count as f64
            } else {
                0.0
            },
            per_tier_success,
            per_strategy_success,
        }
    }

    /// Store a record in the bounded FIFO.
    fn store_record(&mut self, record: DeploymentRecord) {
        self.records.push_back(record);
        while self.records.len() > self.config.max_tracked_records {
            self.records.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::SimulatedDeploymentExecutor;
    use crate::github::SimulatedGitHub;
    use crate::rollback::SimulatedRollback;
    use chrono::Utc;
    use maple_worldline_codegen::types::GeneratedCode;
    use maple_worldline_codegen::CodegenId;
    use maple_worldline_intent::proposal::{RollbackPlan, RollbackStrategy};
    use maple_worldline_intent::types::{IntentId, MeaningId};
    use maple_worldline_self_mod_gate::commitment::IntentChain;
    use maple_worldline_self_mod_gate::types::DeploymentStrategy;

    fn make_artifact(
        tier: SelfModTier,
        strategy: DeploymentStrategy,
    ) -> CodegenArtifact {
        CodegenArtifact {
            codegen_id: CodegenId::new(),
            commitment_id: "commit-1".into(),
            tier,
            generated_files: vec![GeneratedCode {
                change_spec_index: 0,
                file_path: "src/config.rs".into(),
                content: "fn load() {}".into(),
                description: "test".into(),
                content_hash: "abc123".into(),
                generated_at: Utc::now(),
            }],
            compilation_results: vec![],
            test_results: vec![],
            performance_results: vec![],
            fully_validated: true,
            total_files: 1,
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

    fn make_non_deployable_artifact() -> CodegenArtifact {
        let mut artifact = make_artifact(
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
        );
        artifact.fully_validated = false;
        artifact
    }

    fn make_engine() -> DeploymentEngine {
        DeploymentEngine::new(
            Box::new(SimulatedDeploymentExecutor::healthy()),
            Box::new(SimulatedRollback::succeeding()),
        )
    }

    #[test]
    fn deploy_immediate_success() {
        let mut engine = make_engine();
        let artifact = make_artifact(
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
        );
        let result = engine.deploy(&artifact);
        assert!(result.is_ok());

        let id = result.unwrap();
        let record = engine.find(&id).unwrap();
        assert!(matches!(record.status, DeploymentStatus::Succeeded));
    }

    #[test]
    fn deploy_non_deployable_artifact_fails() {
        let mut engine = make_engine();
        let artifact = make_non_deployable_artifact();
        let result = engine.deploy(&artifact);
        assert!(result.is_err());

        // Record should still be stored
        assert_eq!(engine.all_records().len(), 1);
    }

    #[test]
    fn deploy_canary_success() {
        let mut engine = make_engine();
        let artifact = make_artifact(
            SelfModTier::Tier1OperatorInternal,
            DeploymentStrategy::Canary { traffic_fraction: 0.05 },
        );
        let result = engine.deploy(&artifact);
        assert!(result.is_ok());

        let id = result.unwrap();
        let record = engine.find(&id).unwrap();
        assert!(matches!(record.status, DeploymentStatus::Succeeded));
        assert!(!record.health_snapshots.is_empty());
    }

    #[test]
    fn deploy_with_rollback_on_regression() {
        let mut engine = DeploymentEngine::new(
            Box::new(SimulatedDeploymentExecutor::unhealthy()),
            Box::new(SimulatedRollback::succeeding()),
        );
        let artifact = make_artifact(
            SelfModTier::Tier1OperatorInternal,
            DeploymentStrategy::Canary { traffic_fraction: 0.05 },
        );
        let result = engine.deploy(&artifact);
        assert!(result.is_err());

        // Should have one record that was rolled back
        let summary = engine.summary();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.rolled_back, 1);
    }

    #[test]
    fn deploy_with_rollback_failure() {
        let mut engine = DeploymentEngine::new(
            Box::new(SimulatedDeploymentExecutor::unhealthy()),
            Box::new(SimulatedRollback::failing()),
        );
        let artifact = make_artifact(
            SelfModTier::Tier1OperatorInternal,
            DeploymentStrategy::Canary { traffic_fraction: 0.05 },
        );
        let result = engine.deploy(&artifact);
        assert!(result.is_err());

        // Rollback failed, so it should be marked as Failed, not RolledBack
        let summary = engine.summary();
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn deploy_with_github_integration() {
        let mut engine = make_engine().with_github(Box::new(SimulatedGitHub::succeeding()));
        let artifact = make_artifact(
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
        );
        let result = engine.deploy(&artifact);
        assert!(result.is_ok());
    }

    #[test]
    fn deploy_feedback_generation() {
        let mut engine = make_engine();
        let artifact = make_artifact(
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
        );
        let id = engine.deploy(&artifact).unwrap();

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
            maple_worldline_observation::SubsystemId::Custom("deployment-pipeline".into())
        );
    }

    #[test]
    fn find_by_commitment() {
        let mut engine = make_engine();
        let artifact = make_artifact(
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
        );
        engine.deploy(&artifact).unwrap();

        let record = engine.find_by_commitment("commit-1");
        assert!(record.is_some());
        assert!(engine.find_by_commitment("nonexistent").is_none());
    }

    #[test]
    fn summary_statistics() {
        let mut engine = make_engine();

        // Deploy two successful artifacts
        for _ in 0..2 {
            let artifact = make_artifact(
                SelfModTier::Tier0Configuration,
                DeploymentStrategy::Immediate,
            );
            engine.deploy(&artifact).unwrap();
        }

        let summary = engine.summary();
        assert_eq!(summary.total, 2);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.total_files_deployed, 2);
    }

    #[test]
    fn learning_metrics() {
        let mut engine = make_engine();

        // One success
        let artifact = make_artifact(
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
        );
        engine.deploy(&artifact).unwrap();

        // One failure (non-deployable)
        let bad = make_non_deployable_artifact();
        let _ = engine.deploy(&bad);

        let metrics = engine.learning_metrics();
        assert_eq!(metrics.total_terminal, 2);
        assert!((metrics.success_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn bounded_record_fifo() {
        let config = DeploymentConfig {
            max_tracked_records: 3,
            ..DeploymentConfig::default()
        };
        let mut engine = DeploymentEngine::with_config(
            Box::new(SimulatedDeploymentExecutor::healthy()),
            Box::new(SimulatedRollback::succeeding()),
            config,
        );

        for _ in 0..5 {
            let artifact = make_artifact(
                SelfModTier::Tier0Configuration,
                DeploymentStrategy::Immediate,
            );
            engine.deploy(&artifact).unwrap();
        }

        assert_eq!(engine.all_records().len(), 3);
    }

    #[test]
    fn deploy_staged_success() {
        let mut engine = make_engine();
        let artifact = make_artifact(
            SelfModTier::Tier2ApiChange,
            DeploymentStrategy::Staged,
        );
        let result = engine.deploy(&artifact);
        assert!(result.is_ok());

        let id = result.unwrap();
        let record = engine.find(&id).unwrap();
        assert!(matches!(record.status, DeploymentStatus::Succeeded));
        // Staged has 4 fractions, so multiple phases
        assert!(record.phase_count() > 2);
    }

    #[test]
    fn learning_metrics_per_tier() {
        let mut engine = make_engine();

        // Tier0 success
        let a0 = make_artifact(SelfModTier::Tier0Configuration, DeploymentStrategy::Immediate);
        engine.deploy(&a0).unwrap();

        // Tier1 success
        let a1 = make_artifact(
            SelfModTier::Tier1OperatorInternal,
            DeploymentStrategy::Canary { traffic_fraction: 0.05 },
        );
        engine.deploy(&a1).unwrap();

        let metrics = engine.learning_metrics();
        assert!(!metrics.per_tier_success.is_empty());
        assert!(!metrics.per_strategy_success.is_empty());
    }

    #[test]
    fn deploy_auto_rollback_disabled() {
        let config = DeploymentConfig {
            auto_rollback: false,
            ..DeploymentConfig::default()
        };
        let mut engine = DeploymentEngine::with_config(
            Box::new(SimulatedDeploymentExecutor::unhealthy()),
            Box::new(SimulatedRollback::succeeding()),
            config,
        );
        let artifact = make_artifact(
            SelfModTier::Tier1OperatorInternal,
            DeploymentStrategy::Canary { traffic_fraction: 0.05 },
        );
        let result = engine.deploy(&artifact);
        assert!(result.is_err());

        // With auto_rollback=false, should be Failed not RolledBack
        let summary = engine.summary();
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.rolled_back, 0);
    }
}
