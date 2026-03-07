//! MAPLE Model Benchmark -- model benchmarking and quality gates.
//!
//! Defines benchmark suites with tasks and metrics, runs benchmarks,
//! and evaluates results against quality gates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum BenchmarkError {
    #[error("suite not found: {0}")]
    SuiteNotFound(String),
    #[error("task failed: {0}")]
    TaskFailed(String),
    #[error("quality gate failed: {gate} (required: {required}, actual: {actual})")]
    QualityGateFailed { gate: String, required: String, actual: String },
    #[error("benchmark error: {0}")]
    Internal(String),
}

pub type BenchmarkResult<T> = Result<T, BenchmarkError>;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// A single benchmark task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTask {
    pub id: String,
    pub prompt: String,
    pub expected_output: Option<String>,
    pub timeout: Duration,
    pub category: String,
}

/// A suite of benchmark tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    pub name: String,
    pub description: String,
    pub tasks: Vec<BenchmarkTask>,
    pub metrics: Vec<String>,
}

impl BenchmarkSuite {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            tasks: Vec::new(),
            metrics: vec!["latency".into(), "quality_score".into(), "throughput".into()],
        }
    }

    pub fn add_task(&mut self, task: BenchmarkTask) {
        self.tasks.push(task);
    }
}

/// Result of running a single benchmark task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub latency: Duration,
    pub output: String,
    pub quality_score: f64,
    pub token_usage: u32,
    pub success: bool,
}

/// Aggregate result of running a benchmark suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRunResult {
    pub suite_name: String,
    pub model: String,
    pub task_results: Vec<TaskResult>,
    pub avg_latency: Duration,
    pub avg_quality_score: f64,
    pub throughput: f64,
    pub total_tokens: u32,
    pub pass_rate: f64,
    pub run_at: DateTime<Utc>,
}

/// Quality gate that a model must pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGate {
    pub name: String,
    pub min_score: Option<f64>,
    pub max_latency_ms: Option<u64>,
    pub max_cost_per_token: Option<f64>,
    pub min_pass_rate: Option<f64>,
}

impl QualityGate {
    /// Check if a benchmark result passes this gate.
    pub fn check(&self, result: &BenchmarkRunResult) -> BenchmarkResult<()> {
        if let Some(min) = self.min_score {
            if result.avg_quality_score < min {
                return Err(BenchmarkError::QualityGateFailed {
                    gate: self.name.clone(),
                    required: format!("min_score >= {:.2}", min),
                    actual: format!("{:.2}", result.avg_quality_score),
                });
            }
        }
        if let Some(max_ms) = self.max_latency_ms {
            if result.avg_latency.as_millis() as u64 > max_ms {
                return Err(BenchmarkError::QualityGateFailed {
                    gate: self.name.clone(),
                    required: format!("latency <= {}ms", max_ms),
                    actual: format!("{}ms", result.avg_latency.as_millis()),
                });
            }
        }
        if let Some(min_rate) = self.min_pass_rate {
            if result.pass_rate < min_rate {
                return Err(BenchmarkError::QualityGateFailed {
                    gate: self.name.clone(),
                    required: format!("pass_rate >= {:.2}", min_rate),
                    actual: format!("{:.2}", result.pass_rate),
                });
            }
        }
        Ok(())
    }
}

/// Comparison of two benchmark runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub baseline: String,
    pub candidate: String,
    pub latency_delta_ms: i64,
    pub quality_delta: f64,
    pub throughput_delta: f64,
    pub candidate_better: bool,
}

// ---------------------------------------------------------------------------
// Benchmark Runner
// ---------------------------------------------------------------------------

/// Runs benchmarks and evaluates quality gates.
pub struct BenchmarkRunner {
    results: Vec<BenchmarkRunResult>,
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Run a benchmark suite (mock execution).
    pub fn run(&mut self, suite: &BenchmarkSuite, model: &str) -> BenchmarkRunResult {
        let mut task_results = Vec::new();
        let mut total_latency = Duration::ZERO;
        let mut total_score = 0.0;
        let mut total_tokens = 0u32;
        let mut pass_count = 0u32;

        for task in &suite.tasks {
            let latency = Duration::from_millis(50); // Mock latency
            let score = if task.expected_output.is_some() { 0.85 } else { 0.75 };
            let tokens = 100u32;

            let result = TaskResult {
                task_id: task.id.clone(),
                latency,
                output: format!("Mock response for task {}", task.id),
                quality_score: score,
                token_usage: tokens,
                success: true,
            };
            total_latency += latency;
            total_score += score;
            total_tokens += tokens;
            pass_count += 1;
            task_results.push(result);
        }

        let count = suite.tasks.len().max(1) as f64;
        let run_result = BenchmarkRunResult {
            suite_name: suite.name.clone(),
            model: model.to_string(),
            task_results,
            avg_latency: total_latency / count as u32,
            avg_quality_score: total_score / count,
            throughput: total_tokens as f64 / total_latency.as_secs_f64().max(0.001),
            total_tokens,
            pass_rate: pass_count as f64 / count,
            run_at: Utc::now(),
        };

        self.results.push(run_result.clone());
        run_result
    }

    /// Compare two benchmark results.
    pub fn compare(&self, baseline: &BenchmarkRunResult, candidate: &BenchmarkRunResult) -> BenchmarkComparison {
        let latency_delta = candidate.avg_latency.as_millis() as i64 - baseline.avg_latency.as_millis() as i64;
        let quality_delta = candidate.avg_quality_score - baseline.avg_quality_score;
        let throughput_delta = candidate.throughput - baseline.throughput;

        BenchmarkComparison {
            baseline: baseline.model.clone(),
            candidate: candidate.model.clone(),
            latency_delta_ms: latency_delta,
            quality_delta,
            throughput_delta,
            candidate_better: quality_delta >= 0.0 && latency_delta <= 0,
        }
    }

    /// Get all results.
    pub fn results(&self) -> &[BenchmarkRunResult] {
        &self.results
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_suite() -> BenchmarkSuite {
        let mut suite = BenchmarkSuite::new("basic-suite");
        suite.add_task(BenchmarkTask {
            id: "t1".into(),
            prompt: "What is 2+2?".into(),
            expected_output: Some("4".into()),
            timeout: Duration::from_secs(30),
            category: "math".into(),
        });
        suite.add_task(BenchmarkTask {
            id: "t2".into(),
            prompt: "Hello world".into(),
            expected_output: None,
            timeout: Duration::from_secs(30),
            category: "general".into(),
        });
        suite
    }

    #[test]
    fn test_run_benchmark() {
        let mut runner = BenchmarkRunner::new();
        let suite = make_suite();
        let result = runner.run(&suite, "gpt-4");
        assert_eq!(result.task_results.len(), 2);
        assert_eq!(result.model, "gpt-4");
    }

    #[test]
    fn test_quality_gate_pass() {
        let mut runner = BenchmarkRunner::new();
        let suite = make_suite();
        let result = runner.run(&suite, "gpt-4");
        let gate = QualityGate {
            name: "basic".into(),
            min_score: Some(0.5),
            max_latency_ms: Some(1000),
            max_cost_per_token: None,
            min_pass_rate: Some(0.8),
        };
        assert!(gate.check(&result).is_ok());
    }

    #[test]
    fn test_quality_gate_fail_score() {
        let mut runner = BenchmarkRunner::new();
        let suite = make_suite();
        let result = runner.run(&suite, "gpt-4");
        let gate = QualityGate {
            name: "strict".into(),
            min_score: Some(0.99),
            max_latency_ms: None,
            max_cost_per_token: None,
            min_pass_rate: None,
        };
        assert!(gate.check(&result).is_err());
    }

    #[test]
    fn test_compare_benchmarks() {
        let mut runner = BenchmarkRunner::new();
        let suite = make_suite();
        let baseline = runner.run(&suite, "model-a");
        let candidate = runner.run(&suite, "model-b");
        let cmp = runner.compare(&baseline, &candidate);
        assert_eq!(cmp.baseline, "model-a");
        assert_eq!(cmp.candidate, "model-b");
    }

    #[test]
    fn test_suite_creation() {
        let suite = BenchmarkSuite::new("test");
        assert_eq!(suite.name, "test");
        assert!(suite.tasks.is_empty());
        assert!(!suite.metrics.is_empty());
    }

    #[test]
    fn test_results_history() {
        let mut runner = BenchmarkRunner::new();
        let suite = make_suite();
        runner.run(&suite, "a");
        runner.run(&suite, "b");
        assert_eq!(runner.results().len(), 2);
    }

    #[test]
    fn test_task_result_fields() {
        let mut runner = BenchmarkRunner::new();
        let suite = make_suite();
        let result = runner.run(&suite, "gpt-4");
        let tr = &result.task_results[0];
        assert!(tr.success);
        assert!(tr.quality_score > 0.0);
        assert!(tr.token_usage > 0);
    }

    #[test]
    fn test_avg_quality_score() {
        let mut runner = BenchmarkRunner::new();
        let suite = make_suite();
        let result = runner.run(&suite, "gpt-4");
        assert!(result.avg_quality_score > 0.0);
        assert!(result.avg_quality_score <= 1.0);
    }

    #[test]
    fn test_pass_rate() {
        let mut runner = BenchmarkRunner::new();
        let suite = make_suite();
        let result = runner.run(&suite, "gpt-4");
        assert!((result.pass_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_empty_suite() {
        let mut runner = BenchmarkRunner::new();
        let suite = BenchmarkSuite::new("empty");
        let result = runner.run(&suite, "model");
        assert!(result.task_results.is_empty());
    }
}
