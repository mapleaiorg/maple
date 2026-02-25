//! Sandbox compiler — trait and simulated implementation.
//!
//! The `SandboxCompiler` trait abstracts sandbox compilation, test
//! execution, and performance evaluation. Real implementations would
//! write to a temp directory and invoke `cargo check`/`cargo test`.
//! The `SimulatedSandbox` returns configurable results for testing.

use maple_worldline_intent::proposal::{Comparison, PerformanceGate, TestSpec};

use crate::error::CodegenResult;
use crate::types::{CompilationResult, GeneratedCode, PerformanceResult, TestResult};

// ── SandboxCompiler Trait ──────────────────────────────────────────────

/// Trait for sandbox compilation and validation.
///
/// Real implementations would write files to a temp directory, invoke
/// the Rust compiler, run tests, and measure performance.
pub trait SandboxCompiler: Send + Sync {
    /// Compile a single generated code artifact in a sandbox.
    fn compile(&self, code: &GeneratedCode) -> CodegenResult<CompilationResult>;

    /// Run tests against the compiled code.
    fn run_tests(
        &self,
        code: &[GeneratedCode],
        tests: &[TestSpec],
    ) -> CodegenResult<Vec<TestResult>>;

    /// Evaluate performance gates.
    fn evaluate_performance(
        &self,
        code: &[GeneratedCode],
        gates: &[PerformanceGate],
    ) -> CodegenResult<Vec<PerformanceResult>>;

    /// Name of this sandbox for logging.
    fn name(&self) -> &str;
}

// ── Simulated Sandbox ──────────────────────────────────────────────────

/// A simulated sandbox compiler for testing.
///
/// Each stage (compilation, tests, performance) can be independently
/// configured to succeed or fail.
pub struct SimulatedSandbox {
    compilation_succeeds: bool,
    tests_succeed: bool,
    performance_succeeds: bool,
}

impl SimulatedSandbox {
    /// Create a sandbox with per-stage configuration.
    pub fn new(compilation: bool, tests: bool, performance: bool) -> Self {
        Self {
            compilation_succeeds: compilation,
            tests_succeed: tests,
            performance_succeeds: performance,
        }
    }

    /// Convenience: all stages pass.
    pub fn all_pass() -> Self {
        Self::new(true, true, true)
    }

    /// Convenience: only compilation fails.
    pub fn compilation_fails() -> Self {
        Self::new(false, true, true)
    }

    /// Convenience: only tests fail.
    pub fn tests_fail() -> Self {
        Self::new(true, false, true)
    }

    /// Convenience: only performance gates fail.
    pub fn performance_fails() -> Self {
        Self::new(true, true, false)
    }
}

impl SandboxCompiler for SimulatedSandbox {
    fn compile(&self, code: &GeneratedCode) -> CodegenResult<CompilationResult> {
        if self.compilation_succeeds {
            Ok(CompilationResult {
                file_path: code.file_path.clone(),
                success: true,
                diagnostics: vec![],
                duration_ms: 50,
            })
        } else {
            Ok(CompilationResult {
                file_path: code.file_path.clone(),
                success: false,
                diagnostics: vec![format!("simulated compilation error in {}", code.file_path,)],
                duration_ms: 30,
            })
        }
    }

    fn run_tests(
        &self,
        _code: &[GeneratedCode],
        tests: &[TestSpec],
    ) -> CodegenResult<Vec<TestResult>> {
        Ok(tests
            .iter()
            .map(|test| TestResult {
                test_name: test.name.clone(),
                passed: self.tests_succeed,
                output: if self.tests_succeed {
                    format!("PASS: {}", test.name)
                } else {
                    format!("FAIL: {} — simulated test failure", test.name)
                },
                duration_ms: 10,
            })
            .collect())
    }

    fn evaluate_performance(
        &self,
        _code: &[GeneratedCode],
        gates: &[PerformanceGate],
    ) -> CodegenResult<Vec<PerformanceResult>> {
        Ok(gates
            .iter()
            .map(|gate| {
                let measured_value = if self.performance_succeeds {
                    // Produce a passing value based on comparison direction
                    match gate.comparison {
                        Comparison::LessThan => gate.threshold * 0.8,
                        Comparison::GreaterThan => gate.threshold * 1.2,
                        Comparison::Within(_) => gate.threshold,
                    }
                } else {
                    // Produce a failing value
                    match gate.comparison {
                        Comparison::LessThan => gate.threshold * 1.5,
                        Comparison::GreaterThan => gate.threshold * 0.5,
                        Comparison::Within(tol) => gate.threshold + tol * 2.0,
                    }
                };

                let passed = match gate.comparison {
                    Comparison::LessThan => measured_value < gate.threshold,
                    Comparison::GreaterThan => measured_value > gate.threshold,
                    Comparison::Within(tol) => (measured_value - gate.threshold).abs() <= tol,
                };

                PerformanceResult {
                    metric: gate.metric.clone(),
                    measured_value,
                    threshold: gate.threshold,
                    passed,
                    description: if passed {
                        format!(
                            "{}: {:.2} {} {:.2} ✓",
                            gate.metric, measured_value, gate.comparison, gate.threshold
                        )
                    } else {
                        format!(
                            "{}: {:.2} {} {:.2} ✗",
                            gate.metric, measured_value, gate.comparison, gate.threshold
                        )
                    },
                }
            })
            .collect())
    }

    fn name(&self) -> &str {
        "simulated-sandbox"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use maple_worldline_intent::proposal::TestType;

    fn make_generated_code() -> GeneratedCode {
        GeneratedCode {
            change_spec_index: 0,
            file_path: "src/test.rs".into(),
            content: "fn test() {}".into(),
            description: "test".into(),
            content_hash: "abc123".into(),
            generated_at: Utc::now(),
        }
    }

    fn make_test_specs() -> Vec<TestSpec> {
        vec![
            TestSpec {
                name: "test_a".into(),
                description: "Test A".into(),
                test_type: TestType::Unit,
            },
            TestSpec {
                name: "test_b".into(),
                description: "Test B".into(),
                test_type: TestType::Integration,
            },
        ]
    }

    fn make_perf_gates() -> Vec<PerformanceGate> {
        vec![PerformanceGate {
            metric: "latency_p99".into(),
            threshold: 10.0,
            comparison: Comparison::LessThan,
        }]
    }

    #[test]
    fn simulated_sandbox_compile_success() {
        let sandbox = SimulatedSandbox::all_pass();
        let code = make_generated_code();
        let result = sandbox.compile(&code).unwrap();
        assert!(result.success);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn simulated_sandbox_compile_failure() {
        let sandbox = SimulatedSandbox::compilation_fails();
        let code = make_generated_code();
        let result = sandbox.compile(&code).unwrap();
        assert!(!result.success);
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn simulated_sandbox_run_tests_all_pass() {
        let sandbox = SimulatedSandbox::all_pass();
        let code = vec![make_generated_code()];
        let tests = make_test_specs();
        let results = sandbox.run_tests(&code, &tests).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn simulated_sandbox_run_tests_failure() {
        let sandbox = SimulatedSandbox::tests_fail();
        let code = vec![make_generated_code()];
        let tests = make_test_specs();
        let results = sandbox.run_tests(&code, &tests).unwrap();
        assert!(results.iter().all(|r| !r.passed));
    }

    #[test]
    fn simulated_sandbox_performance_pass() {
        let sandbox = SimulatedSandbox::all_pass();
        let code = vec![make_generated_code()];
        let gates = make_perf_gates();
        let results = sandbox.evaluate_performance(&code, &gates).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
        assert!(results[0].measured_value < results[0].threshold);
    }

    #[test]
    fn simulated_sandbox_performance_failure() {
        let sandbox = SimulatedSandbox::performance_fails();
        let code = vec![make_generated_code()];
        let gates = make_perf_gates();
        let results = sandbox.evaluate_performance(&code, &gates).unwrap();
        assert!(!results[0].passed);
        assert!(results[0].measured_value > results[0].threshold);
    }

    #[test]
    fn simulated_sandbox_all_pass_convenience() {
        let sandbox = SimulatedSandbox::all_pass();
        assert_eq!(sandbox.name(), "simulated-sandbox");
    }

    #[test]
    fn simulated_sandbox_empty_gates() {
        let sandbox = SimulatedSandbox::all_pass();
        let code = vec![make_generated_code()];
        let results = sandbox.evaluate_performance(&code, &[]).unwrap();
        assert!(results.is_empty());
    }
}
