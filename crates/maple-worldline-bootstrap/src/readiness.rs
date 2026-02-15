//! Phase readiness checking.
//!
//! Determines whether the system is ready to advance to the next
//! bootstrap phase, based on stability, success rate, rollback rate,
//! governance approval, and observation time.

use serde::{Deserialize, Serialize};

use crate::error::BootstrapResult;
use crate::types::{BootstrapPhase, ReadinessScore};

// ── Readiness Criteria ──────────────────────────────────────────────

/// Criteria for advancing to the next bootstrap phase.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReadinessCriteria {
    /// Minimum stability score (0.0-1.0).
    pub stability_threshold: f64,
    /// Minimum success rate for operations in current phase (0.0-1.0).
    pub success_rate_threshold: f64,
    /// Maximum allowable rollback rate (0.0-1.0).
    pub max_rollback_rate: f64,
    /// Whether governance approval is required.
    pub governance_approval_required: bool,
    /// Minimum observation hours before advancement.
    pub min_observation_hours: u64,
}

impl Default for ReadinessCriteria {
    fn default() -> Self {
        Self {
            stability_threshold: 0.8,
            success_rate_threshold: 0.9,
            max_rollback_rate: 0.1,
            governance_approval_required: true,
            min_observation_hours: 24,
        }
    }
}

impl ReadinessCriteria {
    /// Stricter criteria for higher phases.
    pub fn for_phase(phase: &BootstrapPhase) -> Self {
        match phase.ordinal() {
            0 | 1 => Self {
                stability_threshold: 0.7,
                success_rate_threshold: 0.85,
                max_rollback_rate: 0.15,
                governance_approval_required: false,
                min_observation_hours: 12,
            },
            2 | 3 => Self::default(),
            4 => Self {
                stability_threshold: 0.9,
                success_rate_threshold: 0.95,
                max_rollback_rate: 0.05,
                governance_approval_required: true,
                min_observation_hours: 48,
            },
            _ => Self {
                stability_threshold: 0.95,
                success_rate_threshold: 0.98,
                max_rollback_rate: 0.02,
                governance_approval_required: true,
                min_observation_hours: 72,
            },
        }
    }
}

// ── Criterion Result ────────────────────────────────────────────────

/// Result of evaluating a single readiness criterion.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CriterionResult {
    pub name: String,
    pub passed: bool,
    pub actual_value: String,
    pub required_value: String,
}

impl std::fmt::Display for CriterionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.passed { "PASS" } else { "FAIL" };
        write!(
            f,
            "[{}] {}: {} (required: {})",
            status, self.name, self.actual_value, self.required_value,
        )
    }
}

// ── Readiness Report ────────────────────────────────────────────────

/// Report of a readiness evaluation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReadinessReport {
    /// Phase being evaluated for readiness.
    pub target_phase: BootstrapPhase,
    /// Overall readiness score.
    pub overall_score: ReadinessScore,
    /// Individual criterion results.
    pub criteria_results: Vec<CriterionResult>,
    /// Whether all criteria passed.
    pub all_passed: bool,
    /// When the report was generated.
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl ReadinessReport {
    /// How many criteria passed.
    pub fn passed_count(&self) -> usize {
        self.criteria_results.iter().filter(|c| c.passed).count()
    }

    /// How many criteria failed.
    pub fn failed_count(&self) -> usize {
        self.criteria_results.iter().filter(|c| !c.passed).count()
    }
}

impl std::fmt::Display for ReadinessReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ReadinessReport(target={}, score={}, passed={}/{})",
            self.target_phase,
            self.overall_score,
            self.passed_count(),
            self.criteria_results.len(),
        )
    }
}

// ── Readiness Checker Trait ─────────────────────────────────────────

/// Trait for checking phase readiness.
pub trait ReadinessChecker: Send + Sync {
    /// Evaluate readiness for the target phase.
    fn check(
        &self,
        current_phase: &BootstrapPhase,
        target_phase: &BootstrapPhase,
        criteria: &ReadinessCriteria,
    ) -> BootstrapResult<ReadinessReport>;

    /// Name of this checker.
    fn name(&self) -> &str;
}

/// Simulated readiness checker for deterministic testing.
pub struct SimulatedReadinessChecker {
    /// Simulated stability score.
    pub stability: f64,
    /// Simulated success rate.
    pub success_rate: f64,
    /// Simulated rollback rate.
    pub rollback_rate: f64,
    /// Simulated observation hours.
    pub observation_hours: u64,
    /// Simulated governance approval.
    pub governance_approved: bool,
}

impl SimulatedReadinessChecker {
    /// Create a checker that passes all criteria.
    pub fn passing() -> Self {
        Self {
            stability: 0.95,
            success_rate: 0.99,
            rollback_rate: 0.01,
            observation_hours: 100,
            governance_approved: true,
        }
    }

    /// Create a checker that fails stability criteria.
    pub fn failing_stability() -> Self {
        Self {
            stability: 0.3,
            success_rate: 0.99,
            rollback_rate: 0.01,
            observation_hours: 100,
            governance_approved: true,
        }
    }

    /// Create a checker that fails governance.
    pub fn failing_governance() -> Self {
        Self {
            stability: 0.95,
            success_rate: 0.99,
            rollback_rate: 0.01,
            observation_hours: 100,
            governance_approved: false,
        }
    }
}

impl Default for SimulatedReadinessChecker {
    fn default() -> Self {
        Self::passing()
    }
}

impl ReadinessChecker for SimulatedReadinessChecker {
    fn check(
        &self,
        _current_phase: &BootstrapPhase,
        target_phase: &BootstrapPhase,
        criteria: &ReadinessCriteria,
    ) -> BootstrapResult<ReadinessReport> {
        let mut results = Vec::new();

        // Stability check
        let stability_passed = self.stability >= criteria.stability_threshold;
        results.push(CriterionResult {
            name: "stability".into(),
            passed: stability_passed,
            actual_value: format!("{:.2}", self.stability),
            required_value: format!(">= {:.2}", criteria.stability_threshold),
        });

        // Success rate check
        let success_passed = self.success_rate >= criteria.success_rate_threshold;
        results.push(CriterionResult {
            name: "success_rate".into(),
            passed: success_passed,
            actual_value: format!("{:.2}", self.success_rate),
            required_value: format!(">= {:.2}", criteria.success_rate_threshold),
        });

        // Rollback rate check
        let rollback_passed = self.rollback_rate <= criteria.max_rollback_rate;
        results.push(CriterionResult {
            name: "rollback_rate".into(),
            passed: rollback_passed,
            actual_value: format!("{:.2}", self.rollback_rate),
            required_value: format!("<= {:.2}", criteria.max_rollback_rate),
        });

        // Observation hours check
        let observation_passed = self.observation_hours >= criteria.min_observation_hours;
        results.push(CriterionResult {
            name: "observation_hours".into(),
            passed: observation_passed,
            actual_value: format!("{}h", self.observation_hours),
            required_value: format!(">= {}h", criteria.min_observation_hours),
        });

        // Governance check
        let governance_passed =
            !criteria.governance_approval_required || self.governance_approved;
        results.push(CriterionResult {
            name: "governance".into(),
            passed: governance_passed,
            actual_value: format!("{}", self.governance_approved),
            required_value: if criteria.governance_approval_required {
                "approved".into()
            } else {
                "not required".into()
            },
        });

        let all_passed = results.iter().all(|r| r.passed);
        let passed_fraction = results.iter().filter(|r| r.passed).count() as f64
            / results.len() as f64;

        Ok(ReadinessReport {
            target_phase: target_phase.clone(),
            overall_score: ReadinessScore::new(passed_fraction),
            criteria_results: results,
            all_passed,
            generated_at: chrono::Utc::now(),
        })
    }

    fn name(&self) -> &str {
        "simulated-readiness-checker"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_criteria() {
        let c = ReadinessCriteria::default();
        assert_eq!(c.stability_threshold, 0.8);
        assert_eq!(c.success_rate_threshold, 0.9);
        assert!(c.governance_approval_required);
    }

    #[test]
    fn criteria_for_early_phases() {
        let c = ReadinessCriteria::for_phase(&BootstrapPhase::Phase0ExternalSubstrate);
        assert_eq!(c.stability_threshold, 0.7);
        assert!(!c.governance_approval_required);
    }

    #[test]
    fn criteria_for_late_phases() {
        let c = ReadinessCriteria::for_phase(&BootstrapPhase::Phase5SubstrateSelfDescription);
        assert_eq!(c.stability_threshold, 0.95);
        assert_eq!(c.min_observation_hours, 72);
        assert!(c.governance_approval_required);
    }

    #[test]
    fn passing_checker_all_pass() {
        let checker = SimulatedReadinessChecker::passing();
        let report = checker
            .check(
                &BootstrapPhase::Phase0ExternalSubstrate,
                &BootstrapPhase::Phase1ConfigSelfTuning,
                &ReadinessCriteria::default(),
            )
            .unwrap();
        assert!(report.all_passed);
        assert_eq!(report.failed_count(), 0);
        assert_eq!(report.passed_count(), 5);
    }

    #[test]
    fn failing_stability_checker() {
        let checker = SimulatedReadinessChecker::failing_stability();
        let report = checker
            .check(
                &BootstrapPhase::Phase0ExternalSubstrate,
                &BootstrapPhase::Phase1ConfigSelfTuning,
                &ReadinessCriteria::default(),
            )
            .unwrap();
        assert!(!report.all_passed);
        assert_eq!(report.failed_count(), 1);
        let failed = report
            .criteria_results
            .iter()
            .find(|c| !c.passed)
            .unwrap();
        assert_eq!(failed.name, "stability");
    }

    #[test]
    fn failing_governance_checker() {
        let checker = SimulatedReadinessChecker::failing_governance();
        let report = checker
            .check(
                &BootstrapPhase::Phase2OperatorSelfGeneration,
                &BootstrapPhase::Phase3ModuleSelfRegeneration,
                &ReadinessCriteria::default(),
            )
            .unwrap();
        assert!(!report.all_passed);
        let failed = report
            .criteria_results
            .iter()
            .find(|c| c.name == "governance")
            .unwrap();
        assert!(!failed.passed);
    }

    #[test]
    fn governance_not_required_passes() {
        let checker = SimulatedReadinessChecker::failing_governance();
        let criteria = ReadinessCriteria {
            governance_approval_required: false,
            ..ReadinessCriteria::default()
        };
        let report = checker
            .check(
                &BootstrapPhase::Phase0ExternalSubstrate,
                &BootstrapPhase::Phase1ConfigSelfTuning,
                &criteria,
            )
            .unwrap();
        let gov_result = report
            .criteria_results
            .iter()
            .find(|c| c.name == "governance")
            .unwrap();
        assert!(gov_result.passed); // Not required, so passes
    }

    #[test]
    fn readiness_report_display() {
        let checker = SimulatedReadinessChecker::passing();
        let report = checker
            .check(
                &BootstrapPhase::Phase0ExternalSubstrate,
                &BootstrapPhase::Phase1ConfigSelfTuning,
                &ReadinessCriteria::default(),
            )
            .unwrap();
        let display = report.to_string();
        assert!(display.contains("Phase1:ConfigSelfTuning"));
        assert!(display.contains("passed=5/5"));
    }

    #[test]
    fn criterion_result_display() {
        let cr = CriterionResult {
            name: "stability".into(),
            passed: true,
            actual_value: "0.95".into(),
            required_value: ">= 0.80".into(),
        };
        let display = cr.to_string();
        assert!(display.contains("[PASS]"));
        assert!(display.contains("stability"));
    }

    #[test]
    fn checker_name() {
        let checker = SimulatedReadinessChecker::passing();
        assert_eq!(checker.name(), "simulated-readiness-checker");
    }
}
