//! Core types for WorldLine conformance testing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifies one of the 22 WorldLine safety invariants.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InvariantId(pub String);

impl InvariantId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InvariantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Category grouping for invariants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvariantCategory {
    /// Observation subsystem invariants (I.OBS-1 through I.OBS-5)
    Observation,
    /// Self-modification gate invariants (I.REGEN-1 through I.REGEN-7)
    SelfModGate,
    /// Consequence subsystem invariants (I.CSQ-1, I.CSQ-2)
    Consequence,
    /// Compiler invariants (I.COMPILE-1, I.COMPILE-2)
    Compiler,
    /// Substrate Abstraction Layer invariants (I.SAL-1, I.SAL-5)
    Sal,
    /// Bootstrap protocol invariants (I.BOOT-1, I.BOOT-2)
    Bootstrap,
    /// EVOS orchestrator invariants (I.EVOS-1, I.EVOS-2)
    Evos,
}

impl InvariantCategory {
    /// All categories in canonical order.
    pub fn all() -> &'static [InvariantCategory] {
        &[
            Self::Observation,
            Self::SelfModGate,
            Self::Consequence,
            Self::Compiler,
            Self::Sal,
            Self::Bootstrap,
            Self::Evos,
        ]
    }

    /// Number of invariants in this category.
    pub fn invariant_count(&self) -> usize {
        match self {
            Self::Observation => 5,
            Self::SelfModGate => 7,
            Self::Consequence => 2,
            Self::Compiler => 2,
            Self::Sal => 2,
            Self::Bootstrap => 2,
            Self::Evos => 2,
        }
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Observation => "Observation",
            Self::SelfModGate => "Self-Modification Gate",
            Self::Consequence => "Consequence",
            Self::Compiler => "Compiler",
            Self::Sal => "Substrate Abstraction Layer",
            Self::Bootstrap => "Bootstrap Protocol",
            Self::Evos => "EVOS Orchestrator",
        }
    }
}

impl fmt::Display for InvariantCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Result of checking a single invariant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantResult {
    /// Which invariant was checked.
    pub id: InvariantId,
    /// Category this invariant belongs to.
    pub category: InvariantCategory,
    /// Human-readable name of the invariant.
    pub name: String,
    /// Whether the invariant holds.
    pub passed: bool,
    /// Description of what was checked.
    pub description: String,
    /// Additional details (error message if failed).
    pub details: Option<String>,
    /// When the check was performed.
    pub checked_at: DateTime<Utc>,
}

impl InvariantResult {
    /// Create a passing result.
    pub fn pass(id: &str, category: InvariantCategory, name: &str, description: &str) -> Self {
        Self {
            id: InvariantId::new(id),
            category,
            name: name.into(),
            passed: true,
            description: description.into(),
            details: None,
            checked_at: Utc::now(),
        }
    }

    /// Create a failing result.
    pub fn fail(
        id: &str,
        category: InvariantCategory,
        name: &str,
        description: &str,
        details: &str,
    ) -> Self {
        Self {
            id: InvariantId::new(id),
            category,
            name: name.into(),
            passed: false,
            description: description.into(),
            details: Some(details.into()),
            checked_at: Utc::now(),
        }
    }
}

impl fmt::Display for InvariantResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.passed { "PASS" } else { "FAIL" };
        write!(f, "[{}] {} - {}: {}", status, self.id, self.name, self.description)?;
        if let Some(ref details) = self.details {
            write!(f, " ({})", details)?;
        }
        Ok(())
    }
}

/// Configuration for a conformance run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceConfig {
    /// Filter to specific categories (empty = all).
    pub categories: Vec<InvariantCategory>,
    /// Filter to specific invariant IDs (empty = all).
    pub invariant_ids: Vec<InvariantId>,
    /// Whether to stop on first failure.
    pub fail_fast: bool,
    /// Whether to include benchmark measurements.
    pub include_benchmarks: bool,
}

impl Default for ConformanceConfig {
    fn default() -> Self {
        Self {
            categories: Vec::new(),
            invariant_ids: Vec::new(),
            fail_fast: false,
            include_benchmarks: false,
        }
    }
}

/// Summary statistics from a conformance run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceSummary {
    /// Total invariants checked.
    pub total: usize,
    /// Number that passed.
    pub passed: usize,
    /// Number that failed.
    pub failed: usize,
    /// Number skipped (due to fail_fast or filter).
    pub skipped: usize,
    /// When the run started.
    pub started_at: DateTime<Utc>,
    /// When the run completed.
    pub completed_at: DateTime<Utc>,
}

impl ConformanceSummary {
    /// Whether all checked invariants passed.
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// Pass rate as a percentage.
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 100.0;
        }
        (self.passed as f64 / self.total as f64) * 100.0
    }
}

impl fmt::Display for ConformanceSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{} passed ({:.1}%), {} failed, {} skipped",
            self.passed,
            self.total,
            self.pass_rate(),
            self.failed,
            self.skipped,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invariant_id_creation() {
        let id = InvariantId::new("I.OBS-1");
        assert_eq!(id.as_str(), "I.OBS-1");
        assert_eq!(id.to_string(), "I.OBS-1");
    }

    #[test]
    fn test_invariant_id_equality() {
        let a = InvariantId::new("I.OBS-1");
        let b = InvariantId::new("I.OBS-1");
        let c = InvariantId::new("I.OBS-2");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_all_categories() {
        let cats = InvariantCategory::all();
        assert_eq!(cats.len(), 7);
        let total: usize = cats.iter().map(|c| c.invariant_count()).sum();
        assert_eq!(total, 22);
    }

    #[test]
    fn test_category_labels() {
        assert_eq!(InvariantCategory::Observation.label(), "Observation");
        assert_eq!(InvariantCategory::SelfModGate.label(), "Self-Modification Gate");
        assert_eq!(InvariantCategory::Evos.label(), "EVOS Orchestrator");
    }

    #[test]
    fn test_invariant_result_pass() {
        let r = InvariantResult::pass("I.OBS-1", InvariantCategory::Observation, "Overhead", "< 1%");
        assert!(r.passed);
        assert!(r.details.is_none());
        assert_eq!(r.id.as_str(), "I.OBS-1");
    }

    #[test]
    fn test_invariant_result_fail() {
        let r = InvariantResult::fail(
            "I.OBS-1",
            InvariantCategory::Observation,
            "Overhead",
            "< 1%",
            "measured 2.5%",
        );
        assert!(!r.passed);
        assert_eq!(r.details.as_deref(), Some("measured 2.5%"));
    }

    #[test]
    fn test_invariant_result_display() {
        let r = InvariantResult::pass("I.OBS-1", InvariantCategory::Observation, "Overhead", "< 1%");
        let s = r.to_string();
        assert!(s.contains("[PASS]"));
        assert!(s.contains("I.OBS-1"));
    }

    #[test]
    fn test_config_default() {
        let cfg = ConformanceConfig::default();
        assert!(cfg.categories.is_empty());
        assert!(cfg.invariant_ids.is_empty());
        assert!(!cfg.fail_fast);
        assert!(!cfg.include_benchmarks);
    }

    #[test]
    fn test_summary_all_passed() {
        let s = ConformanceSummary {
            total: 22,
            passed: 22,
            failed: 0,
            skipped: 0,
            started_at: Utc::now(),
            completed_at: Utc::now(),
        };
        assert!(s.all_passed());
        assert!((s.pass_rate() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_summary_with_failures() {
        let s = ConformanceSummary {
            total: 22,
            passed: 20,
            failed: 2,
            skipped: 0,
            started_at: Utc::now(),
            completed_at: Utc::now(),
        };
        assert!(!s.all_passed());
        assert!((s.pass_rate() - 90.909).abs() < 0.01);
    }
}
