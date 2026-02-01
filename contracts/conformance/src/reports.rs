//! Conformance test reporting

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Test status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
}

/// Test category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestCategory {
    Core,
    Behavioral,
    PlatformSpecific,
}

impl std::fmt::Display for TestCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestCategory::Core => write!(f, "Core"),
            TestCategory::Behavioral => write!(f, "Behavioral"),
            TestCategory::PlatformSpecific => write!(f, "Platform-Specific"),
        }
    }
}

/// Individual test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub status: TestStatus,
    pub duration: Duration,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub details: HashMap<String, String>,
}

impl TestResult {
    /// Create a passed test result
    pub fn passed(name: impl Into<String>, duration: Duration) -> Self {
        Self {
            name: name.into(),
            status: TestStatus::Passed,
            duration,
            error: None,
            warnings: Vec::new(),
            details: HashMap::new(),
        }
    }

    /// Create a failed test result
    pub fn failed(name: impl Into<String>, error: String, duration: Duration) -> Self {
        Self {
            name: name.into(),
            status: TestStatus::Failed,
            duration,
            error: Some(error),
            warnings: Vec::new(),
            details: HashMap::new(),
        }
    }

    /// Create a skipped test result
    pub fn skipped(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: TestStatus::Skipped,
            duration: Duration::ZERO,
            error: None,
            warnings: vec![reason.into()],
            details: HashMap::new(),
        }
    }

    /// Add a warning to the result
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Add a detail to the result
    pub fn add_detail(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.details.insert(key.into(), value.into());
    }
}

/// Report summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub conformant: bool,
}

/// Complete conformance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceReport {
    pub platform_name: String,
    pub timestamp: DateTime<Utc>,
    pub duration: Duration,
    pub results: HashMap<TestCategory, Vec<TestResult>>,
    pub summary: ReportSummary,
}

impl ConformanceReport {
    /// Create a new report
    pub fn new(platform_name: String) -> Self {
        Self {
            platform_name,
            timestamp: Utc::now(),
            duration: Duration::ZERO,
            results: HashMap::new(),
            summary: ReportSummary::default(),
        }
    }

    /// Add results for a category
    pub fn add_results(&mut self, category: TestCategory, results: Vec<TestResult>) {
        self.results.insert(category, results);
    }

    /// Finalize the report and compute summary
    pub fn finalize(&mut self) {
        let mut total = 0;
        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;

        for results in self.results.values() {
            for result in results {
                total += 1;
                match result.status {
                    TestStatus::Passed => passed += 1,
                    TestStatus::Failed => failed += 1,
                    TestStatus::Skipped => skipped += 1,
                }
            }
        }

        self.summary = ReportSummary {
            total,
            passed,
            failed,
            skipped,
            conformant: failed == 0,
        };
    }

    /// Get passed count
    pub fn passed_count(&self) -> usize {
        self.summary.passed
    }

    /// Get failed count
    pub fn failed_count(&self) -> usize {
        self.summary.failed
    }

    /// Get skipped count
    pub fn skipped_count(&self) -> usize {
        self.summary.skipped
    }

    /// Check if conformant
    pub fn is_conformant(&self) -> bool {
        self.summary.conformant
    }

    /// Generate a text report
    pub fn to_text(&self) -> String {
        let mut output = String::new();

        output.push_str(
            "╔════════════════════════════════════════════════════════════╗\n",
        );
        output.push_str(
            "║  PALM Platform Pack Conformance Report                     ║\n",
        );
        output.push_str(
            "╠════════════════════════════════════════════════════════════╣\n",
        );
        output.push_str(&format!(
            "║  Platform: {:<47} ║\n",
            self.platform_name
        ));
        output.push_str(&format!(
            "║  Timestamp: {:<46} ║\n",
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        output.push_str(&format!(
            "║  Duration: {:<47} ║\n",
            format!("{:?}", self.duration)
        ));
        output.push_str(
            "╠════════════════════════════════════════════════════════════╣\n",
        );

        // Sort categories for consistent output
        let mut categories: Vec<_> = self.results.keys().collect();
        categories.sort_by_key(|c| match c {
            TestCategory::Core => 0,
            TestCategory::Behavioral => 1,
            TestCategory::PlatformSpecific => 2,
        });

        for category in categories {
            let results = &self.results[category];
            output.push_str(&format!("║  {} Tests:                                            \n", category));
            output.push_str(
                "╟────────────────────────────────────────────────────────────╢\n",
            );

            for result in results {
                let status_icon = match result.status {
                    TestStatus::Passed => "✓",
                    TestStatus::Failed => "✗",
                    TestStatus::Skipped => "○",
                };

                output.push_str(&format!(
                    "║  {} {:<45} {:>8?}\n",
                    status_icon, result.name, result.duration
                ));

                if let Some(error) = &result.error {
                    output.push_str(&format!("║      Error: {}\n", error));
                }

                for warning in &result.warnings {
                    output.push_str(&format!("║      Warning: {}\n", warning));
                }
            }

            output.push_str(
                "╟────────────────────────────────────────────────────────────╢\n",
            );
        }

        output.push_str(
            "╠════════════════════════════════════════════════════════════╣\n",
        );
        output.push_str(
            "║  Summary:                                                  ║\n",
        );
        output.push_str(&format!(
            "║    Total: {:<5}  Passed: {:<5}  Failed: {:<5}  Skipped: {:<3} ║\n",
            self.summary.total, self.summary.passed, self.summary.failed, self.summary.skipped
        ));
        output.push_str(
            "║                                                            ║\n",
        );

        if self.summary.conformant {
            output.push_str(
                "║  Result: ✓ CONFORMANT                                      ║\n",
            );
        } else {
            output.push_str(
                "║  Result: ✗ NON-CONFORMANT                                  ║\n",
            );
        }

        output.push_str(
            "╚════════════════════════════════════════════════════════════╝\n",
        );

        output
    }

    /// Generate JSON report
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_result_passed() {
        let result = TestResult::passed("test_name", Duration::from_millis(100));
        assert_eq!(result.status, TestStatus::Passed);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_test_result_failed() {
        let result = TestResult::failed("test_name", "error message".to_string(), Duration::from_millis(100));
        assert_eq!(result.status, TestStatus::Failed);
        assert_eq!(result.error, Some("error message".to_string()));
    }

    #[test]
    fn test_report_finalize() {
        let mut report = ConformanceReport::new("test".to_string());
        report.add_results(
            TestCategory::Core,
            vec![
                TestResult::passed("test1", Duration::ZERO),
                TestResult::failed("test2", "error".to_string(), Duration::ZERO),
                TestResult::skipped("test3", "reason"),
            ],
        );
        report.finalize();

        assert_eq!(report.summary.total, 3);
        assert_eq!(report.summary.passed, 1);
        assert_eq!(report.summary.failed, 1);
        assert_eq!(report.summary.skipped, 1);
        assert!(!report.is_conformant());
    }

    #[test]
    fn test_report_to_text() {
        let mut report = ConformanceReport::new("test-platform".to_string());
        report.add_results(
            TestCategory::Core,
            vec![TestResult::passed("test1", Duration::from_millis(10))],
        );
        report.finalize();

        let text = report.to_text();
        assert!(text.contains("test-platform"));
        assert!(text.contains("CONFORMANT"));
    }
}
