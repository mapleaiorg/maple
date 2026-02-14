//! Validators — test, performance, and safety validation.
//!
//! Interprets raw results from the sandbox and produces pass/fail
//! judgments based on the proposal requirements and engine configuration.

use maple_worldline_intent::proposal::{PerformanceGate, SafetyCheck, TestSpec};

use crate::error::{CodegenError, CodegenResult};
use crate::types::{CodegenConfig, GeneratedCode, PerformanceResult, TestResult};

// ── Safety-Critical Patterns ───────────────────────────────────────────

/// File path patterns that indicate safety-critical code.
const SAFETY_PATTERNS: &[&str] = &[
    "safety", "rollback", "emergency", "invariant", "gate", "adjudication",
];

// ── Test Validation ────────────────────────────────────────────────────

/// Summary of test validation.
#[derive(Clone, Debug)]
pub struct TestValidationSummary {
    /// Total tests evaluated.
    pub total: usize,
    /// Tests that passed.
    pub passed: usize,
    /// Tests that failed.
    pub failed: usize,
    /// Whether all tests passed.
    pub all_passed: bool,
}

/// Validates test results against proposal requirements.
pub struct TestValidator;

impl TestValidator {
    /// Validate that required tests passed.
    ///
    /// If `require_all_pass` is true, any test failure returns an error.
    /// Otherwise, returns a summary even with failures.
    pub fn validate(
        results: &[TestResult],
        _required_tests: &[TestSpec],
        require_all_pass: bool,
    ) -> CodegenResult<TestValidationSummary> {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        let all_passed = failed == 0;

        if require_all_pass && !all_passed {
            return Err(CodegenError::TestValidationFailed(format!(
                "{}/{} tests failed",
                failed, total,
            )));
        }

        Ok(TestValidationSummary {
            total,
            passed,
            failed,
            all_passed,
        })
    }
}

// ── Performance Validation ─────────────────────────────────────────────

/// Summary of performance validation.
#[derive(Clone, Debug)]
pub struct PerformanceValidationSummary {
    /// Total gates evaluated.
    pub total: usize,
    /// Gates that passed.
    pub passed: usize,
    /// Gates that failed.
    pub failed: usize,
    /// Whether all gates passed.
    pub all_passed: bool,
}

/// Validates performance results against proposal gates.
pub struct PerformanceValidator;

impl PerformanceValidator {
    /// Validate that performance gates are satisfied.
    ///
    /// If `require_all_pass` is true, any gate failure returns an error.
    pub fn validate(
        results: &[PerformanceResult],
        _gates: &[PerformanceGate],
        require_all_pass: bool,
    ) -> CodegenResult<PerformanceValidationSummary> {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        let all_passed = failed == 0;

        if require_all_pass && !all_passed {
            let failing: Vec<String> = results
                .iter()
                .filter(|r| !r.passed)
                .map(|r| format!("{}: {:.2} (threshold {:.2})", r.metric, r.measured_value, r.threshold))
                .collect();
            return Err(CodegenError::PerformanceGateFailed(format!(
                "{}/{} gates failed: {}",
                failed,
                total,
                failing.join(", "),
            )));
        }

        Ok(PerformanceValidationSummary {
            total,
            passed,
            failed,
            all_passed,
        })
    }
}

// ── Safety Validation ──────────────────────────────────────────────────

/// Validates that generated code does not violate safety constraints.
pub struct SafetyValidator;

impl SafetyValidator {
    /// Validate safety checks against generated code.
    ///
    /// Checks:
    /// - No generated code targets safety-critical file paths
    /// - Generated code size is within bounds
    /// - Content hashes are present (integrity)
    pub fn validate(
        generated: &[GeneratedCode],
        _safety_checks: &[SafetyCheck],
        config: &CodegenConfig,
    ) -> CodegenResult<()> {
        for code in generated {
            // Check size bounds
            if code.size_bytes() > config.max_code_size_bytes {
                return Err(CodegenError::SafetyViolation(format!(
                    "Generated code for '{}' exceeds max size ({} > {} bytes)",
                    code.file_path,
                    code.size_bytes(),
                    config.max_code_size_bytes,
                )));
            }

            // Check content hash present
            if code.content_hash.is_empty() {
                return Err(CodegenError::SafetyViolation(format!(
                    "Generated code for '{}' has no content hash",
                    code.file_path,
                )));
            }

            // Check for safety-critical file paths
            let lower = code.file_path.to_lowercase();
            for pattern in SAFETY_PATTERNS {
                if lower.contains(pattern) {
                    return Err(CodegenError::SafetyViolation(format!(
                        "Generated code targets safety-critical path '{}' (matches '{}')",
                        code.file_path, pattern,
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_test_results(pass: bool, count: usize) -> Vec<TestResult> {
        (0..count)
            .map(|i| TestResult {
                test_name: format!("test_{}", i),
                passed: pass,
                output: "ok".into(),
                duration_ms: 10,
            })
            .collect()
    }

    fn make_perf_results(pass: bool) -> Vec<PerformanceResult> {
        vec![PerformanceResult {
            metric: "latency".into(),
            measured_value: if pass { 5.0 } else { 15.0 },
            threshold: 10.0,
            passed: pass,
            description: "test".into(),
        }]
    }

    fn make_generated(file_path: &str, size: usize) -> GeneratedCode {
        GeneratedCode {
            change_spec_index: 0,
            file_path: file_path.into(),
            content: "x".repeat(size),
            description: "test".into(),
            content_hash: "abc123".into(),
            generated_at: Utc::now(),
        }
    }

    #[test]
    fn test_validator_all_pass() {
        let results = make_test_results(true, 5);
        let summary = TestValidator::validate(&results, &[], true).unwrap();
        assert_eq!(summary.total, 5);
        assert_eq!(summary.passed, 5);
        assert!(summary.all_passed);
    }

    #[test]
    fn test_validator_partial_failure_strict() {
        let mut results = make_test_results(true, 3);
        results[1].passed = false;
        let result = TestValidator::validate(&results, &[], true);
        assert!(result.is_err());
    }

    #[test]
    fn test_validator_partial_failure_lenient() {
        let mut results = make_test_results(true, 3);
        results[1].passed = false;
        let summary = TestValidator::validate(&results, &[], false).unwrap();
        assert_eq!(summary.failed, 1);
        assert!(!summary.all_passed);
    }

    #[test]
    fn test_validator_empty_tests() {
        let summary = TestValidator::validate(&[], &[], true).unwrap();
        assert_eq!(summary.total, 0);
        assert!(summary.all_passed);
    }

    #[test]
    fn performance_validator_all_pass() {
        let results = make_perf_results(true);
        let summary = PerformanceValidator::validate(&results, &[], true).unwrap();
        assert_eq!(summary.passed, 1);
        assert!(summary.all_passed);
    }

    #[test]
    fn performance_validator_gate_failure_strict() {
        let results = make_perf_results(false);
        let result = PerformanceValidator::validate(&results, &[], true);
        assert!(result.is_err());
    }

    #[test]
    fn performance_validator_empty_gates() {
        let summary = PerformanceValidator::validate(&[], &[], true).unwrap();
        assert_eq!(summary.total, 0);
        assert!(summary.all_passed);
    }

    #[test]
    fn safety_validator_rejects_oversized_code() {
        let config = CodegenConfig {
            max_code_size_bytes: 10, // Very small limit
            ..CodegenConfig::default()
        };
        let generated = vec![make_generated("src/config.rs", 100)]; // 100 bytes > 10 limit
        let result = SafetyValidator::validate(&generated, &[], &config);
        assert!(result.is_err());
    }

    #[test]
    fn safety_validator_rejects_safety_critical_paths() {
        let config = CodegenConfig::default();
        let generated = vec![make_generated("src/safety/handler.rs", 10)];
        let result = SafetyValidator::validate(&generated, &[], &config);
        assert!(result.is_err());
    }

    #[test]
    fn safety_validator_passes_normal_code() {
        let config = CodegenConfig::default();
        let generated = vec![make_generated("src/config.rs", 100)];
        let result = SafetyValidator::validate(&generated, &[], &config);
        assert!(result.is_ok());
    }
}
