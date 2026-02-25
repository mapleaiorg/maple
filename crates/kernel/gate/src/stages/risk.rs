use async_trait::async_trait;
use maple_mwl_types::{DenialReason, EffectDomain, Reversibility, RiskClass, RiskLevel};
use serde::{Deserialize, Serialize};

use crate::context::{GateContext, StageResult};
use crate::error::GateError;
use crate::traits::GateStage;

/// Risk configuration for the Gate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskConfig {
    /// Maximum allowed risk class (anything above is denied)
    pub max_allowed_risk: RiskClass,
    /// Risk class threshold that requires human approval
    pub human_approval_threshold: RiskClass,
    /// Base risk score for irreversible actions
    pub irreversible_penalty: f64,
    /// Risk multiplier for financial domain
    pub financial_multiplier: f64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_allowed_risk: RiskClass::High,
            human_approval_threshold: RiskClass::Critical,
            irreversible_penalty: 0.3,
            financial_multiplier: 1.5,
        }
    }
}

/// Stage 5: Risk Assessment
///
/// Scores risk based on scope, reversibility, effect domain, and number of targets.
/// Compares against configured thresholds and may deny or require escalation.
pub struct RiskAssessmentStage {
    config: RiskConfig,
}

impl RiskAssessmentStage {
    pub fn new(config: RiskConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl GateStage for RiskAssessmentStage {
    fn stage_name(&self) -> &str {
        "Risk Assessment"
    }

    fn stage_number(&self) -> u8 {
        5
    }

    async fn evaluate(&self, context: &mut GateContext) -> Result<StageResult, GateError> {
        let decl = &context.declaration;

        // Calculate risk score
        let mut score: f64 = 0.2; // base risk
        let mut factors = Vec::new();

        // Reversibility factor
        match &decl.reversibility {
            Reversibility::Irreversible => {
                score += self.config.irreversible_penalty;
                factors.push("Irreversible action".into());
            }
            Reversibility::Conditional { conditions } => {
                score += self.config.irreversible_penalty * 0.5;
                factors.push(format!(
                    "Conditional reversal: {} conditions",
                    conditions.len()
                ));
            }
            Reversibility::TimeWindow { window_ms } => {
                if *window_ms < 60_000 {
                    score += 0.1;
                    factors.push("Short reversal window (<1min)".into());
                }
            }
            Reversibility::FullyReversible => {
                // No penalty
            }
        }

        // Effect domain factor
        match &decl.scope.effect_domain {
            EffectDomain::Financial => {
                score *= self.config.financial_multiplier;
                factors.push("Financial domain (elevated risk)".into());
            }
            EffectDomain::Infrastructure => {
                score += 0.2;
                factors.push("Infrastructure domain".into());
            }
            EffectDomain::Governance => {
                score += 0.15;
                factors.push("Governance domain".into());
            }
            _ => {}
        }

        // Target count factor
        let target_count = decl.scope.targets.len() + decl.affected_parties.len();
        if target_count > 10 {
            score += 0.15;
            factors.push(format!("High target count: {}", target_count));
        } else if target_count > 5 {
            score += 0.05;
            factors.push(format!("Moderate target count: {}", target_count));
        }

        // Confidence factor (inverse: low confidence = higher risk)
        if decl.confidence.overall < 0.7 {
            score += 0.1;
            factors.push(format!(
                "Below-threshold confidence: {:.2}",
                decl.confidence.overall
            ));
        }

        // Clamp score to [0, 1]
        score = score.clamp(0.0, 1.0);

        // Determine risk class
        let risk_class = if score >= 0.8 {
            RiskClass::Critical
        } else if score >= 0.6 {
            RiskClass::High
        } else if score >= 0.3 {
            RiskClass::Medium
        } else {
            RiskClass::Low
        };

        let risk_level = RiskLevel {
            class: risk_class,
            score: Some(score),
            factors,
        };

        // Store assessment
        context.risk_assessment = Some(risk_level.clone());

        // Check against thresholds
        if risk_class > self.config.max_allowed_risk {
            return Ok(StageResult::Deny(DenialReason {
                code: "RISK_TOO_HIGH".into(),
                message: format!(
                    "Risk {:?} (score {:.2}) exceeds maximum allowed {:?}",
                    risk_class, score, self.config.max_allowed_risk
                ),
                policy_refs: vec![],
            }));
        }

        if risk_class >= self.config.human_approval_threshold {
            return Ok(StageResult::RequireHumanApproval(format!(
                "Risk {:?} (score {:.2}) requires human approval",
                risk_class, score
            )));
        }

        Ok(StageResult::Pass)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::GateContext;
    use crate::declaration::CommitmentDeclaration;
    use maple_mwl_types::{CommitmentScope, IdentityMaterial, WorldlineId};

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn low_risk_scope() -> CommitmentScope {
        CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![test_worldline()],
            constraints: vec![],
        }
    }

    fn high_risk_scope() -> CommitmentScope {
        CommitmentScope {
            effect_domain: EffectDomain::Financial,
            targets: vec![test_worldline()],
            constraints: vec![],
        }
    }

    #[tokio::test]
    async fn pass_low_risk() {
        let stage = RiskAssessmentStage::new(RiskConfig::default());
        let decl = CommitmentDeclaration::builder(test_worldline(), low_risk_scope()).build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_pass());
        assert!(ctx.risk_assessment.is_some());
    }

    #[tokio::test]
    async fn deny_above_threshold() {
        let config = RiskConfig {
            max_allowed_risk: RiskClass::Low,
            ..Default::default()
        };
        let stage = RiskAssessmentStage::new(config);

        // Financial + irreversible = high risk
        let decl = CommitmentDeclaration::builder(test_worldline(), high_risk_scope())
            .reversibility(Reversibility::Irreversible)
            .build();
        let mut ctx = GateContext::new(decl);
        let result = stage.evaluate(&mut ctx).await.unwrap();
        assert!(result.is_deny());
    }

    #[tokio::test]
    async fn risk_assessment_stored() {
        let stage = RiskAssessmentStage::new(RiskConfig::default());
        let decl = CommitmentDeclaration::builder(test_worldline(), low_risk_scope()).build();
        let mut ctx = GateContext::new(decl);
        stage.evaluate(&mut ctx).await.unwrap();
        let risk = ctx.risk_assessment.unwrap();
        assert!(risk.score.is_some());
        assert!(!risk.factors.is_empty() || risk.class == RiskClass::Low);
    }
}
