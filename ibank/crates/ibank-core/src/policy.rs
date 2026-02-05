use crate::types::{
    ComplianceDecision, ComplianceProof, RiskBreakdown, RiskReport, TransferIntent,
};
use std::collections::BTreeSet;

/// Operational autonomy mode for a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutonomyMode {
    PureAi,
    Hybrid,
}

/// Explicit compliance gate policy configuration.
#[derive(Debug, Clone)]
pub struct CompliancePolicyConfig {
    pub policy_version: String,
    /// Fraud/anomaly scores at or above this threshold cannot run pure autonomous mode.
    pub pure_ai_fraud_threshold: u8,
    /// Counterparty risk at or above this threshold requires review.
    pub pure_ai_counterparty_threshold: u8,
    /// Jurisdiction scores at or above this threshold require review.
    pub pure_ai_jurisdiction_threshold: u8,
    /// Explicit fraud/anomaly hard block.
    pub block_fraud_threshold: u8,
    /// Any uncertainty at/above this value cannot be auto-green.
    pub uncertainty_review_threshold: f32,
    /// Risk score uplift applied when compliance is uncertain.
    pub uncertainty_risk_penalty: u8,
}

impl Default for CompliancePolicyConfig {
    fn default() -> Self {
        Self {
            policy_version: "ibank-compliance-v1".to_string(),
            pure_ai_fraud_threshold: 60,
            pure_ai_counterparty_threshold: 70,
            pure_ai_jurisdiction_threshold: 70,
            block_fraud_threshold: 95,
            uncertainty_review_threshold: 0.30,
            uncertainty_risk_penalty: 20,
        }
    }
}

/// Deterministic risk policy configuration.
#[derive(Debug, Clone)]
pub struct RiskPolicyConfig {
    /// Pure AI may not execute transfers above this amount (minor units).
    pub pure_ai_max_amount_minor: u64,
    /// Transfers above this amount are denied.
    pub hard_limit_amount_minor: u64,
    /// Ambiguity above this value blocks pure autonomy and requires hybrid.
    pub ambiguity_hybrid_threshold: f32,
    /// Uncertainty above this value requires hybrid review.
    pub uncertainty_hybrid_threshold: f32,
    /// Anomaly score above this value is considered fraud-high and requires hybrid.
    pub fraud_hybrid_threshold: u8,
    /// Risk score at/above this value requires hybrid review.
    pub hybrid_score_threshold: u8,
    /// Compliance gate policy.
    pub compliance: CompliancePolicyConfig,
}

impl Default for RiskPolicyConfig {
    fn default() -> Self {
        Self {
            // 10,000.00 in cents: aligns with the requested escalation threshold.
            pure_ai_max_amount_minor: 1_000_000,
            // 250,000.00 in cents.
            hard_limit_amount_minor: 25_000_000,
            ambiguity_hybrid_threshold: 0.35,
            uncertainty_hybrid_threshold: 0.45,
            fraud_hybrid_threshold: 70,
            hybrid_score_threshold: 65,
            compliance: CompliancePolicyConfig::default(),
        }
    }
}

/// Policy decision from deterministic risk evaluation.
#[derive(Debug, Clone)]
pub enum RiskDecision {
    Allow(RiskReport),
    RequireHybrid(RiskReport),
    Deny(RiskReport),
}

/// Deterministic risk + compliance policy engine.
///
/// This logic is intentionally rule-based and free of probabilistic side effects,
/// so the same input always yields the same decision.
#[derive(Debug, Clone)]
pub struct RiskPolicyEngine {
    config: RiskPolicyConfig,
}

impl RiskPolicyEngine {
    pub fn new(config: RiskPolicyConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &RiskPolicyConfig {
        &self.config
    }

    /// Evaluate explicit compliance gate with reason codes and evidence pointers.
    pub fn evaluate_compliance(&self, intent: &TransferIntent) -> ComplianceDecision {
        let mut block_reasons = BTreeSet::new();
        let mut review_reasons = BTreeSet::new();
        let mut uncertainty_score = 0_u8;
        let evidence_pointers = compliance_evidence_pointers(intent);

        let kyc_status = intent
            .metadata
            .get("kyc_status")
            .map(|value| value.to_ascii_lowercase());
        let aml_status = intent
            .metadata
            .get("aml_status")
            .map(|value| value.to_ascii_lowercase());
        let sanctions_status = intent
            .metadata
            .get("sanctions_status")
            .map(|value| value.to_ascii_lowercase());

        if has_flag(intent, "sanctions_hit")
            || matches!(
                sanctions_status.as_deref(),
                Some("hit" | "blocked" | "positive_match")
            )
        {
            block_reasons.insert("SANCTIONS_HIT".to_string());
        }

        if has_flag(intent, "invalid_kyc")
            || matches!(
                kyc_status.as_deref(),
                Some("invalid" | "rejected" | "failed")
            )
        {
            block_reasons.insert("INVALID_KYC".to_string());
        }

        if has_flag(intent, "aml_blocked") || matches!(aml_status.as_deref(), Some("blocked")) {
            block_reasons.insert("AML_BLOCKED".to_string());
        }

        if has_flag(intent, "missing_kyc")
            || has_flag(intent, "manual_kyc_required")
            || matches!(kyc_status.as_deref(), Some("missing" | "pending"))
        {
            review_reasons.insert("MISSING_KYC".to_string());
        }

        if matches!(kyc_status.as_deref(), Some("unknown")) {
            review_reasons.insert("KYC_UNKNOWN".to_string());
            uncertainty_score = uncertainty_score.max(65);
        }

        if has_flag(intent, "aml_review")
            || matches!(aml_status.as_deref(), Some("review" | "pending"))
        {
            review_reasons.insert("AML_REVIEW_REQUIRED".to_string());
        }

        if matches!(aml_status.as_deref(), Some("unknown")) {
            review_reasons.insert("AML_UNKNOWN".to_string());
            uncertainty_score = uncertainty_score.max(60);
        }

        if matches!(sanctions_status.as_deref(), Some("unknown")) {
            review_reasons.insert("SANCTIONS_UNKNOWN".to_string());
            uncertainty_score = uncertainty_score.max(70);
        }

        let jurisdiction = jurisdiction_score(&intent.jurisdiction);
        if jurisdiction >= self.config.compliance.pure_ai_jurisdiction_threshold {
            review_reasons.insert("JURISDICTION_POLICY_REVIEW".to_string());
        }

        if intent.anomaly_score >= self.config.compliance.block_fraud_threshold {
            block_reasons.insert("FRAUD_BLOCK".to_string());
        } else if intent.anomaly_score >= self.config.compliance.pure_ai_fraud_threshold {
            review_reasons.insert("FRAUD_ESCALATION".to_string());
        }

        if intent.counterparty_risk >= self.config.compliance.pure_ai_counterparty_threshold {
            review_reasons.insert("COUNTERPARTY_RISK_REVIEW".to_string());
        }

        if intent.model_uncertainty >= self.config.compliance.uncertainty_review_threshold
            || has_flag(intent, "compliance_uncertain")
        {
            review_reasons.insert("MODEL_UNCERTAINTY_REVIEW".to_string());
            uncertainty_score = uncertainty_score.max(scale_uncertainty(intent.model_uncertainty));
        }

        if evidence_pointers.is_empty() {
            review_reasons.insert("MISSING_EVIDENCE".to_string());
            uncertainty_score = uncertainty_score.max(80);
        }

        if !block_reasons.is_empty() {
            return ComplianceDecision::block(
                block_reasons.into_iter().collect(),
                evidence_pointers,
            );
        }

        if !review_reasons.is_empty() {
            return ComplianceDecision::review_required(
                review_reasons.into_iter().collect(),
                evidence_pointers,
                uncertainty_score,
            );
        }

        ComplianceDecision::green(
            vec!["BASELINE_CHECKS_PASSED".to_string()],
            evidence_pointers,
        )
    }

    /// Build a redacted compliance proof suitable for commitment platform data.
    pub fn generate_compliance_proof(&self, decision: &ComplianceDecision) -> ComplianceProof {
        let mut evidence_hashes = decision
            .evidence_pointers
            .iter()
            .map(|pointer| blake3::hash(pointer.as_bytes()).to_hex().to_string())
            .collect::<Vec<_>>();
        evidence_hashes.sort();
        evidence_hashes.dedup();

        let mut reason_codes = decision.reasons.clone();
        reason_codes.sort();
        reason_codes.dedup();

        ComplianceProof {
            policy_version: self.config.compliance.policy_version.clone(),
            decision: decision.state.clone(),
            reason_codes,
            evidence_hashes,
        }
    }

    pub fn evaluate(&self, intent: &TransferIntent, mode: AutonomyMode) -> RiskDecision {
        let compliance = self.evaluate_compliance(intent);
        self.evaluate_with_compliance(intent, mode, &compliance)
    }

    pub fn evaluate_with_compliance(
        &self,
        intent: &TransferIntent,
        mode: AutonomyMode,
        compliance: &ComplianceDecision,
    ) -> RiskDecision {
        let mut reasons = compliance
            .reasons
            .iter()
            .map(|reason| format!("compliance:{reason}"))
            .collect::<Vec<_>>();

        // Hard-deny conditions are deterministic and independent of autonomy mode.
        if intent.amount_minor > self.config.hard_limit_amount_minor {
            reasons.push(format!(
                "amount {} exceeds hard limit {}",
                intent.amount_minor, self.config.hard_limit_amount_minor
            ));
            return RiskDecision::Deny(RiskReport {
                score: 100,
                reasons,
                factors: RiskBreakdown {
                    amount: 100,
                    counterparty: intent.counterparty_risk.min(100),
                    jurisdiction: jurisdiction_score(&intent.jurisdiction),
                    anomaly: intent.anomaly_score.min(100),
                    model_uncertainty: scale_uncertainty(intent.model_uncertainty),
                },
                fraud_score: intent.anomaly_score.min(100),
                blocking_ambiguity: false,
                requires_hybrid: false,
                denied: true,
            });
        }

        if compliance.is_block() {
            if reasons.is_empty() {
                reasons.push("compliance:block".to_string());
            }
            return RiskDecision::Deny(RiskReport {
                score: 100,
                reasons,
                factors: RiskBreakdown {
                    amount: amount_score(intent.amount_minor, self.config.pure_ai_max_amount_minor),
                    counterparty: intent.counterparty_risk.min(100),
                    jurisdiction: jurisdiction_score(&intent.jurisdiction),
                    anomaly: intent.anomaly_score.min(100),
                    model_uncertainty: scale_uncertainty(intent.model_uncertainty),
                },
                fraud_score: intent.anomaly_score.min(100),
                blocking_ambiguity: false,
                requires_hybrid: false,
                denied: true,
            });
        }

        let amount_factor = amount_score(intent.amount_minor, self.config.pure_ai_max_amount_minor);
        let counterparty_factor = intent.counterparty_risk.min(100);
        let jurisdiction_factor = jurisdiction_score(&intent.jurisdiction);
        let anomaly_factor = intent.anomaly_score.min(100);
        let uncertainty_factor = scale_uncertainty(intent.model_uncertainty);

        // Weighted deterministic score over all required factors.
        // Integer arithmetic keeps results stable across platforms.
        let mut score = ((amount_factor as u16 * 30
            + counterparty_factor as u16 * 20
            + jurisdiction_factor as u16 * 20
            + anomaly_factor as u16 * 20
            + uncertainty_factor as u16 * 10)
            / 100) as u8;

        let blocking_ambiguity = intent.ambiguity > self.config.ambiguity_hybrid_threshold;
        if blocking_ambiguity {
            reasons.push(format!(
                "ambiguity {:.2} exceeds threshold {:.2}",
                intent.ambiguity, self.config.ambiguity_hybrid_threshold
            ));
        }

        if intent.model_uncertainty > self.config.uncertainty_hybrid_threshold {
            reasons.push(format!(
                "model uncertainty {:.2} exceeds threshold {:.2}",
                intent.model_uncertainty, self.config.uncertainty_hybrid_threshold
            ));
        }

        if compliance.uncertainty_score > 0 {
            let penalty = self
                .config
                .compliance
                .uncertainty_risk_penalty
                .min(compliance.uncertainty_score.max(1));
            score = score.saturating_add(penalty);
            reasons.push(format!("compliance uncertainty elevated risk by {penalty}"));
        }

        if intent.dispute_flag || intent.transaction_type.eq_ignore_ascii_case("dispute") {
            reasons.push("dispute workflow requires human approval".to_string());
        }

        if intent.amount_minor > self.config.pure_ai_max_amount_minor {
            reasons.push(format!(
                "amount {} exceeds pure-ai cap {}",
                intent.amount_minor, self.config.pure_ai_max_amount_minor
            ));
        }

        if anomaly_factor >= self.config.fraud_hybrid_threshold {
            reasons.push(format!(
                "fraud/anomaly score {} exceeds threshold {}",
                anomaly_factor, self.config.fraud_hybrid_threshold
            ));
        }

        let compliance_requires_review = compliance.is_review_required();
        let compliance_uncertain = compliance.uncertainty_score > 0;

        let requires_hybrid = intent.amount_minor > self.config.pure_ai_max_amount_minor
            || blocking_ambiguity
            || intent.model_uncertainty > self.config.uncertainty_hybrid_threshold
            || intent.dispute_flag
            || anomaly_factor >= self.config.fraud_hybrid_threshold
            || score >= self.config.hybrid_score_threshold
            || compliance_requires_review
            || compliance_uncertain;

        let report = RiskReport {
            score,
            reasons,
            factors: RiskBreakdown {
                amount: amount_factor,
                counterparty: counterparty_factor,
                jurisdiction: jurisdiction_factor,
                anomaly: anomaly_factor,
                model_uncertainty: uncertainty_factor,
            },
            fraud_score: anomaly_factor,
            blocking_ambiguity,
            requires_hybrid,
            denied: false,
        };

        if requires_hybrid && matches!(mode, AutonomyMode::PureAi) {
            return RiskDecision::RequireHybrid(report);
        }

        RiskDecision::Allow(report)
    }
}

fn amount_score(amount_minor: u64, pure_ai_cap_minor: u64) -> u8 {
    if amount_minor == 0 {
        return 0;
    }

    // Scale amount relative to pure-ai cap and clamp to 100.
    let scaled = ((amount_minor as f64 / pure_ai_cap_minor.max(1) as f64) * 100.0).round() as u16;
    scaled.min(100) as u8
}

fn jurisdiction_score(jurisdiction: &str) -> u8 {
    match jurisdiction.to_ascii_uppercase().as_str() {
        "US" | "CA" | "SG" | "EU" => 15,
        "UNKNOWN" => 35,
        "HIGH_RISK" | "SANCTIONED_BORDER" => 80,
        _ => 25,
    }
}

fn scale_uncertainty(uncertainty: f32) -> u8 {
    (uncertainty.clamp(0.0, 1.0) * 100.0) as u8
}

fn has_flag(intent: &TransferIntent, expected: &str) -> bool {
    intent
        .compliance_flags
        .iter()
        .any(|flag| flag.eq_ignore_ascii_case(expected))
}

fn compliance_evidence_pointers(intent: &TransferIntent) -> Vec<String> {
    let mut pointers = intent
        .metadata
        .iter()
        .filter(|(key, _)| key.starts_with("evidence_"))
        .map(|(_, value)| value.clone())
        .collect::<Vec<_>>();
    pointers.push(format!("proof://{}/kyc", intent.trace_id));
    pointers.push(format!("proof://{}/aml", intent.trace_id));
    pointers.push(format!("proof://{}/sanctions", intent.trace_id));
    pointers.sort();
    pointers.dedup();
    pointers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_intent(amount_minor: u64) -> TransferIntent {
        TransferIntent::new(
            "origin",
            "counterparty",
            amount_minor,
            "USD",
            "ach",
            "acct-1",
            "invoice payment",
        )
        .with_risk_inputs("US", 10, 10, 0.05)
    }

    #[test]
    fn sanctioned_counterparty_results_in_block() {
        let engine = RiskPolicyEngine::new(RiskPolicyConfig::default());
        let mut intent = base_intent(50_000);
        intent.compliance_flags.push("sanctions_hit".to_string());

        let decision = engine.evaluate_compliance(&intent);
        assert!(decision.is_block());
        assert!(decision
            .reasons
            .iter()
            .any(|reason| reason == "SANCTIONS_HIT"));
    }

    #[test]
    fn missing_kyc_results_in_review_required() {
        let engine = RiskPolicyEngine::new(RiskPolicyConfig::default());
        let mut intent = base_intent(50_000);
        intent
            .metadata
            .insert("kyc_status".to_string(), "missing".to_string());

        let decision = engine.evaluate_compliance(&intent);
        assert!(decision.is_review_required());
        assert!(decision
            .reasons
            .iter()
            .any(|reason| reason == "MISSING_KYC"));
    }

    #[test]
    fn low_risk_results_in_green() {
        let engine = RiskPolicyEngine::new(RiskPolicyConfig::default());
        let intent = base_intent(50_000);

        let decision = engine.evaluate_compliance(&intent);
        assert!(decision.is_green());
    }

    #[test]
    fn pure_ai_requires_hybrid_above_10k() {
        let engine = RiskPolicyEngine::new(RiskPolicyConfig::default());
        let intent = base_intent(1_100_000);

        match engine.evaluate(&intent, AutonomyMode::PureAi) {
            RiskDecision::RequireHybrid(report) => {
                assert!(report.requires_hybrid);
                assert!(report
                    .reasons
                    .iter()
                    .any(|r| r.contains("exceeds pure-ai cap")));
            }
            other => panic!("expected hybrid requirement, got {:?}", other),
        }
    }

    #[test]
    fn dispute_requires_hybrid_even_for_small_amount() {
        let engine = RiskPolicyEngine::new(RiskPolicyConfig::default());
        let intent = base_intent(50_000).with_transaction_type("dispute", true);

        match engine.evaluate(&intent, AutonomyMode::PureAi) {
            RiskDecision::RequireHybrid(report) => {
                assert!(report
                    .reasons
                    .iter()
                    .any(|r| r.contains("requires human approval")));
            }
            other => panic!("expected hybrid requirement, got {:?}", other),
        }
    }

    #[test]
    fn compliance_deny_is_deterministic() {
        let engine = RiskPolicyEngine::new(RiskPolicyConfig::default());
        let mut intent = base_intent(50_000);
        intent.compliance_flags.push("sanctions_hit".to_string());

        match engine.evaluate(&intent, AutonomyMode::Hybrid) {
            RiskDecision::Deny(report) => {
                assert!(report.denied);
                assert!(report.score >= 100);
            }
            other => panic!("expected deny, got {:?}", other),
        }
    }

    #[test]
    fn high_fraud_forces_hybrid() {
        let engine = RiskPolicyEngine::new(RiskPolicyConfig::default());
        let intent = base_intent(20_000).with_risk_inputs("US", 15, 90, 0.1);

        match engine.evaluate(&intent, AutonomyMode::PureAi) {
            RiskDecision::RequireHybrid(report) => {
                assert!(report.fraud_score >= 90);
            }
            other => panic!("expected hybrid requirement, got {:?}", other),
        }
    }

    #[test]
    fn uncertainty_must_not_auto_green() {
        let engine = RiskPolicyEngine::new(RiskPolicyConfig::default());
        let intent = base_intent(50_000).with_risk_inputs("US", 10, 10, 0.65);
        let decision = engine.evaluate_compliance(&intent);
        assert!(decision.is_review_required());

        match engine.evaluate_with_compliance(&intent, AutonomyMode::PureAi, &decision) {
            RiskDecision::RequireHybrid(report) => {
                assert!(report
                    .reasons
                    .iter()
                    .any(|reason| reason.contains("compliance uncertainty elevated risk")));
            }
            other => panic!("expected hybrid requirement, got {:?}", other),
        }
    }
}
