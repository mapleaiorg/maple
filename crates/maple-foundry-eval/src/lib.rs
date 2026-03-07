//! MAPLE Foundry Eval -- evaluation pipeline for model quality assessment.
//!
//! Defines evaluation suites, tasks with scoring rubrics, and pipelines
//! for running suites with comparison and quality gating.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum EvalError {
    #[error("suite not found: {0}")]
    SuiteNotFound(String),
    #[error("task evaluation failed: {0}")]
    TaskFailed(String),
    #[error("quality gate failed: {0}")]
    GateFailed(String),
    #[error("eval error: {0}")]
    Internal(String),
}

pub type EvalResult<T> = Result<T, EvalError>;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Method used to score an evaluation task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoringMethod {
    ExactMatch,
    Contains,
    LLMJudge,
    Custom(String),
}

/// A single evaluation task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalTask {
    pub id: String,
    pub input: String,
    pub expected_output: Option<String>,
    pub scoring_rubric: String,
    pub scoring_method: ScoringMethod,
    pub weight: f64,
}

impl EvalTask {
    pub fn new(id: impl Into<String>, input: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            input: input.into(),
            expected_output: None,
            scoring_rubric: String::new(),
            scoring_method: ScoringMethod::ExactMatch,
            weight: 1.0,
        }
    }

    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected_output = Some(expected.into());
        self
    }

    pub fn with_scoring(mut self, method: ScoringMethod) -> Self {
        self.scoring_method = method;
        self
    }
}

/// A suite of evaluation tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSuite {
    pub name: String,
    pub version: String,
    pub tasks: Vec<EvalTask>,
    pub description: String,
}

impl EvalSuite {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            tasks: Vec::new(),
            description: String::new(),
        }
    }

    pub fn add_task(&mut self, task: EvalTask) {
        self.tasks.push(task);
    }
}

/// Result of evaluating a single task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalTaskResult {
    pub task_id: String,
    pub score: f64,
    pub latency: Duration,
    pub model: String,
    pub output: String,
    pub details: HashMap<String, String>,
}

/// Aggregate evaluation result for a suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSuiteResult {
    pub suite_name: String,
    pub model: String,
    pub task_results: Vec<EvalTaskResult>,
    pub avg_score: f64,
    pub weighted_score: f64,
    pub total_latency: Duration,
    pub evaluated_at: DateTime<Utc>,
}

/// Comparison between two eval runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalComparison {
    pub baseline_model: String,
    pub candidate_model: String,
    pub score_delta: f64,
    pub latency_delta_ms: i64,
    pub improved_tasks: Vec<String>,
    pub regressed_tasks: Vec<String>,
}

// ---------------------------------------------------------------------------
// Scorer trait
// ---------------------------------------------------------------------------

/// Trait for implementing custom scoring.
#[async_trait::async_trait]
pub trait Scorer: Send + Sync {
    async fn score(&self, task: &EvalTask, output: &str) -> EvalResult<f64>;
}

/// Default scorer using built-in methods.
pub struct DefaultScorer;

impl DefaultScorer {
    pub fn score_sync(&self, task: &EvalTask, output: &str) -> f64 {
        match &task.scoring_method {
            ScoringMethod::ExactMatch => {
                if let Some(expected) = &task.expected_output {
                    if output.trim() == expected.trim() { 1.0 } else { 0.0 }
                } else {
                    0.5
                }
            }
            ScoringMethod::Contains => {
                if let Some(expected) = &task.expected_output {
                    if output.contains(expected.as_str()) { 1.0 } else { 0.0 }
                } else {
                    0.5
                }
            }
            ScoringMethod::LLMJudge => 0.75, // Mock LLM judge
            ScoringMethod::Custom(_) => 0.5,  // Mock custom
        }
    }
}

// ---------------------------------------------------------------------------
// Eval Pipeline
// ---------------------------------------------------------------------------

/// Runs evaluation suites and manages results.
pub struct EvalPipeline {
    suites: HashMap<String, EvalSuite>,
    results: Vec<EvalSuiteResult>,
    scorer: DefaultScorer,
}

impl Default for EvalPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl EvalPipeline {
    pub fn new() -> Self {
        Self {
            suites: HashMap::new(),
            results: Vec::new(),
            scorer: DefaultScorer,
        }
    }

    /// Register a suite.
    pub fn register_suite(&mut self, suite: EvalSuite) {
        self.suites.insert(suite.name.clone(), suite);
    }

    /// Run a suite against a model (mock execution).
    pub fn run_suite(&mut self, suite_name: &str, model: &str) -> EvalResult<EvalSuiteResult> {
        let suite = self
            .suites
            .get(suite_name)
            .ok_or_else(|| EvalError::SuiteNotFound(suite_name.to_string()))?
            .clone();

        let mut task_results = Vec::new();
        let mut total_score = 0.0;
        let mut total_weighted = 0.0;
        let mut total_weight = 0.0;
        let mut total_latency = Duration::ZERO;

        for task in &suite.tasks {
            let mock_output = format!("Output for {}", task.id);
            let score = self.scorer.score_sync(task, &mock_output);
            let latency = Duration::from_millis(25);

            total_score += score;
            total_weighted += score * task.weight;
            total_weight += task.weight;
            total_latency += latency;

            task_results.push(EvalTaskResult {
                task_id: task.id.clone(),
                score,
                latency,
                model: model.to_string(),
                output: mock_output,
                details: HashMap::new(),
            });
        }

        let count = suite.tasks.len().max(1) as f64;
        let result = EvalSuiteResult {
            suite_name: suite_name.to_string(),
            model: model.to_string(),
            task_results,
            avg_score: total_score / count,
            weighted_score: if total_weight > 0.0 {
                total_weighted / total_weight
            } else {
                0.0
            },
            total_latency,
            evaluated_at: Utc::now(),
        };

        self.results.push(result.clone());
        Ok(result)
    }

    /// Compare two suite results.
    pub fn compare(baseline: &EvalSuiteResult, candidate: &EvalSuiteResult) -> EvalComparison {
        let mut improved = Vec::new();
        let mut regressed = Vec::new();

        for (b, c) in baseline.task_results.iter().zip(candidate.task_results.iter()) {
            if c.score > b.score {
                improved.push(c.task_id.clone());
            } else if c.score < b.score {
                regressed.push(c.task_id.clone());
            }
        }

        EvalComparison {
            baseline_model: baseline.model.clone(),
            candidate_model: candidate.model.clone(),
            score_delta: candidate.avg_score - baseline.avg_score,
            latency_delta_ms: candidate.total_latency.as_millis() as i64
                - baseline.total_latency.as_millis() as i64,
            improved_tasks: improved,
            regressed_tasks: regressed,
        }
    }

    /// Quality gate: check if a result meets minimum score.
    pub fn gate(&self, result: &EvalSuiteResult, min_score: f64) -> EvalResult<()> {
        if result.avg_score < min_score {
            return Err(EvalError::GateFailed(format!(
                "avg_score {:.2} below threshold {:.2}",
                result.avg_score, min_score
            )));
        }
        Ok(())
    }

    /// Get all results.
    pub fn results(&self) -> &[EvalSuiteResult] {
        &self.results
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_suite() -> EvalSuite {
        let mut suite = EvalSuite::new("test-suite", "1.0");
        suite.add_task(
            EvalTask::new("t1", "What is 2+2?")
                .with_expected("4")
                .with_scoring(ScoringMethod::ExactMatch),
        );
        suite.add_task(
            EvalTask::new("t2", "Describe Rust")
                .with_scoring(ScoringMethod::LLMJudge),
        );
        suite
    }

    #[test]
    fn test_run_suite() {
        let mut pipeline = EvalPipeline::new();
        pipeline.register_suite(make_suite());
        let result = pipeline.run_suite("test-suite", "gpt-4").unwrap();
        assert_eq!(result.task_results.len(), 2);
    }

    #[test]
    fn test_suite_not_found() {
        let mut pipeline = EvalPipeline::new();
        assert!(pipeline.run_suite("nonexistent", "model").is_err());
    }

    #[test]
    fn test_exact_match_scoring() {
        let scorer = DefaultScorer;
        let task = EvalTask::new("t1", "input").with_expected("hello");
        assert!((scorer.score_sync(&task, "hello") - 1.0).abs() < f64::EPSILON);
        assert!((scorer.score_sync(&task, "world") - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_contains_scoring() {
        let scorer = DefaultScorer;
        let task = EvalTask::new("t1", "input")
            .with_expected("hello")
            .with_scoring(ScoringMethod::Contains);
        assert!((scorer.score_sync(&task, "say hello world") - 1.0).abs() < f64::EPSILON);
        assert!((scorer.score_sync(&task, "goodbye") - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_quality_gate_pass() {
        let mut pipeline = EvalPipeline::new();
        pipeline.register_suite(make_suite());
        let result = pipeline.run_suite("test-suite", "model").unwrap();
        // LLM judge gives 0.75, exact match will give 0 for mock output
        // gate at 0.1 should pass
        assert!(pipeline.gate(&result, 0.1).is_ok());
    }

    #[test]
    fn test_quality_gate_fail() {
        let mut pipeline = EvalPipeline::new();
        pipeline.register_suite(make_suite());
        let result = pipeline.run_suite("test-suite", "model").unwrap();
        assert!(pipeline.gate(&result, 0.99).is_err());
    }

    #[test]
    fn test_compare() {
        let mut pipeline = EvalPipeline::new();
        pipeline.register_suite(make_suite());
        let r1 = pipeline.run_suite("test-suite", "model-a").unwrap();
        let r2 = pipeline.run_suite("test-suite", "model-b").unwrap();
        let cmp = EvalPipeline::compare(&r1, &r2);
        assert_eq!(cmp.baseline_model, "model-a");
        assert_eq!(cmp.candidate_model, "model-b");
    }

    #[test]
    fn test_results_history() {
        let mut pipeline = EvalPipeline::new();
        pipeline.register_suite(make_suite());
        pipeline.run_suite("test-suite", "a").unwrap();
        pipeline.run_suite("test-suite", "b").unwrap();
        assert_eq!(pipeline.results().len(), 2);
    }

    #[test]
    fn test_eval_task_builder() {
        let task = EvalTask::new("t1", "input")
            .with_expected("output")
            .with_scoring(ScoringMethod::Contains);
        assert_eq!(task.expected_output.as_deref(), Some("output"));
        assert_eq!(task.scoring_method, ScoringMethod::Contains);
    }

    #[test]
    fn test_scoring_method_variants() {
        assert_ne!(ScoringMethod::ExactMatch, ScoringMethod::Contains);
        assert_eq!(ScoringMethod::Custom("a".into()), ScoringMethod::Custom("a".into()));
    }

    #[test]
    fn test_weighted_score() {
        let mut suite = EvalSuite::new("weighted", "1.0");
        let mut t1 = EvalTask::new("t1", "input").with_scoring(ScoringMethod::LLMJudge);
        t1.weight = 3.0;
        let mut t2 = EvalTask::new("t2", "input").with_scoring(ScoringMethod::LLMJudge);
        t2.weight = 1.0;
        suite.add_task(t1);
        suite.add_task(t2);

        let mut pipeline = EvalPipeline::new();
        pipeline.register_suite(suite);
        let result = pipeline.run_suite("weighted", "model").unwrap();
        // Both LLM judge tasks score 0.75, so weighted should also be 0.75
        assert!((result.weighted_score - 0.75).abs() < 0.01);
    }
}
