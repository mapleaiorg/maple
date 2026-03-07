//! MAPLE Reference Compliance Agent — regulatory monitoring, policy enforcement, and audit.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Regulation types ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Regulation {
    Gdpr,
    Ccpa,
    Sox,
    Hipaa,
    Pci,
    Aml,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceStatus {
    Compliant,
    NonCompliant,
    NeedsReview,
    Exempted,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

// ── Compliance check & finding ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheck {
    pub id: String,
    pub regulation: Regulation,
    pub name: String,
    pub description: String,
    pub severity: Severity,
    pub auto_remediate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFinding {
    pub id: String,
    pub check_id: String,
    pub regulation: Regulation,
    pub status: ComplianceStatus,
    pub severity: Severity,
    pub details: String,
    pub remediation: Option<String>,
    pub found_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

// ── Policy ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompliancePolicy {
    pub name: String,
    pub regulations: Vec<Regulation>,
    pub checks: Vec<ComplianceCheck>,
    pub enforcement_mode: EnforcementMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementMode {
    Enforce,
    AuditOnly,
    DryRun,
}

// ── Errors ────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ComplianceError {
    #[error("check not found: {0}")]
    CheckNotFound(String),
    #[error("policy not found: {0}")]
    PolicyNotFound(String),
    #[error("finding not found: {0}")]
    FindingNotFound(String),
    #[error("regulation not supported: {0:?}")]
    UnsupportedRegulation(Regulation),
}

// ── Compliance agent ──────────────────────────────────────────

pub struct ComplianceAgent {
    policies: Vec<CompliancePolicy>,
    findings: Vec<ComplianceFinding>,
}

impl ComplianceAgent {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            findings: Vec::new(),
        }
    }

    pub fn add_policy(&mut self, policy: CompliancePolicy) {
        self.policies.push(policy);
    }

    pub fn run_checks(&mut self, context: &AuditContext) -> Vec<ComplianceFinding> {
        let mut new_findings = Vec::new();
        for policy in &self.policies {
            for check in &policy.checks {
                let status = evaluate_check(check, context);
                let finding = ComplianceFinding {
                    id: uuid::Uuid::new_v4().to_string(),
                    check_id: check.id.clone(),
                    regulation: check.regulation.clone(),
                    status,
                    severity: check.severity.clone(),
                    details: format!("Check '{}' evaluated against context", check.name),
                    remediation: if check.auto_remediate {
                        Some(format!("Auto-remediate: {}", check.description))
                    } else {
                        None
                    },
                    found_at: Utc::now(),
                    resolved_at: None,
                };
                new_findings.push(finding);
            }
        }
        self.findings.extend(new_findings.clone());
        new_findings
    }

    pub fn resolve_finding(&mut self, finding_id: &str) -> Result<(), ComplianceError> {
        let finding = self
            .findings
            .iter_mut()
            .find(|f| f.id == finding_id)
            .ok_or_else(|| ComplianceError::FindingNotFound(finding_id.to_string()))?;
        finding.status = ComplianceStatus::Compliant;
        finding.resolved_at = Some(Utc::now());
        Ok(())
    }

    pub fn get_findings(&self) -> &[ComplianceFinding] {
        &self.findings
    }

    pub fn non_compliant_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| matches!(f.status, ComplianceStatus::NonCompliant))
            .count()
    }

    pub fn findings_by_regulation(&self, regulation: &Regulation) -> Vec<&ComplianceFinding> {
        self.findings.iter().filter(|f| &f.regulation == regulation).collect()
    }

    pub fn critical_findings(&self) -> Vec<&ComplianceFinding> {
        self.findings
            .iter()
            .filter(|f| {
                matches!(f.severity, Severity::Critical | Severity::High)
                    && matches!(f.status, ComplianceStatus::NonCompliant)
            })
            .collect()
    }

    pub fn get_policies(&self) -> &[CompliancePolicy] {
        &self.policies
    }
}

impl Default for ComplianceAgent {
    fn default() -> Self {
        Self::new()
    }
}

// ── Audit context ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditContext {
    pub data_types: Vec<String>,
    pub jurisdiction: String,
    pub has_pii: bool,
    pub has_financial_data: bool,
    pub user_consent_given: bool,
}

impl AuditContext {
    pub fn new(jurisdiction: &str) -> Self {
        Self {
            data_types: Vec::new(),
            jurisdiction: jurisdiction.to_string(),
            has_pii: false,
            has_financial_data: false,
            user_consent_given: false,
        }
    }

    pub fn with_pii(mut self) -> Self {
        self.has_pii = true;
        self
    }

    pub fn with_financial_data(mut self) -> Self {
        self.has_financial_data = true;
        self
    }

    pub fn with_consent(mut self) -> Self {
        self.user_consent_given = true;
        self
    }
}

fn evaluate_check(check: &ComplianceCheck, context: &AuditContext) -> ComplianceStatus {
    match &check.regulation {
        Regulation::Gdpr => {
            if context.has_pii && !context.user_consent_given {
                ComplianceStatus::NonCompliant
            } else {
                ComplianceStatus::Compliant
            }
        }
        Regulation::Pci => {
            if context.has_financial_data {
                ComplianceStatus::NeedsReview
            } else {
                ComplianceStatus::Compliant
            }
        }
        Regulation::Hipaa => {
            if context.data_types.iter().any(|d| d == "health") {
                ComplianceStatus::NeedsReview
            } else {
                ComplianceStatus::Compliant
            }
        }
        _ => ComplianceStatus::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gdpr_policy() -> CompliancePolicy {
        CompliancePolicy {
            name: "GDPR Policy".to_string(),
            regulations: vec![Regulation::Gdpr],
            checks: vec![ComplianceCheck {
                id: "gdpr-consent".to_string(),
                regulation: Regulation::Gdpr,
                name: "PII Consent Check".to_string(),
                description: "Verify user consent for PII processing".to_string(),
                severity: Severity::High,
                auto_remediate: false,
            }],
            enforcement_mode: EnforcementMode::Enforce,
        }
    }

    #[test]
    fn test_compliant_with_consent() {
        let mut agent = ComplianceAgent::new();
        agent.add_policy(gdpr_policy());
        let ctx = AuditContext::new("EU").with_pii().with_consent();
        let findings = agent.run_checks(&ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ComplianceStatus::Compliant);
    }

    #[test]
    fn test_non_compliant_without_consent() {
        let mut agent = ComplianceAgent::new();
        agent.add_policy(gdpr_policy());
        let ctx = AuditContext::new("EU").with_pii();
        let findings = agent.run_checks(&ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ComplianceStatus::NonCompliant);
    }

    #[test]
    fn test_resolve_finding() {
        let mut agent = ComplianceAgent::new();
        agent.add_policy(gdpr_policy());
        let ctx = AuditContext::new("EU").with_pii();
        let findings = agent.run_checks(&ctx);
        let fid = findings[0].id.clone();
        agent.resolve_finding(&fid).unwrap();
        assert_eq!(agent.get_findings()[0].status, ComplianceStatus::Compliant);
        assert!(agent.get_findings()[0].resolved_at.is_some());
    }

    #[test]
    fn test_non_compliant_count() {
        let mut agent = ComplianceAgent::new();
        agent.add_policy(gdpr_policy());
        let ctx = AuditContext::new("EU").with_pii();
        agent.run_checks(&ctx);
        assert_eq!(agent.non_compliant_count(), 1);
    }

    #[test]
    fn test_findings_by_regulation() {
        let mut agent = ComplianceAgent::new();
        agent.add_policy(gdpr_policy());
        let ctx = AuditContext::new("EU").with_pii();
        agent.run_checks(&ctx);
        let gdpr_findings = agent.findings_by_regulation(&Regulation::Gdpr);
        assert_eq!(gdpr_findings.len(), 1);
        let sox_findings = agent.findings_by_regulation(&Regulation::Sox);
        assert!(sox_findings.is_empty());
    }

    #[test]
    fn test_critical_findings() {
        let mut agent = ComplianceAgent::new();
        agent.add_policy(gdpr_policy());
        let ctx = AuditContext::new("EU").with_pii();
        agent.run_checks(&ctx);
        let crits = agent.critical_findings();
        assert_eq!(crits.len(), 1); // High severity + NonCompliant
    }

    #[test]
    fn test_pci_needs_review() {
        let mut agent = ComplianceAgent::new();
        agent.add_policy(CompliancePolicy {
            name: "PCI Policy".to_string(),
            regulations: vec![Regulation::Pci],
            checks: vec![ComplianceCheck {
                id: "pci-data".to_string(),
                regulation: Regulation::Pci,
                name: "Financial data check".to_string(),
                description: "Check financial data handling".to_string(),
                severity: Severity::Medium,
                auto_remediate: false,
            }],
            enforcement_mode: EnforcementMode::Enforce,
        });
        let ctx = AuditContext::new("US").with_financial_data();
        let findings = agent.run_checks(&ctx);
        assert_eq!(findings[0].status, ComplianceStatus::NeedsReview);
    }

    #[test]
    fn test_resolve_nonexistent_finding() {
        let mut agent = ComplianceAgent::new();
        assert!(agent.resolve_finding("nope").is_err());
    }

    #[test]
    fn test_multiple_policies() {
        let mut agent = ComplianceAgent::new();
        agent.add_policy(gdpr_policy());
        agent.add_policy(CompliancePolicy {
            name: "PCI Policy".to_string(),
            regulations: vec![Regulation::Pci],
            checks: vec![ComplianceCheck {
                id: "pci-data".to_string(),
                regulation: Regulation::Pci,
                name: "Financial data check".to_string(),
                description: "Check financial data handling".to_string(),
                severity: Severity::Critical,
                auto_remediate: true,
            }],
            enforcement_mode: EnforcementMode::AuditOnly,
        });
        let ctx = AuditContext::new("EU").with_pii().with_financial_data();
        let findings = agent.run_checks(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_serde_roundtrip() {
        let reg = Regulation::Gdpr;
        let json = serde_json::to_string(&reg).unwrap();
        let deserialized: Regulation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Regulation::Gdpr);
    }
}
