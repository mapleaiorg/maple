//! Benchmark measurement for conformance checks.
//!
//! Measures invariant check latencies and produces benchmark results.

use crate::invariants::{check_invariant, ALL_WORLDLINE_INVARIANT_IDS};
use crate::types::InvariantCategory;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Result of benchmarking a single invariant check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Invariant ID.
    pub invariant_id: String,
    /// Category.
    pub category: InvariantCategory,
    /// Elapsed time in microseconds.
    pub elapsed_us: u64,
    /// Whether the check passed.
    pub passed: bool,
    /// When the benchmark was run.
    pub measured_at: DateTime<Utc>,
}

/// Trait for running benchmarks.
pub trait BenchmarkRunner {
    /// Benchmark all invariants.
    fn benchmark_all(&self) -> Vec<BenchmarkResult>;

    /// Benchmark a single invariant.
    fn benchmark_one(&self, invariant_id: &str) -> BenchmarkResult;

    /// Runner name.
    fn name(&self) -> &str;
}

/// Simulated benchmark runner using wall-clock timing.
pub struct SimulatedBenchmarkRunner;

impl SimulatedBenchmarkRunner {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimulatedBenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkRunner for SimulatedBenchmarkRunner {
    fn benchmark_all(&self) -> Vec<BenchmarkResult> {
        ALL_WORLDLINE_INVARIANT_IDS
            .iter()
            .map(|id| self.benchmark_one(id))
            .collect()
    }

    fn benchmark_one(&self, invariant_id: &str) -> BenchmarkResult {
        let start = Instant::now();
        let result = check_invariant(invariant_id);
        let elapsed = start.elapsed();

        BenchmarkResult {
            invariant_id: invariant_id.to_string(),
            category: result.category,
            elapsed_us: elapsed.as_micros() as u64,
            passed: result.passed,
            measured_at: Utc::now(),
        }
    }

    fn name(&self) -> &str {
        "simulated-benchmark"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result_fields() {
        let r = BenchmarkResult {
            invariant_id: "I.OBS-1".into(),
            category: InvariantCategory::Observation,
            elapsed_us: 42,
            passed: true,
            measured_at: Utc::now(),
        };
        assert_eq!(r.invariant_id, "I.OBS-1");
        assert!(r.passed);
    }

    #[test]
    fn test_simulated_runner_name() {
        let runner = SimulatedBenchmarkRunner::new();
        assert_eq!(runner.name(), "simulated-benchmark");
    }

    #[test]
    fn test_benchmark_one() {
        let runner = SimulatedBenchmarkRunner::new();
        let result = runner.benchmark_one("I.OBS-1");
        assert_eq!(result.invariant_id, "I.OBS-1");
        assert!(result.passed);
    }

    #[test]
    fn test_benchmark_all_count() {
        let runner = SimulatedBenchmarkRunner::new();
        let results = runner.benchmark_all();
        assert_eq!(results.len(), 22);
    }

    #[test]
    fn test_benchmark_all_passed() {
        let runner = SimulatedBenchmarkRunner::new();
        let results = runner.benchmark_all();
        let all_pass = results.iter().all(|r| r.passed);
        assert!(all_pass, "not all benchmarked invariants passed");
    }

    #[test]
    fn test_benchmark_timing_positive() {
        let runner = SimulatedBenchmarkRunner::new();
        let result = runner.benchmark_one("I.OBS-1");
        // Elapsed should be >= 0 (always true for u64, but confirms measurement ran)
        assert!(result.elapsed_us < 10_000_000, "benchmark took too long");
    }

    #[test]
    fn test_benchmark_serialization() {
        let r = BenchmarkResult {
            invariant_id: "I.OBS-1".into(),
            category: InvariantCategory::Observation,
            elapsed_us: 42,
            passed: true,
            measured_at: Utc::now(),
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("I.OBS-1"));
        let deserialized: BenchmarkResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.invariant_id, "I.OBS-1");
    }
}
