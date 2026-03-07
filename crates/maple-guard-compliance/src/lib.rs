//! MAPLE Guard Compliance -- industry compliance packs and evaluation.
//!
//! Loads compliance packs for various industry standards (PCI-DSS, HIPAA, SOC2, etc.)
//! and evaluates systems against their controls.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ComplianceError {
    #[error("compliance pack not found: {0}")]
    PackNotFound(String),
    #[error("control not found: {0}")]
    ControlNotFound(String),
    #[error("evaluation error: {0}")]
    EvaluationError(String),
    #[error("invalid pack version: {0}")]
    InvalidVersion(String),
}

pub type ComplianceResult<T> = Result<T, ComplianceError>;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Industry compliance standards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceStandard {
    #[serde(rename = "PCI_DSS")]
    PciDss,
    HIPAA,
    SOC2,
    GDPR,
    ISO27001,
    FedRAMP,
}

impl std::fmt::Display for ComplianceStandard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PciDss => write!(f, "PCI-DSS"),
            Self::HIPAA => write!(f, "HIPAA"),
            Self::SOC2 => write!(f, "SOC2"),
            Self::GDPR => write!(f, "GDPR"),
            Self::ISO27001 => write!(f, "ISO-27001"),
            Self::FedRAMP => write!(f, "FedRAMP"),
        }
    }
}

/// A single compliance control requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceControl {
    pub id: String,
    pub description: String,
    pub requirement: String,
    /// Name of the test function used to evaluate this control.
    pub test_fn_name: String,
}

/// A compliance pack containing a set of controls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompliancePack {
    pub id: String,
    pub name: String,
    pub standard: ComplianceStandard,
    pub version: semver::Version,
    pub controls: Vec<ComplianceControl>,
}

impl CompliancePack {
    pub fn new(name: impl Into<String>, standard: ComplianceStandard, version: semver::Version) -> Self {
        Self {
            id: format!("{}-{}", standard, version),
            name: name.into(),
            standard,
            version,
            controls: Vec::new(),
        }
    }

    pub fn add_control(&mut self, control: ComplianceControl) {
        self.controls.push(control);
    }
}

/// Result of evaluating a single control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlResult {
    Pass,
    Fail,
    NotApplicable,
    Error,
}

/// Evidence collected during a control check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlEvidence {
    pub control_id: String,
    pub result: ControlResult,
    pub details: String,
    pub evidence: Vec<String>,
    pub checked_at: DateTime<Utc>,
}

/// Full compliance report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub pack_id: String,
    pub standard: ComplianceStandard,
    pub results: Vec<ControlEvidence>,
    pub overall_score: f64,
    pub pass_count: usize,
    pub fail_count: usize,
    pub total_controls: usize,
    pub generated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Compliance Engine
// ---------------------------------------------------------------------------

/// A function that evaluates a compliance control.
pub type ControlCheckFn = Box<dyn Fn(&ComplianceControl) -> ControlEvidence + Send + Sync>;

/// Engine that loads compliance packs and evaluates controls.
pub struct ComplianceEngine {
    packs: HashMap<String, CompliancePack>,
    check_fns: HashMap<String, ControlCheckFn>,
}

impl Default for ComplianceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplianceEngine {
    pub fn new() -> Self {
        Self {
            packs: HashMap::new(),
            check_fns: HashMap::new(),
        }
    }

    /// Load a compliance pack.
    pub fn load_pack(&mut self, pack: CompliancePack) {
        self.packs.insert(pack.id.clone(), pack);
    }

    /// Register a check function by name.
    pub fn register_check(&mut self, name: impl Into<String>, f: ControlCheckFn) {
        self.check_fns.insert(name.into(), f);
    }

    /// Get a pack by ID.
    pub fn get_pack(&self, id: &str) -> ComplianceResult<&CompliancePack> {
        self.packs
            .get(id)
            .ok_or_else(|| ComplianceError::PackNotFound(id.to_string()))
    }

    /// List all loaded packs.
    pub fn list_packs(&self) -> Vec<&CompliancePack> {
        self.packs.values().collect()
    }

    /// Evaluate all controls in a pack.
    pub fn evaluate(&self, pack_id: &str) -> ComplianceResult<ComplianceReport> {
        let pack = self
            .packs
            .get(pack_id)
            .ok_or_else(|| ComplianceError::PackNotFound(pack_id.to_string()))?;

        let mut results = Vec::new();
        for control in &pack.controls {
            let evidence = if let Some(check_fn) = self.check_fns.get(&control.test_fn_name) {
                check_fn(control)
            } else {
                // No check function registered -- treat as not applicable
                ControlEvidence {
                    control_id: control.id.clone(),
                    result: ControlResult::NotApplicable,
                    details: format!("No check function registered: {}", control.test_fn_name),
                    evidence: Vec::new(),
                    checked_at: Utc::now(),
                }
            };
            results.push(evidence);
        }

        let pass_count = results.iter().filter(|r| r.result == ControlResult::Pass).count();
        let fail_count = results.iter().filter(|r| r.result == ControlResult::Fail).count();
        let total = pack.controls.len();
        let score = if total > 0 {
            pass_count as f64 / total as f64 * 100.0
        } else {
            100.0
        };

        Ok(ComplianceReport {
            pack_id: pack_id.to_string(),
            standard: pack.standard,
            results,
            overall_score: score,
            pass_count,
            fail_count,
            total_controls: total,
            generated_at: Utc::now(),
        })
    }

    /// List packs by standard.
    pub fn packs_by_standard(&self, standard: ComplianceStandard) -> Vec<&CompliancePack> {
        self.packs.values().filter(|p| p.standard == standard).collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pack() -> CompliancePack {
        let mut pack = CompliancePack::new("SOC2 Type II", ComplianceStandard::SOC2, semver::Version::new(1, 0, 0));
        pack.add_control(ComplianceControl {
            id: "CC1.1".into(),
            description: "Access control policy".into(),
            requirement: "Organization has defined access control policies".into(),
            test_fn_name: "check_access_control".into(),
        });
        pack.add_control(ComplianceControl {
            id: "CC1.2".into(),
            description: "Audit logging".into(),
            requirement: "All access is logged".into(),
            test_fn_name: "check_audit_logging".into(),
        });
        pack
    }

    #[test]
    fn test_load_pack() {
        let mut engine = ComplianceEngine::new();
        engine.load_pack(sample_pack());
        assert_eq!(engine.list_packs().len(), 1);
    }

    #[test]
    fn test_evaluate_with_no_checks() {
        let mut engine = ComplianceEngine::new();
        let pack = sample_pack();
        let pack_id = pack.id.clone();
        engine.load_pack(pack);
        let report = engine.evaluate(&pack_id).unwrap();
        assert_eq!(report.total_controls, 2);
        // All should be NotApplicable since no check fns registered
        assert_eq!(report.pass_count, 0);
    }

    #[test]
    fn test_evaluate_with_checks() {
        let mut engine = ComplianceEngine::new();
        let pack = sample_pack();
        let pack_id = pack.id.clone();
        engine.load_pack(pack);

        engine.register_check("check_access_control", Box::new(|ctrl| ControlEvidence {
            control_id: ctrl.id.clone(),
            result: ControlResult::Pass,
            details: "Access control policy found".into(),
            evidence: vec!["policy.yaml".into()],
            checked_at: Utc::now(),
        }));

        engine.register_check("check_audit_logging", Box::new(|ctrl| ControlEvidence {
            control_id: ctrl.id.clone(),
            result: ControlResult::Fail,
            details: "Audit logging not configured".into(),
            evidence: Vec::new(),
            checked_at: Utc::now(),
        }));

        let report = engine.evaluate(&pack_id).unwrap();
        assert_eq!(report.pass_count, 1);
        assert_eq!(report.fail_count, 1);
        assert!((report.overall_score - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_pack_not_found() {
        let engine = ComplianceEngine::new();
        assert!(engine.evaluate("nonexistent").is_err());
    }

    #[test]
    fn test_standard_display() {
        assert_eq!(ComplianceStandard::PciDss.to_string(), "PCI-DSS");
        assert_eq!(ComplianceStandard::HIPAA.to_string(), "HIPAA");
        assert_eq!(ComplianceStandard::GDPR.to_string(), "GDPR");
    }

    #[test]
    fn test_packs_by_standard() {
        let mut engine = ComplianceEngine::new();
        engine.load_pack(sample_pack());
        let mut hipaa = CompliancePack::new("HIPAA Pack", ComplianceStandard::HIPAA, semver::Version::new(1, 0, 0));
        hipaa.add_control(ComplianceControl {
            id: "H1".into(),
            description: "PHI protection".into(),
            requirement: "PHI must be encrypted".into(),
            test_fn_name: "check_phi".into(),
        });
        engine.load_pack(hipaa);
        let soc2 = engine.packs_by_standard(ComplianceStandard::SOC2);
        assert_eq!(soc2.len(), 1);
        let hipaa_packs = engine.packs_by_standard(ComplianceStandard::HIPAA);
        assert_eq!(hipaa_packs.len(), 1);
    }

    #[test]
    fn test_get_pack() {
        let mut engine = ComplianceEngine::new();
        let pack = sample_pack();
        let pack_id = pack.id.clone();
        engine.load_pack(pack);
        let fetched = engine.get_pack(&pack_id).unwrap();
        assert_eq!(fetched.controls.len(), 2);
    }

    #[test]
    fn test_empty_pack_evaluation() {
        let mut engine = ComplianceEngine::new();
        let pack = CompliancePack::new("Empty", ComplianceStandard::GDPR, semver::Version::new(1, 0, 0));
        let pack_id = pack.id.clone();
        engine.load_pack(pack);
        let report = engine.evaluate(&pack_id).unwrap();
        assert_eq!(report.total_controls, 0);
        assert!((report.overall_score - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_control_result_variants() {
        assert_eq!(ControlResult::Pass, ControlResult::Pass);
        assert_ne!(ControlResult::Pass, ControlResult::Fail);
    }

    #[test]
    fn test_pack_version() {
        let pack = sample_pack();
        assert_eq!(pack.version, semver::Version::new(1, 0, 0));
    }

    #[test]
    fn test_compliance_report_fields() {
        let mut engine = ComplianceEngine::new();
        let pack = sample_pack();
        let pack_id = pack.id.clone();
        engine.load_pack(pack);
        let report = engine.evaluate(&pack_id).unwrap();
        assert_eq!(report.standard, ComplianceStandard::SOC2);
        assert_eq!(report.results.len(), 2);
    }
}
