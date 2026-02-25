//! Conformance test runner.
//!
//! Orchestrates running invariant checks according to a configuration,
//! producing a full conformance report.

use crate::benchmarks::{BenchmarkRunner, SimulatedBenchmarkRunner};
use crate::error::{ConformanceError, ConformanceResult};
use crate::invariants::{self, check_invariant, ALL_WORLDLINE_INVARIANT_IDS};
use crate::report::ConformanceReport;
use crate::types::{ConformanceConfig, InvariantCategory, InvariantResult};
use chrono::Utc;

/// Runs WorldLine conformance checks.
pub struct ConformanceRunner {
    config: ConformanceConfig,
}

impl ConformanceRunner {
    /// Create a runner with default config (all invariants, no fail-fast).
    pub fn new() -> Self {
        Self {
            config: ConformanceConfig::default(),
        }
    }

    /// Create a runner with a specific configuration.
    pub fn with_config(config: ConformanceConfig) -> Self {
        Self { config }
    }

    /// Run all 22 invariants (or filtered subset) and produce a report.
    pub fn run_all(&self) -> ConformanceResult<ConformanceReport> {
        let started_at = Utc::now();
        let ids = self.resolve_ids()?;
        let total_expected = ids.len();
        let mut results = Vec::new();

        for id in &ids {
            let result = check_invariant(id);
            let failed = !result.passed;
            results.push(result);

            if self.config.fail_fast && failed {
                break;
            }
        }

        let skipped = total_expected - results.len();
        let completed_at = Utc::now();
        let mut report =
            ConformanceReport::from_results(results, skipped, started_at, completed_at);

        if self.config.include_benchmarks {
            let benchmark_runner = SimulatedBenchmarkRunner::new();
            let benchmark_results = benchmark_runner.benchmark_all();
            report = report.with_benchmarks(benchmark_results);
        }

        Ok(report)
    }

    /// Run all invariants in a specific category.
    pub fn run_category(
        &self,
        category: InvariantCategory,
    ) -> ConformanceResult<ConformanceReport> {
        let started_at = Utc::now();
        let ids = invariants::ids_for_category(category);

        if ids.is_empty() {
            return Err(ConformanceError::InvalidConfiguration(format!(
                "no invariants found for category {:?}",
                category
            )));
        }

        let total_expected = ids.len();
        let mut results = Vec::new();
        for id in &ids {
            let result = check_invariant(id);
            let failed = !result.passed;
            results.push(result);

            if self.config.fail_fast && failed {
                break;
            }
        }

        let skipped = total_expected - results.len();
        let completed_at = Utc::now();
        Ok(ConformanceReport::from_results(
            results,
            skipped,
            started_at,
            completed_at,
        ))
    }

    /// Run a single invariant by ID.
    pub fn run_single(&self, id: &str) -> ConformanceResult<InvariantResult> {
        if invariants::category_for(id).is_none() {
            return Err(ConformanceError::InvalidConfiguration(format!(
                "unknown invariant ID: {}",
                id
            )));
        }
        Ok(check_invariant(id))
    }

    /// Resolve which IDs to check based on config filters.
    fn resolve_ids(&self) -> ConformanceResult<Vec<&'static str>> {
        // If specific IDs are requested, use those.
        if !self.config.invariant_ids.is_empty() {
            let mut ids = Vec::new();
            for requested in &self.config.invariant_ids {
                let found = ALL_WORLDLINE_INVARIANT_IDS
                    .iter()
                    .find(|&&id| id == requested.as_str());
                match found {
                    Some(id) => ids.push(*id),
                    None => {
                        return Err(ConformanceError::InvalidConfiguration(format!(
                            "unknown invariant ID: {}",
                            requested
                        )));
                    }
                }
            }
            return Ok(ids);
        }

        // If categories are filtered, collect IDs from those categories.
        if !self.config.categories.is_empty() {
            let mut ids = Vec::new();
            for cat in &self.config.categories {
                ids.extend(invariants::ids_for_category(*cat));
            }
            return Ok(ids);
        }

        // Default: all invariants.
        Ok(ALL_WORLDLINE_INVARIANT_IDS.to_vec())
    }
}

impl Default for ConformanceRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::InvariantId;

    #[test]
    fn test_runner_default() {
        let runner = ConformanceRunner::new();
        assert!(!runner.config.fail_fast);
    }

    #[test]
    fn test_run_all() {
        let runner = ConformanceRunner::new();
        let report = runner.run_all().unwrap();
        assert_eq!(report.summary.total, 22);
    }

    #[test]
    fn test_run_category_observation() {
        let runner = ConformanceRunner::new();
        let report = runner.run_category(InvariantCategory::Observation).unwrap();
        assert_eq!(report.summary.total, 5);
    }

    #[test]
    fn test_run_category_self_mod_gate() {
        let runner = ConformanceRunner::new();
        let report = runner.run_category(InvariantCategory::SelfModGate).unwrap();
        assert_eq!(report.summary.total, 7);
    }

    #[test]
    fn test_run_single_valid() {
        let runner = ConformanceRunner::new();
        let result = runner.run_single("I.OBS-1").unwrap();
        assert_eq!(result.id.as_str(), "I.OBS-1");
    }

    #[test]
    fn test_run_single_invalid() {
        let runner = ConformanceRunner::new();
        let result = runner.run_single("I.NONEXISTENT");
        assert!(result.is_err());
    }

    #[test]
    fn test_filtered_by_ids() {
        let config = ConformanceConfig {
            invariant_ids: vec![InvariantId::new("I.OBS-1"), InvariantId::new("I.EVOS-2")],
            ..Default::default()
        };
        let runner = ConformanceRunner::with_config(config);
        let report = runner.run_all().unwrap();
        assert_eq!(report.summary.total, 2);
    }

    #[test]
    fn test_filtered_by_category() {
        let config = ConformanceConfig {
            categories: vec![InvariantCategory::Bootstrap],
            ..Default::default()
        };
        let runner = ConformanceRunner::with_config(config);
        let report = runner.run_all().unwrap();
        assert_eq!(report.summary.total, 2);
    }

    #[test]
    fn test_run_all_fail_fast_no_failures_skipped_zero() {
        let config = ConformanceConfig {
            fail_fast: true,
            ..Default::default()
        };
        let runner = ConformanceRunner::with_config(config);
        let report = runner.run_all().unwrap();
        assert_eq!(report.summary.skipped, 0);
        assert_eq!(report.summary.total, 22);
    }

    #[test]
    fn test_run_all_with_benchmarks() {
        let config = ConformanceConfig {
            include_benchmarks: true,
            ..Default::default()
        };
        let runner = ConformanceRunner::with_config(config);
        let report = runner.run_all().unwrap();
        assert!(report.benchmarks.is_some());
        assert_eq!(report.benchmarks.unwrap().len(), 22);
    }

    #[test]
    fn test_run_all_without_benchmarks() {
        let runner = ConformanceRunner::new();
        let report = runner.run_all().unwrap();
        assert!(report.benchmarks.is_none());
    }
}
