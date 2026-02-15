//! Cycle reporting — aggregated summaries of cycle execution.
//!
//! Produces a `CycleReport` that summarizes a cycle's execution
//! across all subsystems with timing and status information.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::cycle::CycleRecord;
use crate::types::{CycleId, SubsystemId, SubsystemStatus};

// ── Subsystem Summary Entry ─────────────────────────────────────────

/// Summary of one subsystem's contribution to a cycle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubsystemSummaryEntry {
    /// Which subsystem.
    pub subsystem_id: SubsystemId,
    /// Status during the cycle.
    pub status: SubsystemStatus,
    /// Key metrics as name-value pairs.
    pub metrics: HashMap<String, String>,
}

impl std::fmt::Display for SubsystemSummaryEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}(status={}, metrics={})",
            self.subsystem_id,
            self.status,
            self.metrics.len(),
        )
    }
}

// ── Cycle Report ────────────────────────────────────────────────────

/// Aggregated report of a cycle execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CycleReport {
    /// Which cycle this report covers.
    pub cycle_id: CycleId,
    /// Number of steps that completed.
    pub steps_completed: usize,
    /// Total steps attempted.
    pub steps_attempted: usize,
    /// Whether the cycle succeeded.
    pub success: bool,
    /// Per-subsystem summaries.
    pub subsystem_summaries: Vec<SubsystemSummaryEntry>,
    /// When the report was generated.
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl CycleReport {
    /// Generate a report from a cycle record.
    pub fn from_record(record: &CycleRecord) -> Self {
        let mut summaries: Vec<SubsystemSummaryEntry> = Vec::new();

        // Aggregate steps by subsystem
        for step in &record.steps {
            let existing = summaries
                .iter_mut()
                .find(|s| s.subsystem_id == step.subsystem);

            if let Some(entry) = existing {
                // Update existing entry
                entry.metrics.insert(
                    format!("phase_{}", step.phase),
                    if step.success {
                        "success".into()
                    } else {
                        "failed".into()
                    },
                );
                if !step.success {
                    entry.status =
                        SubsystemStatus::Failed(step.message.clone().unwrap_or_default());
                }
            } else {
                let mut metrics = HashMap::new();
                metrics.insert(
                    format!("phase_{}", step.phase),
                    if step.success {
                        "success".into()
                    } else {
                        "failed".into()
                    },
                );
                let status = if step.success {
                    SubsystemStatus::Healthy
                } else {
                    SubsystemStatus::Failed(step.message.clone().unwrap_or_default())
                };
                summaries.push(SubsystemSummaryEntry {
                    subsystem_id: step.subsystem.clone(),
                    status,
                    metrics,
                });
            }
        }

        Self {
            cycle_id: record.id.clone(),
            steps_completed: record.successful_steps(),
            steps_attempted: record.total_steps(),
            success: record.success,
            subsystem_summaries: summaries,
            generated_at: chrono::Utc::now(),
        }
    }

    /// Number of subsystems that participated.
    pub fn participating_subsystems(&self) -> usize {
        self.subsystem_summaries.len()
    }

    /// Number of healthy subsystems in the report.
    pub fn healthy_subsystems(&self) -> usize {
        self.subsystem_summaries
            .iter()
            .filter(|s| s.status.is_healthy())
            .count()
    }
}

impl std::fmt::Display for CycleReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.success { "success" } else { "failed" };
        write!(
            f,
            "CycleReport({}, steps={}/{}, subsystems={}, {})",
            self.cycle_id,
            self.steps_completed,
            self.steps_attempted,
            self.participating_subsystems(),
            status,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cycle::{CycleRunner, SimulatedCycleRunner};
    use crate::health::{HealthChecker, SimulatedHealthChecker};
    use crate::types::EvosConfig;

    fn run_cycle(fail_phases: Vec<u8>, abort: bool) -> CycleRecord {
        let runner = SimulatedCycleRunner::failing_at(fail_phases);
        let checker = SimulatedHealthChecker::all_healthy();
        let report = checker.check_all().unwrap();
        let config = EvosConfig {
            abort_on_failure: abort,
            ..EvosConfig::default()
        };
        runner.run_cycle(&report, &config).unwrap()
    }

    #[test]
    fn report_from_successful_cycle() {
        let record = run_cycle(vec![], true);
        let report = CycleReport::from_record(&record);
        assert!(report.success);
        assert_eq!(report.steps_completed, 8);
        assert_eq!(report.steps_attempted, 8);
    }

    #[test]
    fn report_from_failed_cycle() {
        let record = run_cycle(vec![3], true); // Abort at Committing
        let report = CycleReport::from_record(&record);
        assert!(!report.success);
        assert_eq!(report.steps_attempted, 4);
    }

    #[test]
    fn report_subsystem_summaries() {
        let record = run_cycle(vec![], true);
        let report = CycleReport::from_record(&record);
        // Observation appears twice (first phase + Complete), others once
        assert!(report.participating_subsystems() >= 7);
    }

    #[test]
    fn report_display() {
        let record = run_cycle(vec![], true);
        let report = CycleReport::from_record(&record);
        let display = report.to_string();
        assert!(display.contains("cycle:"));
        assert!(display.contains("success"));
    }

    #[test]
    fn report_healthy_subsystems() {
        let record = run_cycle(vec![], true);
        let report = CycleReport::from_record(&record);
        assert_eq!(report.healthy_subsystems(), report.participating_subsystems());
    }

    #[test]
    fn report_with_failures_no_abort() {
        let record = run_cycle(vec![2, 5], false);
        let report = CycleReport::from_record(&record);
        assert!(!report.success);
        assert_eq!(report.steps_attempted, 8);
        assert_eq!(report.steps_completed, 6); // 8 - 2 failures
    }

    #[test]
    fn subsystem_summary_display() {
        let entry = SubsystemSummaryEntry {
            subsystem_id: SubsystemId::Compiler,
            status: SubsystemStatus::Healthy,
            metrics: HashMap::new(),
        };
        let display = entry.to_string();
        assert!(display.contains("compiler"));
        assert!(display.contains("healthy"));
    }

    #[test]
    fn report_metrics_populated() {
        let record = run_cycle(vec![], true);
        let report = CycleReport::from_record(&record);
        // Each subsystem entry should have at least one metric
        for entry in &report.subsystem_summaries {
            assert!(!entry.metrics.is_empty(), "no metrics for {}", entry.subsystem_id);
        }
    }
}
