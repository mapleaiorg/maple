//! Conformance report generation.
//!
//! Produces a structured report of all invariant check results.

use crate::invariants::InvariantResult;
use std::fmt;

/// A complete conformance report for the MWL kernel.
#[derive(Clone, Debug)]
pub struct ConformanceReport {
    /// Results for each invariant checked
    pub results: Vec<InvariantResult>,
    /// Total invariants checked
    pub total: usize,
    /// Number that passed
    pub passed: usize,
    /// Number that failed
    pub failed: usize,
}

impl ConformanceReport {
    /// Create a report from a list of invariant results.
    pub fn from_results(results: Vec<InvariantResult>) -> Self {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        Self {
            results,
            total,
            passed,
            failed,
        }
    }

    /// Whether all invariants passed.
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// Get only failed results.
    pub fn failures(&self) -> Vec<&InvariantResult> {
        self.results.iter().filter(|r| !r.passed).collect()
    }
}

impl fmt::Display for ConformanceReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "╔══════════════════════════════════════════════════╗")?;
        writeln!(f, "║     MWL Constitutional Conformance Report       ║")?;
        writeln!(f, "╠══════════════════════════════════════════════════╣")?;
        writeln!(f, "║ Total: {:3}  Passed: {:3}  Failed: {:3}            ║",
            self.total, self.passed, self.failed)?;
        writeln!(f, "╚══════════════════════════════════════════════════╝")?;
        writeln!(f)?;

        for result in &self.results {
            writeln!(f, "  {}", result)?;
        }

        writeln!(f)?;
        if self.all_passed() {
            writeln!(f, "✓ ALL INVARIANTS SATISFIED")?;
        } else {
            writeln!(f, "✗ {} INVARIANT(S) VIOLATED", self.failed)?;
        }
        Ok(())
    }
}
