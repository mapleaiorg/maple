//! Strategy planning and multi-phase execution.
//!
//! Translates a `DeploymentStrategy` into a `StrategyPlan` (sequence of
//! traffic fractions), then executes each phase: deploy → monitor health →
//! promote (or trigger rollback on failure).

use maple_worldline_codegen::CodegenArtifact;
use maple_worldline_self_mod_gate::types::DeploymentStrategy;

use crate::error::{DeploymentError, DeploymentResult};
use crate::executor::DeploymentExecutor;
use crate::monitor::DeploymentMonitor;
use crate::types::{DeploymentConfig, DeploymentPhase, DeploymentRecord};

// ── Strategy Plan ──────────────────────────────────────────────────────

/// A concrete plan for executing a deployment strategy.
///
/// Contains the sequence of traffic fractions to deploy through
/// and whether monitoring is required between phases.
#[derive(Clone, Debug)]
pub struct StrategyPlan {
    /// Ordered traffic fractions to deploy through.
    pub phases: Vec<f64>,
    /// Whether health monitoring is required between phase promotions.
    pub requires_monitoring: bool,
    /// Human-readable description of the plan.
    pub description: String,
}

// ── Plan Strategy ──────────────────────────────────────────────────────

/// Translate a DeploymentStrategy into a concrete StrategyPlan.
pub fn plan_strategy(strategy: &DeploymentStrategy, config: &DeploymentConfig) -> StrategyPlan {
    match strategy {
        DeploymentStrategy::Immediate => StrategyPlan {
            phases: vec![1.0],
            requires_monitoring: false,
            description: "Immediate: deploy to 100% traffic".into(),
        },
        DeploymentStrategy::Canary { traffic_fraction } => StrategyPlan {
            phases: vec![*traffic_fraction, 1.0],
            requires_monitoring: true,
            description: format!(
                "Canary: deploy to {:.0}% → monitor → promote to 100%",
                traffic_fraction * 100.0,
            ),
        },
        DeploymentStrategy::Staged => StrategyPlan {
            phases: config.staged_fractions.clone(),
            requires_monitoring: true,
            description: format!(
                "Staged: {}",
                config
                    .staged_fractions
                    .iter()
                    .map(|f| format!("{:.0}%", f * 100.0))
                    .collect::<Vec<_>>()
                    .join(" → "),
            ),
        },
        DeploymentStrategy::BlueGreen => StrategyPlan {
            phases: vec![0.0, 1.0],
            requires_monitoring: true,
            description: "Blue-Green: deploy to standby → verify → switch traffic".into(),
        },
    }
}

// ── Execute Strategy ───────────────────────────────────────────────────

/// Execute a deployment strategy against the given artifact.
///
/// For each phase in the plan:
/// 1. Deploy files at the phase's traffic fraction
/// 2. If monitoring required: check health and evaluate verdict
/// 3. If healthy: promote to next phase
/// 4. If regression: return error (caller handles rollback)
pub fn execute_strategy(
    executor: &dyn DeploymentExecutor,
    artifact: &CodegenArtifact,
    plan: &StrategyPlan,
    config: &DeploymentConfig,
    record: &mut DeploymentRecord,
) -> DeploymentResult<()> {
    let files: Vec<String> = artifact
        .generated_files
        .iter()
        .map(|g| g.file_path.clone())
        .collect();

    let monitor = DeploymentMonitor::new(config);

    for (i, &fraction) in plan.phases.iter().enumerate() {
        // Phase: Deploying
        record.advance_phase(DeploymentPhase::Deploying {
            traffic_fraction: fraction,
        });

        let deploy_result = executor.deploy_files(&files, fraction)?;
        if !deploy_result.success {
            return Err(DeploymentError::PhaseFailed(format!(
                "Deploy failed at {:.0}% traffic: {}",
                fraction * 100.0,
                deploy_result.output,
            )));
        }

        // Phase: Monitoring (if required and not the last phase for immediate)
        if plan.requires_monitoring {
            record.advance_phase(DeploymentPhase::Monitoring {
                traffic_fraction: fraction,
                elapsed_secs: config.health_check_interval_secs,
            });

            let snapshots = executor.check_health(&files, fraction)?;
            for snap in &snapshots {
                record.add_health_snapshot(snap.clone());
            }

            let verdict = monitor.evaluate(&snapshots);
            if monitor.should_rollback(&verdict) {
                return Err(DeploymentError::HealthCheckFailed(format!(
                    "Health check failed at {:.0}% traffic: {}",
                    fraction * 100.0,
                    verdict,
                )));
            }

            // Compute performance deltas from monitoring
            let deltas = monitor.compute_performance_deltas(&snapshots);
            for delta in deltas {
                record.add_performance_delta(delta);
            }
        }

        // Phase: Promoting (if there's a next phase)
        if i + 1 < plan.phases.len() {
            let next_fraction = plan.phases[i + 1];
            record.advance_phase(DeploymentPhase::Promoting {
                from: fraction,
                to: next_fraction,
            });
        }
    }

    // Final traffic switch
    executor.switch_traffic()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::SimulatedDeploymentExecutor;
    use crate::types::DeploymentConfig;
    use maple_worldline_self_mod_gate::types::SelfModTier;

    fn make_config() -> DeploymentConfig {
        DeploymentConfig::default()
    }

    fn make_artifact() -> CodegenArtifact {
        use chrono::Utc;
        use maple_worldline_codegen::types::GeneratedCode;
        use maple_worldline_codegen::CodegenId;
        use maple_worldline_intent::proposal::{RollbackPlan, RollbackStrategy};
        use maple_worldline_intent::types::{IntentId, MeaningId};
        use maple_worldline_self_mod_gate::commitment::IntentChain;
        use maple_worldline_self_mod_gate::types::DeploymentStrategy;

        CodegenArtifact {
            codegen_id: CodegenId::new(),
            commitment_id: "commit-1".into(),
            tier: SelfModTier::Tier0Configuration,
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
            deployment_strategy: DeploymentStrategy::Immediate,
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
    fn plan_immediate_strategy() {
        let config = make_config();
        let plan = plan_strategy(&DeploymentStrategy::Immediate, &config);
        assert_eq!(plan.phases, vec![1.0]);
        assert!(!plan.requires_monitoring);
    }

    #[test]
    fn plan_canary_strategy() {
        let config = make_config();
        let plan = plan_strategy(
            &DeploymentStrategy::Canary {
                traffic_fraction: 0.05,
            },
            &config,
        );
        assert_eq!(plan.phases, vec![0.05, 1.0]);
        assert!(plan.requires_monitoring);
    }

    #[test]
    fn plan_staged_strategy() {
        let config = make_config();
        let plan = plan_strategy(&DeploymentStrategy::Staged, &config);
        assert_eq!(plan.phases, vec![0.10, 0.25, 0.50, 1.0]);
        assert!(plan.requires_monitoring);
    }

    #[test]
    fn plan_blue_green_strategy() {
        let config = make_config();
        let plan = plan_strategy(&DeploymentStrategy::BlueGreen, &config);
        assert_eq!(plan.phases, vec![0.0, 1.0]);
        assert!(plan.requires_monitoring);
    }

    #[test]
    fn execute_immediate_strategy_success() {
        let executor = SimulatedDeploymentExecutor::healthy();
        let config = make_config();
        let artifact = make_artifact();
        let plan = plan_strategy(&DeploymentStrategy::Immediate, &config);
        let mut record = DeploymentRecord::new(
            "codegen-1".into(),
            "commit-1".into(),
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            vec!["src/config.rs".into()],
        );

        let result = execute_strategy(&executor, &artifact, &plan, &config, &mut record);
        assert!(result.is_ok());
        assert!(record.phase_count() >= 1);
    }

    #[test]
    fn execute_canary_strategy_healthy() {
        let executor = SimulatedDeploymentExecutor::healthy();
        let config = make_config();
        let artifact = make_artifact();
        let strategy = DeploymentStrategy::Canary {
            traffic_fraction: 0.05,
        };
        let plan = plan_strategy(&strategy, &config);
        let mut record = DeploymentRecord::new(
            "codegen-1".into(),
            "commit-1".into(),
            SelfModTier::Tier1OperatorInternal,
            strategy,
            vec!["src/config.rs".into()],
        );

        let result = execute_strategy(&executor, &artifact, &plan, &config, &mut record);
        assert!(result.is_ok());
        assert!(!record.health_snapshots.is_empty());
    }

    #[test]
    fn execute_canary_strategy_regression() {
        // Fail at 100% traffic promotion
        let executor = SimulatedDeploymentExecutor::failing_at_fraction(1.0);
        let config = make_config();
        let artifact = make_artifact();
        let strategy = DeploymentStrategy::Canary {
            traffic_fraction: 0.05,
        };
        let plan = plan_strategy(&strategy, &config);
        let mut record = DeploymentRecord::new(
            "codegen-1".into(),
            "commit-1".into(),
            SelfModTier::Tier1OperatorInternal,
            strategy,
            vec!["src/config.rs".into()],
        );

        let result = execute_strategy(&executor, &artifact, &plan, &config, &mut record);
        assert!(result.is_err());
    }

    #[test]
    fn execute_deploy_failure() {
        let executor = SimulatedDeploymentExecutor::deploy_fails();
        let config = make_config();
        let artifact = make_artifact();
        let plan = plan_strategy(&DeploymentStrategy::Immediate, &config);
        let mut record = DeploymentRecord::new(
            "codegen-1".into(),
            "commit-1".into(),
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            vec!["src/config.rs".into()],
        );

        let result = execute_strategy(&executor, &artifact, &plan, &config, &mut record);
        assert!(result.is_err());
    }
}
