//! Cycle execution — one full iteration of the self-producing loop.
//!
//! A cycle passes through all 8 phases (Observing → Complete),
//! recording each step with its subsystem, timing, and success status.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::{EvosError, EvosResult};
use crate::health::HealthReport;
use crate::types::{CycleId, CyclePhase, EvosConfig, SubsystemId};

// ── Cycle Step ──────────────────────────────────────────────────────

/// A single step within a cycle execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CycleStep {
    /// Which phase this step covers.
    pub phase: CyclePhase,
    /// Which subsystem was involved.
    pub subsystem: SubsystemId,
    /// When the step started.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// When the step completed.
    pub completed_at: chrono::DateTime<chrono::Utc>,
    /// Whether the step succeeded.
    pub success: bool,
    /// Optional message.
    pub message: Option<String>,
}

impl std::fmt::Display for CycleStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.success { "OK" } else { "FAIL" };
        write!(f, "[{}] {} ({})", status, self.phase, self.subsystem)
    }
}

// ── Cycle Record ────────────────────────────────────────────────────

/// Complete record of a cycle execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CycleRecord {
    /// Unique cycle identifier.
    pub id: CycleId,
    /// Steps executed in this cycle.
    pub steps: Vec<CycleStep>,
    /// When the cycle started.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// When the cycle completed (None if still running).
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether the full cycle succeeded.
    pub success: bool,
    /// Error message if the cycle failed.
    pub error_message: Option<String>,
}

impl CycleRecord {
    /// How many steps completed successfully.
    pub fn successful_steps(&self) -> usize {
        self.steps.iter().filter(|s| s.success).count()
    }

    /// How many steps failed.
    pub fn failed_steps(&self) -> usize {
        self.steps.iter().filter(|s| !s.success).count()
    }

    /// Total number of steps.
    pub fn total_steps(&self) -> usize {
        self.steps.len()
    }

    /// The last completed phase.
    pub fn last_phase(&self) -> Option<&CyclePhase> {
        self.steps.last().map(|s| &s.phase)
    }
}

impl std::fmt::Display for CycleRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.success { "success" } else { "failed" };
        write!(
            f,
            "Cycle({}, steps={}, {})",
            self.id,
            self.total_steps(),
            status,
        )
    }
}

// ── Cycle Runner Trait ──────────────────────────────────────────────

/// Trait for executing one full EVOS cycle.
pub trait CycleRunner: Send + Sync {
    /// Execute a full cycle, returning the record.
    fn run_cycle(
        &self,
        health_report: &HealthReport,
        config: &EvosConfig,
    ) -> EvosResult<CycleRecord>;

    /// Name of this runner.
    fn name(&self) -> &str;
}

/// Simulated cycle runner for deterministic testing.
pub struct SimulatedCycleRunner {
    /// Which phases should fail (by ordinal).
    fail_at_phases: Vec<u8>,
}

impl SimulatedCycleRunner {
    /// Create a runner where all phases succeed.
    pub fn all_passing() -> Self {
        Self {
            fail_at_phases: Vec::new(),
        }
    }

    /// Create a runner that fails at specific phase ordinals.
    pub fn failing_at(phases: Vec<u8>) -> Self {
        Self {
            fail_at_phases: phases,
        }
    }
}

impl Default for SimulatedCycleRunner {
    fn default() -> Self {
        Self::all_passing()
    }
}

impl CycleRunner for SimulatedCycleRunner {
    fn run_cycle(
        &self,
        health_report: &HealthReport,
        config: &EvosConfig,
    ) -> EvosResult<CycleRecord> {
        let cycle_id = CycleId::new();
        let started_at = Utc::now();
        let mut steps = Vec::new();

        // Check health before starting if required
        if config.require_healthy_start && !health_report.all_allow_progression() {
            let blocking: Vec<String> = health_report
                .blocking_subsystems()
                .iter()
                .map(|s| s.to_string())
                .collect();
            return Err(EvosError::SubstrateNotReady(format!(
                "blocked by: {}",
                blocking.join(", ")
            )));
        }

        // Execute each phase (0..7, excluding Complete which is just a marker)
        for ordinal in 0..7u8 {
            let phase = CyclePhase::from_ordinal(ordinal).unwrap();
            let subsystem = phase.primary_subsystem();
            let step_start = Utc::now();

            // Check if subsystem is healthy enough
            let subsystem_health = health_report
                .entries
                .iter()
                .find(|e| e.subsystem == subsystem);

            let subsystem_ok = subsystem_health
                .map(|h| h.status.allows_progression())
                .unwrap_or(false);

            let should_fail = self.fail_at_phases.contains(&ordinal);
            let success = subsystem_ok && !should_fail;

            let message = if !subsystem_ok {
                Some(format!("subsystem {} not healthy", subsystem))
            } else if should_fail {
                Some(format!("simulated failure at {}", phase))
            } else {
                Some(format!("{} completed", phase))
            };

            steps.push(CycleStep {
                phase: phase.clone(),
                subsystem,
                started_at: step_start,
                completed_at: Utc::now(),
                success,
                message,
            });

            // Abort on failure if configured
            if !success && config.abort_on_failure {
                return Ok(CycleRecord {
                    id: cycle_id,
                    steps,
                    started_at,
                    completed_at: Some(Utc::now()),
                    success: false,
                    error_message: Some(format!("cycle aborted at {}", phase)),
                });
            }
        }

        // Add the Complete step
        steps.push(CycleStep {
            phase: CyclePhase::Complete,
            subsystem: SubsystemId::Observation, // loops back
            started_at: Utc::now(),
            completed_at: Utc::now(),
            success: true,
            message: Some("cycle complete".into()),
        });

        let all_success = steps.iter().all(|s| s.success);

        Ok(CycleRecord {
            id: cycle_id,
            steps,
            started_at,
            completed_at: Some(Utc::now()),
            success: all_success,
            error_message: if all_success {
                None
            } else {
                Some("some steps failed".into())
            },
        })
    }

    fn name(&self) -> &str {
        "simulated-cycle-runner"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::{HealthChecker, SimulatedHealthChecker};
    use crate::types::SubsystemStatus;

    fn healthy_report() -> HealthReport {
        let checker = SimulatedHealthChecker::all_healthy();
        checker.check_all().unwrap()
    }

    fn default_config() -> EvosConfig {
        EvosConfig::default()
    }

    #[test]
    fn full_successful_cycle() {
        let runner = SimulatedCycleRunner::all_passing();
        let report = healthy_report();
        let record = runner.run_cycle(&report, &default_config()).unwrap();
        assert!(record.success);
        assert_eq!(record.total_steps(), 8); // 7 phases + Complete
        assert_eq!(record.successful_steps(), 8);
        assert_eq!(record.failed_steps(), 0);
    }

    #[test]
    fn cycle_phases_in_order() {
        let runner = SimulatedCycleRunner::all_passing();
        let report = healthy_report();
        let record = runner.run_cycle(&report, &default_config()).unwrap();
        for (i, step) in record.steps.iter().enumerate() {
            assert_eq!(step.phase.ordinal(), i as u8);
        }
    }

    #[test]
    fn cycle_abort_on_failure() {
        let runner = SimulatedCycleRunner::failing_at(vec![3]); // Fail at Committing
        let report = healthy_report();
        let record = runner.run_cycle(&report, &default_config()).unwrap();
        assert!(!record.success);
        assert_eq!(record.total_steps(), 4); // 0,1,2,3 then abort
        assert_eq!(record.last_phase().unwrap(), &CyclePhase::Committing);
    }

    #[test]
    fn cycle_no_abort_continues() {
        let runner = SimulatedCycleRunner::failing_at(vec![2]); // Fail at Forming
        let config = EvosConfig {
            abort_on_failure: false,
            ..EvosConfig::default()
        };
        let report = healthy_report();
        let record = runner.run_cycle(&report, &config).unwrap();
        assert!(!record.success);
        assert_eq!(record.total_steps(), 8); // All phases executed
        assert_eq!(record.failed_steps(), 1);
        assert_eq!(record.successful_steps(), 7);
    }

    #[test]
    fn cycle_blocked_by_unhealthy_subsystem() {
        let health_checker = SimulatedHealthChecker::with_overrides(vec![(
            SubsystemId::Observation,
            SubsystemStatus::Failed("crash".into()),
        )]);
        let report = health_checker.check_all().unwrap();
        let runner = SimulatedCycleRunner::all_passing();
        let result = runner.run_cycle(&report, &default_config());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("observation"));
    }

    #[test]
    fn cycle_not_blocked_when_health_check_disabled() {
        let health_checker = SimulatedHealthChecker::with_overrides(vec![(
            SubsystemId::Bootstrap,
            SubsystemStatus::Failed("offline".into()),
        )]);
        let report = health_checker.check_all().unwrap();
        let config = EvosConfig {
            require_healthy_start: false,
            abort_on_failure: false,
            ..EvosConfig::default()
        };
        let runner = SimulatedCycleRunner::all_passing();
        // Not blocked at start, but the bootstrap step itself may fail
        let record = runner.run_cycle(&report, &config).unwrap();
        // Bootstrap is not a primary subsystem for phases 0-6, so it doesn't affect those
        assert_eq!(record.total_steps(), 8);
    }

    #[test]
    fn cycle_record_display() {
        let runner = SimulatedCycleRunner::all_passing();
        let report = healthy_report();
        let record = runner.run_cycle(&report, &default_config()).unwrap();
        let display = record.to_string();
        assert!(display.contains("cycle:"));
        assert!(display.contains("steps=8"));
        assert!(display.contains("success"));
    }

    #[test]
    fn cycle_step_display() {
        let step = CycleStep {
            phase: CyclePhase::Observing,
            subsystem: SubsystemId::Observation,
            started_at: Utc::now(),
            completed_at: Utc::now(),
            success: true,
            message: None,
        };
        let display = step.to_string();
        assert!(display.contains("[OK]"));
        assert!(display.contains("observing"));
    }

    #[test]
    fn cycle_fail_at_first_phase() {
        let runner = SimulatedCycleRunner::failing_at(vec![0]);
        let report = healthy_report();
        let record = runner.run_cycle(&report, &default_config()).unwrap();
        assert!(!record.success);
        assert_eq!(record.total_steps(), 1);
        assert_eq!(record.last_phase().unwrap(), &CyclePhase::Observing);
    }

    #[test]
    fn cycle_fail_at_last_phase() {
        let runner = SimulatedCycleRunner::failing_at(vec![6]); // Deploying
        let report = healthy_report();
        let record = runner.run_cycle(&report, &default_config()).unwrap();
        assert!(!record.success);
        assert_eq!(record.total_steps(), 7); // 0-6 then abort
    }

    #[test]
    fn runner_name() {
        let runner = SimulatedCycleRunner::all_passing();
        assert_eq!(runner.name(), "simulated-cycle-runner");
    }

    #[test]
    fn multiple_failures_no_abort() {
        let runner = SimulatedCycleRunner::failing_at(vec![1, 3, 5]);
        let config = EvosConfig {
            abort_on_failure: false,
            ..EvosConfig::default()
        };
        let report = healthy_report();
        let record = runner.run_cycle(&report, &config).unwrap();
        assert!(!record.success);
        assert_eq!(record.failed_steps(), 3);
        assert_eq!(record.successful_steps(), 5);
    }
}
