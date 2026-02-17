use crate::types::TestResult;
use async_trait::async_trait;

/// Trait for running tests in a sandboxed environment.
#[async_trait]
pub trait TestRunner: Send + Sync {
    /// Run all tests and return results.
    async fn run_tests(&self) -> Vec<TestResult>;

    /// Run a specific named test.
    async fn run_test(&self, name: &str) -> TestResult;
}

/// Simulated test runner for testing (no actual compilation/execution).
pub struct SimulatedTestRunner {
    results: Vec<TestResult>,
}

impl SimulatedTestRunner {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Configure with predetermined results.
    pub fn with_results(mut self, results: Vec<TestResult>) -> Self {
        self.results = results;
        self
    }

    /// Add a single passing test.
    pub fn with_passing_test(mut self, name: impl Into<String>) -> Self {
        self.results.push(TestResult {
            name: name.into(),
            passed: true,
            duration_ms: 1,
            error: None,
        });
        self
    }

    /// Add a single failing test.
    pub fn with_failing_test(mut self, name: impl Into<String>, error: impl Into<String>) -> Self {
        self.results.push(TestResult {
            name: name.into(),
            passed: false,
            duration_ms: 1,
            error: Some(error.into()),
        });
        self
    }

    /// All tests pass preset (for convenience).
    pub fn all_pass(count: usize) -> Self {
        let results = (0..count)
            .map(|i| TestResult {
                name: format!("test_{}", i),
                passed: true,
                duration_ms: 1,
                error: None,
            })
            .collect();
        Self { results }
    }
}

impl Default for SimulatedTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestRunner for SimulatedTestRunner {
    async fn run_tests(&self) -> Vec<TestResult> {
        self.results.clone()
    }

    async fn run_test(&self, name: &str) -> TestResult {
        self.results
            .iter()
            .find(|r| r.name == name)
            .cloned()
            .unwrap_or(TestResult {
                name: name.to_string(),
                passed: false,
                duration_ms: 0,
                error: Some("test not found".into()),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn simulated_runner_all_pass() {
        let runner = SimulatedTestRunner::all_pass(5);
        let results = runner.run_tests().await;
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.passed));
    }

    #[tokio::test]
    async fn simulated_runner_mixed() {
        let runner = SimulatedTestRunner::new()
            .with_passing_test("pass_1")
            .with_failing_test("fail_1", "assertion error");
        let results = runner.run_tests().await;
        assert_eq!(results.len(), 2);
        assert!(results[0].passed);
        assert!(!results[1].passed);
    }

    #[tokio::test]
    async fn run_specific_test() {
        let runner = SimulatedTestRunner::new().with_passing_test("target_test");
        let result = runner.run_test("target_test").await;
        assert!(result.passed);
    }

    #[tokio::test]
    async fn run_missing_test() {
        let runner = SimulatedTestRunner::new();
        let result = runner.run_test("nonexistent").await;
        assert!(!result.passed);
        assert!(result.error.unwrap().contains("not found"));
    }
}
