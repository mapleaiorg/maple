use crate::types::{RiskBreakdown, RiskReport, TransferIntent};

/// Operational autonomy mode for a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutonomyMode {
    PureAi,
    Hybrid,
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

/// Deterministic risk engine.
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

    pub fn evaluate(&self, intent: &TransferIntent, mode: AutonomyMode) -> RiskDecision {
        let mut reasons = Vec::new();

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

        for flag in &intent.compliance_flags {
            if matches!(flag.as_str(), "sanctions_hit" | "aml_blocked") {
                reasons.push(format!("compliance flag '{}' enforces deny", flag));
                return RiskDecision::Deny(RiskReport {
                    score: 100,
                    reasons,
                    factors: RiskBreakdown {
                        amount: amount_score(
                            intent.amount_minor,
                            self.config.pure_ai_max_amount_minor,
                        ),
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
        }

        let amount_factor = amount_score(intent.amount_minor, self.config.pure_ai_max_amount_minor);
        let counterparty_factor = intent.counterparty_risk.min(100);
        let jurisdiction_factor = jurisdiction_score(&intent.jurisdiction);
        let anomaly_factor = intent.anomaly_score.min(100);
        let uncertainty_factor = scale_uncertainty(intent.model_uncertainty);

        // Weighted deterministic score over all required factors.
        // Integer arithmetic keeps results stable across platforms.
        let score = ((amount_factor as u16 * 30
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

        if intent
            .compliance_flags
            .iter()
            .any(|f| matches!(f.as_str(), "pep_review" | "manual_kyc_required"))
        {
            reasons.push("compliance review requires hybrid approval".to_string());
        }

        let requires_hybrid = intent.amount_minor > self.config.pure_ai_max_amount_minor
            || blocking_ambiguity
            || intent.model_uncertainty > self.config.uncertainty_hybrid_threshold
            || intent.dispute_flag
            || anomaly_factor >= self.config.fraud_hybrid_threshold
            || score >= self.config.hybrid_score_threshold
            || intent
                .compliance_flags
                .iter()
                .any(|f| matches!(f.as_str(), "pep_review" | "manual_kyc_required"));

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
}
