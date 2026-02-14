//! Intent prioritizer — ranks intents by composite score.
//!
//! Computes a weighted score from improvement potential, risk level,
//! confidence, and urgency to determine execution order.

use crate::intent::SelfRegenerationIntent;
use crate::types::IntentConfig;

// ── Prioritization Weights ─────────────────────────────────────────────

/// Weights for composing the prioritization score.
#[derive(Clone, Debug)]
pub struct PrioritizationWeights {
    /// Weight for improvement potential (higher improvement → higher priority).
    pub improvement_weight: f64,
    /// Weight for risk (lower risk → higher priority).
    pub risk_weight: f64,
    /// Weight for confidence (higher confidence → higher priority).
    pub confidence_weight: f64,
    /// Weight for urgency (governance tier affects urgency).
    pub urgency_weight: f64,
}

impl Default for PrioritizationWeights {
    fn default() -> Self {
        Self {
            improvement_weight: 0.3,
            risk_weight: 0.25,
            confidence_weight: 0.25,
            urgency_weight: 0.2,
        }
    }
}

// ── Intent Prioritizer ─────────────────────────────────────────────────

/// Ranks intents by composite score for execution ordering.
pub struct IntentPrioritizer {
    /// Scoring weights.
    pub weights: PrioritizationWeights,
}

impl Default for IntentPrioritizer {
    fn default() -> Self {
        Self {
            weights: PrioritizationWeights::default(),
        }
    }
}

impl IntentPrioritizer {
    /// Create a prioritizer from an intent configuration.
    pub fn from_config(config: &IntentConfig) -> Self {
        Self {
            weights: PrioritizationWeights {
                improvement_weight: config.improvement_weight,
                risk_weight: config.risk_weight,
                confidence_weight: config.confidence_weight,
                urgency_weight: config.urgency_weight,
            },
        }
    }

    /// Compute the composite priority score for an intent.
    ///
    /// Score components:
    /// - **Improvement**: absolute value of improvement factor (0–1 clamped)
    /// - **Risk**: inverted risk score (lower risk → higher component)
    /// - **Confidence**: direct confidence value
    /// - **Urgency**: derived from governance tier (Tier0=0.3, Tier3=1.0)
    pub fn score(&self, intent: &SelfRegenerationIntent) -> f64 {
        let improvement = intent
            .estimated_improvement
            .improvement_factor()
            .abs()
            .min(1.0);

        let risk_component = 1.0 - intent.impact.risk_score;
        let confidence_component = intent.confidence;

        let urgency_component = match intent.governance_tier {
            crate::types::SubstrateTier::Tier0 => 0.3,
            crate::types::SubstrateTier::Tier1 => 0.5,
            crate::types::SubstrateTier::Tier2 => 0.7,
            crate::types::SubstrateTier::Tier3 => 1.0,
        };

        self.weights.improvement_weight * improvement
            + self.weights.risk_weight * risk_component
            + self.weights.confidence_weight * confidence_component
            + self.weights.urgency_weight * urgency_component
    }

    /// Sort intents by priority (highest score first).
    pub fn prioritize(&self, intents: &mut [SelfRegenerationIntent]) {
        intents.sort_by(|a, b| {
            let score_a = self.score(a);
            let score_b = self.score(b);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{ImpactAssessment, ImprovementEstimate, IntentStatus};
    use crate::proposal::{RegenerationProposal, RollbackPlan, RollbackStrategy};
    use crate::types::{
        ChangeType, IntentId, ProposalId, ReversibilityLevel, SubstrateTier,
    };
    use chrono::Utc;
    use maple_worldline_meaning::MeaningId;

    fn make_intent(confidence: f64, risk: f64, tier: SubstrateTier) -> SelfRegenerationIntent {
        SelfRegenerationIntent {
            id: IntentId::new(),
            derived_from: vec![MeaningId::new()],
            change_type: ChangeType::ConfigurationChange {
                parameter: "test".into(),
                current_value: "1".into(),
                proposed_value: "2".into(),
                rationale: "test".into(),
            },
            proposal: RegenerationProposal {
                id: ProposalId::new(),
                summary: "test".into(),
                rationale: "test".into(),
                affected_components: vec![],
                code_changes: vec![],
                required_tests: vec![],
                performance_gates: vec![],
                safety_checks: vec![],
                estimated_improvement: ImprovementEstimate {
                    metric: "latency".into(),
                    current_value: 100.0,
                    projected_value: 80.0,
                    confidence,
                    unit: "ms".into(),
                },
                risk_score: risk,
                rollback_plan: RollbackPlan {
                    strategy: RollbackStrategy::ConfigRestore,
                    steps: vec!["restore".into()],
                    estimated_duration_secs: 60,
                },
            },
            confidence,
            reversibility: ReversibilityLevel::FullyReversible,
            impact: ImpactAssessment {
                affected_components: vec!["test".into()],
                risk_score: risk,
                risk_factors: vec![],
                blast_radius: "test".into(),
            },
            governance_tier: tier,
            estimated_improvement: ImprovementEstimate {
                metric: "latency".into(),
                current_value: 100.0,
                projected_value: 80.0,
                confidence,
                unit: "ms".into(),
            },
            stabilized_at: Utc::now(),
            status: IntentStatus::Validated,
        }
    }

    #[test]
    fn higher_confidence_scores_higher() {
        let p = IntentPrioritizer::default();
        let high = make_intent(0.95, 0.1, SubstrateTier::Tier0);
        let low = make_intent(0.5, 0.1, SubstrateTier::Tier0);
        assert!(p.score(&high) > p.score(&low));
    }

    #[test]
    fn lower_risk_scores_higher() {
        let p = IntentPrioritizer::default();
        let safe = make_intent(0.8, 0.1, SubstrateTier::Tier0);
        let risky = make_intent(0.8, 0.9, SubstrateTier::Tier0);
        assert!(p.score(&safe) > p.score(&risky));
    }

    #[test]
    fn prioritize_sorts_descending() {
        let p = IntentPrioritizer::default();
        let mut intents = vec![
            make_intent(0.5, 0.5, SubstrateTier::Tier0),
            make_intent(0.95, 0.1, SubstrateTier::Tier1),
            make_intent(0.7, 0.3, SubstrateTier::Tier0),
        ];
        p.prioritize(&mut intents);

        let scores: Vec<f64> = intents.iter().map(|i| p.score(i)).collect();
        for w in scores.windows(2) {
            assert!(w[0] >= w[1], "Should be sorted descending");
        }
    }

    #[test]
    fn from_config_applies_weights() {
        let config = IntentConfig {
            improvement_weight: 0.5,
            risk_weight: 0.1,
            confidence_weight: 0.1,
            urgency_weight: 0.3,
            ..IntentConfig::default()
        };
        let p = IntentPrioritizer::from_config(&config);
        assert!((p.weights.improvement_weight - 0.5).abs() < f64::EPSILON);
        assert!((p.weights.urgency_weight - 0.3).abs() < f64::EPSILON);
    }
}
