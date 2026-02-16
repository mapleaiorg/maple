//! WorldLine Conformance Testing Suite
//!
//! Verifies all 22 safety invariants across the 15 WorldLine subsystems
//! and produces a unified conformance report.
//!
//! # Invariant Groups
//!
//! | Group | Count | IDs |
//! |-------|-------|-----|
//! | Observation | 5 | I.OBS-1 … I.OBS-5 |
//! | Self-Mod Gate | 7 | I.REGEN-1 … I.REGEN-7 |
//! | Consequence | 2 | I.CSQ-1, I.CSQ-2 |
//! | Compiler | 2 | I.COMPILE-1, I.COMPILE-2 |
//! | SAL | 2 | I.SAL-1, I.SAL-5 |
//! | Bootstrap | 2 | I.BOOT-1, I.BOOT-2 |
//! | EVOS | 2 | I.EVOS-1, I.EVOS-2 |
//!
//! # Quick Start
//!
//! ```rust
//! use worldline_conformance::runner::ConformanceRunner;
//!
//! let runner = ConformanceRunner::new();
//! let report = runner.run_all().unwrap();
//! assert!(report.all_passed());
//! ```

pub mod benchmarks;
pub mod error;
pub mod invariants;
pub mod report;
pub mod runner;
pub mod types;

// Re-export key types at crate root.
pub use error::{ConformanceError, ConformanceResult};
pub use invariants::{check_invariant, ALL_WORLDLINE_INVARIANT_IDS};
pub use report::ConformanceReport;
pub use runner::ConformanceRunner;
pub use types::{
    ConformanceConfig, ConformanceSummary, InvariantCategory, InvariantId, InvariantResult,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_conformance_pass() {
        let runner = ConformanceRunner::new();
        let report = runner.run_all().unwrap();
        assert!(
            report.all_passed(),
            "Not all invariants passed: {} failed",
            report.summary.failed
        );
        assert_eq!(report.summary.total, 22);
    }

    #[test]
    fn test_report_display() {
        let runner = ConformanceRunner::new();
        let report = runner.run_all().unwrap();
        let output = format!("{}", report);
        assert!(output.contains("WorldLine Conformance Report"));
        assert!(output.contains("SATISFIED"));
    }

    #[test]
    fn test_category_run_observation() {
        let runner = ConformanceRunner::new();
        let report = runner.run_category(InvariantCategory::Observation).unwrap();
        assert_eq!(report.summary.total, 5);
        assert!(report.all_passed());
    }

    #[test]
    fn test_category_run_self_mod_gate() {
        let runner = ConformanceRunner::new();
        let report = runner.run_category(InvariantCategory::SelfModGate).unwrap();
        assert_eq!(report.summary.total, 7);
        assert!(report.all_passed());
    }

    #[test]
    fn test_category_run_consequence() {
        let runner = ConformanceRunner::new();
        let report = runner.run_category(InvariantCategory::Consequence).unwrap();
        assert_eq!(report.summary.total, 2);
        assert!(report.all_passed());
    }

    #[test]
    fn test_category_run_compiler() {
        let runner = ConformanceRunner::new();
        let report = runner.run_category(InvariantCategory::Compiler).unwrap();
        assert_eq!(report.summary.total, 2);
        assert!(report.all_passed());
    }

    #[test]
    fn test_category_run_sal() {
        let runner = ConformanceRunner::new();
        let report = runner.run_category(InvariantCategory::Sal).unwrap();
        assert_eq!(report.summary.total, 2);
        assert!(report.all_passed());
    }

    #[test]
    fn test_category_run_bootstrap() {
        let runner = ConformanceRunner::new();
        let report = runner.run_category(InvariantCategory::Bootstrap).unwrap();
        assert_eq!(report.summary.total, 2);
        assert!(report.all_passed());
    }

    #[test]
    fn test_category_run_evos() {
        let runner = ConformanceRunner::new();
        let report = runner.run_category(InvariantCategory::Evos).unwrap();
        assert_eq!(report.summary.total, 2);
        assert!(report.all_passed());
    }

    #[test]
    fn test_report_json_serialization() {
        let runner = ConformanceRunner::new();
        let report = runner.run_all().unwrap();
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("\"total\": 22"));
        let deserialized: ConformanceReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.summary.total, 22);
    }
}
