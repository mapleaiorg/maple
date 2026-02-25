//! EVOS engine with bounded cycle history.
//!
//! Wraps cycle execution and health checking with a bounded FIFO
//! of `CycleRecord`s for tracking EVOS operations over time.

use std::collections::VecDeque;

use crate::cycle::{CycleRecord, CycleRunner, SimulatedCycleRunner};
use crate::error::EvosResult;
use crate::health::{HealthChecker, HealthReport, SimulatedHealthChecker};
use crate::report::CycleReport;
use crate::types::{CycleId, EvosConfig, EvosSummary};

// ── EVOS Engine ─────────────────────────────────────────────────────

/// Engine orchestrating EVOS cycle execution with bounded history.
pub struct EvosEngine {
    config: EvosConfig,
    cycle_runner: Box<dyn CycleRunner>,
    health_checker: Box<dyn HealthChecker>,
    records: VecDeque<CycleRecord>,
    max_records: usize,
}

impl EvosEngine {
    /// Create with default configuration and simulated components.
    pub fn new() -> Self {
        let config = EvosConfig::default();
        let max = config.max_tracked_records;
        Self {
            config,
            cycle_runner: Box::new(SimulatedCycleRunner::all_passing()),
            health_checker: Box::new(SimulatedHealthChecker::all_healthy()),
            records: VecDeque::new(),
            max_records: max,
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: EvosConfig) -> Self {
        let max = config.max_tracked_records;
        Self {
            config,
            cycle_runner: Box::new(SimulatedCycleRunner::all_passing()),
            health_checker: Box::new(SimulatedHealthChecker::all_healthy()),
            records: VecDeque::new(),
            max_records: max,
        }
    }

    /// Set cycle runner.
    pub fn with_cycle_runner(mut self, runner: Box<dyn CycleRunner>) -> Self {
        self.cycle_runner = runner;
        self
    }

    /// Set health checker.
    pub fn with_health_checker(mut self, checker: Box<dyn HealthChecker>) -> Self {
        self.health_checker = checker;
        self
    }

    /// Run one full EVOS cycle.
    pub fn run_cycle(&mut self) -> EvosResult<CycleReport> {
        // Step 1: Health check
        let health = self.health_checker.check_all()?;

        // Step 2: Run cycle
        let record = self.cycle_runner.run_cycle(&health, &self.config)?;

        // Step 3: Generate report
        let report = CycleReport::from_record(&record);

        // Step 4: Store record with bounded eviction
        self.push_record(record);

        Ok(report)
    }

    /// Perform a health check without running a cycle.
    pub fn health_check(&self) -> EvosResult<HealthReport> {
        self.health_checker.check_all()
    }

    /// Push a record with bounded FIFO eviction.
    fn push_record(&mut self, record: CycleRecord) {
        if self.records.len() >= self.max_records {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    /// Find a cycle record by ID.
    pub fn find(&self, id: &CycleId) -> Option<&CycleRecord> {
        self.records.iter().find(|r| r.id == *id)
    }

    /// All cycle records.
    pub fn all_records(&self) -> &VecDeque<CycleRecord> {
        &self.records
    }

    /// Number of tracked records.
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Summary statistics.
    pub fn summary(&self) -> EvosSummary {
        let total = self.records.len();
        let successful = self.records.iter().filter(|r| r.success).count();
        let failed = total - successful;
        let total_steps: usize = self.records.iter().map(|r| r.total_steps()).sum();

        EvosSummary {
            total_cycles: total,
            successful_cycles: successful,
            failed_cycles: failed,
            total_steps_executed: total_steps,
            current_bootstrap_phase: 0, // Default; real impl would query bootstrap engine
        }
    }
}

impl Default for EvosEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cycle::SimulatedCycleRunner;
    use crate::health::SimulatedHealthChecker;
    use crate::types::SubsystemStatus;

    #[test]
    fn engine_starts_empty() {
        let engine = EvosEngine::new();
        assert_eq!(engine.record_count(), 0);
    }

    #[test]
    fn engine_run_one_cycle() {
        let mut engine = EvosEngine::new();
        let report = engine.run_cycle().unwrap();
        assert!(report.success);
        assert_eq!(engine.record_count(), 1);
    }

    #[test]
    fn engine_run_multiple_cycles() {
        let mut engine = EvosEngine::new();
        for _ in 0..5 {
            engine.run_cycle().unwrap();
        }
        assert_eq!(engine.record_count(), 5);
    }

    #[test]
    fn engine_bounded_fifo() {
        let config = EvosConfig {
            max_tracked_records: 3,
            ..EvosConfig::default()
        };
        let mut engine = EvosEngine::with_config(config);
        for _ in 0..5 {
            engine.run_cycle().unwrap();
        }
        assert_eq!(engine.record_count(), 3);
    }

    #[test]
    fn engine_summary() {
        let mut engine = EvosEngine::new();
        engine.run_cycle().unwrap();
        engine.run_cycle().unwrap();
        let summary = engine.summary();
        assert_eq!(summary.total_cycles, 2);
        assert_eq!(summary.successful_cycles, 2);
        assert_eq!(summary.failed_cycles, 0);
        assert_eq!(summary.total_steps_executed, 16); // 8 steps × 2 cycles
    }

    #[test]
    fn engine_health_check_only() {
        let engine = EvosEngine::new();
        let health = engine.health_check().unwrap();
        assert_eq!(health.healthy_count(), 14);
    }

    #[test]
    fn engine_with_failing_runner() {
        let mut engine = EvosEngine::new()
            .with_cycle_runner(Box::new(SimulatedCycleRunner::failing_at(vec![2])));
        let report = engine.run_cycle().unwrap();
        assert!(!report.success);
        let summary = engine.summary();
        assert_eq!(summary.failed_cycles, 1);
    }

    #[test]
    fn engine_with_unhealthy_checker() {
        let mut engine = EvosEngine::new().with_health_checker(Box::new(
            SimulatedHealthChecker::with_overrides(vec![(
                crate::types::SubsystemId::Observation,
                SubsystemStatus::Failed("crash".into()),
            )]),
        ));
        let result = engine.run_cycle();
        assert!(result.is_err()); // Blocked by health check
    }
}
