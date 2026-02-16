//! Conformance report generation.
//!
//! Produces structured reports of invariant check results with
//! box-drawing display and category breakdowns.

use crate::types::{ConformanceSummary, InvariantCategory, InvariantResult};
use chrono::{DateTime, Utc};
use std::fmt;

/// Per-category report.
#[derive(Debug, Clone)]
pub struct CategoryReport {
    /// Which category.
    pub category: InvariantCategory,
    /// Results for invariants in this category.
    pub results: Vec<InvariantResult>,
    /// Number that passed.
    pub passed: usize,
    /// Number that failed.
    pub failed: usize,
}

impl CategoryReport {
    /// Build from a filtered set of results.
    pub fn from_results(category: InvariantCategory, results: Vec<InvariantResult>) -> Self {
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = results.len() - passed;
        Self {
            category,
            results,
            passed,
            failed,
        }
    }

    /// Whether all invariants in this category passed.
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }
}

impl fmt::Display for CategoryReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let icon = if self.all_passed() { "+" } else { "!" };
        writeln!(
            f,
            "  [{}] {} ({}/{})",
            icon,
            self.category.label(),
            self.passed,
            self.results.len(),
        )?;
        for r in &self.results {
            let mark = if r.passed { "+" } else { "x" };
            writeln!(f, "      [{}] {}", mark, r)?;
        }
        Ok(())
    }
}

/// A complete conformance report for WorldLine safety invariants.
#[derive(Debug, Clone)]
pub struct ConformanceReport {
    /// All individual invariant results.
    pub results: Vec<InvariantResult>,
    /// Per-category breakdowns.
    pub categories: Vec<CategoryReport>,
    /// Summary statistics.
    pub summary: ConformanceSummary,
}

impl ConformanceReport {
    /// Create a report from a list of invariant results.
    pub fn from_results(
        results: Vec<InvariantResult>,
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
    ) -> Self {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total - passed;

        // Build per-category reports.
        let mut categories = Vec::new();
        for cat in InvariantCategory::all() {
            let cat_results: Vec<InvariantResult> = results
                .iter()
                .filter(|r| r.category == *cat)
                .cloned()
                .collect();
            if !cat_results.is_empty() {
                categories.push(CategoryReport::from_results(*cat, cat_results));
            }
        }

        let summary = ConformanceSummary {
            total,
            passed,
            failed,
            skipped: 0,
            started_at,
            completed_at,
        };

        Self {
            results,
            categories,
            summary,
        }
    }

    /// Whether all invariants passed.
    pub fn all_passed(&self) -> bool {
        self.summary.all_passed()
    }

    /// Get only failed results.
    pub fn failures(&self) -> Vec<&InvariantResult> {
        self.results.iter().filter(|r| !r.passed).collect()
    }
}

impl fmt::Display for ConformanceReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "+-------------------------------------------------+")?;
        writeln!(f, "|   WorldLine Conformance Report                  |")?;
        writeln!(f, "+-------------------------------------------------+")?;
        writeln!(
            f,
            "| Total: {:3}  Passed: {:3}  Failed: {:3}             |",
            self.summary.total, self.summary.passed, self.summary.failed,
        )?;
        writeln!(f, "+-------------------------------------------------+")?;
        writeln!(f)?;

        for cat in &self.categories {
            write!(f, "{}", cat)?;
        }

        writeln!(f)?;
        if self.all_passed() {
            writeln!(f, "  ALL 22 WORLDLINE SAFETY INVARIANTS SATISFIED")?;
        } else {
            writeln!(
                f,
                "  {} INVARIANT(S) VIOLATED",
                self.summary.failed,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::InvariantResult;
    use chrono::Utc;

    fn make_results(pass_count: usize, fail_count: usize) -> Vec<InvariantResult> {
        let mut results = Vec::new();
        for i in 0..pass_count {
            results.push(InvariantResult::pass(
                &format!("I.OBS-{}", i + 1),
                InvariantCategory::Observation,
                "test",
                "test",
            ));
        }
        for i in 0..fail_count {
            results.push(InvariantResult::fail(
                &format!("I.REGEN-{}", i + 1),
                InvariantCategory::SelfModGate,
                "test",
                "test",
                "failed",
            ));
        }
        results
    }

    #[test]
    fn test_report_all_passed() {
        let now = Utc::now();
        let report = ConformanceReport::from_results(make_results(5, 0), now, now);
        assert!(report.all_passed());
        assert_eq!(report.summary.total, 5);
        assert_eq!(report.summary.passed, 5);
        assert_eq!(report.failures().len(), 0);
    }

    #[test]
    fn test_report_with_failures() {
        let now = Utc::now();
        let report = ConformanceReport::from_results(make_results(3, 2), now, now);
        assert!(!report.all_passed());
        assert_eq!(report.summary.failed, 2);
        assert_eq!(report.failures().len(), 2);
    }

    #[test]
    fn test_category_report() {
        let results = make_results(3, 0);
        let cat_report = CategoryReport::from_results(InvariantCategory::Observation, results);
        assert!(cat_report.all_passed());
        assert_eq!(cat_report.passed, 3);
    }

    #[test]
    fn test_report_display_all_passed() {
        let now = Utc::now();
        let report = ConformanceReport::from_results(make_results(5, 0), now, now);
        let output = format!("{}", report);
        assert!(output.contains("SATISFIED"));
    }

    #[test]
    fn test_report_display_with_failures() {
        let now = Utc::now();
        let report = ConformanceReport::from_results(make_results(3, 2), now, now);
        let output = format!("{}", report);
        assert!(output.contains("VIOLATED"));
    }

    #[test]
    fn test_report_categories_populated() {
        let now = Utc::now();
        let report = ConformanceReport::from_results(make_results(3, 2), now, now);
        assert_eq!(report.categories.len(), 2); // Observation + SelfModGate
    }

    #[test]
    fn test_category_report_display() {
        let results = make_results(2, 0);
        let cat_report = CategoryReport::from_results(InvariantCategory::Observation, results);
        let output = format!("{}", cat_report);
        assert!(output.contains("Observation"));
        assert!(output.contains("[+]"));
    }

    #[test]
    fn test_empty_report() {
        let now = Utc::now();
        let report = ConformanceReport::from_results(vec![], now, now);
        assert!(report.all_passed());
        assert_eq!(report.summary.total, 0);
    }
}
