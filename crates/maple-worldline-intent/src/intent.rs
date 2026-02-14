//! Self-regeneration intent — a concrete, validated plan for self-modification.
//!
//! An intent represents a specific, actionable proposal derived from converged
//! meanings. It includes impact assessment, improvement estimates, governance
//! tier classification, and reversibility information.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use maple_worldline_meaning::MeaningId;

use crate::proposal::RegenerationProposal;
use crate::types::{ChangeType, IntentConfig, IntentId, ReversibilityLevel, SubstrateTier};

// ── Intent Status ──────────────────────────────────────────────────────

/// Lifecycle status of a self-regeneration intent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IntentStatus {
    /// Intent is being formed from meanings.
    Forming,
    /// Intent has stabilized and is ready for validation.
    Stabilized,
    /// Intent has passed validation.
    Validated,
    /// Intent was deferred with a reason.
    Deferred(String),
    /// Intent was abandoned with a reason.
    Abandoned(String),
}

impl std::fmt::Display for IntentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forming => write!(f, "forming"),
            Self::Stabilized => write!(f, "stabilized"),
            Self::Validated => write!(f, "validated"),
            Self::Deferred(reason) => write!(f, "deferred: {}", reason),
            Self::Abandoned(reason) => write!(f, "abandoned: {}", reason),
        }
    }
}

// ── Impact Assessment ──────────────────────────────────────────────────

/// Assessment of the impact a proposed change will have.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImpactAssessment {
    /// Components affected by this change.
    pub affected_components: Vec<String>,
    /// Overall risk score (0.0 = no risk, 1.0 = maximum risk).
    pub risk_score: f64,
    /// Specific risk factors identified.
    pub risk_factors: Vec<String>,
    /// Description of the blast radius.
    pub blast_radius: String,
}

impl ImpactAssessment {
    /// Whether the risk is within acceptable bounds.
    pub fn is_acceptable(&self, config: &IntentConfig) -> bool {
        self.risk_score <= config.max_risk
    }
}

// ── Improvement Estimate ───────────────────────────────────────────────

/// Estimated improvement from applying a regeneration intent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImprovementEstimate {
    /// Metric being improved (e.g., "latency", "throughput").
    pub metric: String,
    /// Current measured value.
    pub current_value: f64,
    /// Projected value after change.
    pub projected_value: f64,
    /// Confidence in the projection (0.0–1.0).
    pub confidence: f64,
    /// Unit of measurement.
    pub unit: String,
}

impl ImprovementEstimate {
    /// Relative improvement factor (projected / current).
    pub fn improvement_factor(&self) -> f64 {
        if self.current_value.abs() < f64::EPSILON {
            return 0.0;
        }
        (self.projected_value - self.current_value) / self.current_value
    }
}

impl std::fmt::Display for ImprovementEstimate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {:.2}{} → {:.2}{} (confidence={:.2})",
            self.metric, self.current_value, self.unit, self.projected_value, self.unit, self.confidence
        )
    }
}

// ── Self-Regeneration Intent ───────────────────────────────────────────

/// A fully formed self-regeneration intent ready for commitment evaluation.
///
/// An intent is the last stage before crossing the commitment boundary.
/// It contains a concrete proposal, validated impact assessment, and
/// governance classification.
#[derive(Clone, Debug)]
pub struct SelfRegenerationIntent {
    /// Unique identifier.
    pub id: IntentId,
    /// Meaning IDs this intent was derived from.
    pub derived_from: Vec<MeaningId>,
    /// Classification of the proposed change.
    pub change_type: ChangeType,
    /// The concrete proposal.
    pub proposal: RegenerationProposal,
    /// Confidence in this intent (0.0–1.0).
    pub confidence: f64,
    /// Reversibility level.
    pub reversibility: ReversibilityLevel,
    /// Impact assessment.
    pub impact: ImpactAssessment,
    /// Governance tier (determines required scrutiny level).
    pub governance_tier: SubstrateTier,
    /// Estimated improvement.
    pub estimated_improvement: ImprovementEstimate,
    /// When this intent was stabilized.
    pub stabilized_at: DateTime<Utc>,
    /// Current lifecycle status.
    pub status: IntentStatus,
}

impl SelfRegenerationIntent {
    /// Check if this intent is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, IntentStatus::Abandoned(_))
    }

    /// Check if this intent is actionable (validated and not deferred/abandoned).
    pub fn is_actionable(&self) -> bool {
        matches!(self.status, IntentStatus::Validated)
    }

    /// Minimum observation period required by governance tier (seconds).
    pub fn required_observation_secs(&self) -> u64 {
        self.governance_tier.min_observation_secs()
    }
}

impl std::fmt::Display for SelfRegenerationIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} (tier={}, confidence={:.2}, risk={:.2})",
            self.id,
            self.change_type,
            self.governance_tier,
            self.confidence,
            self.impact.risk_score,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proposal::{RegenerationProposal, RollbackPlan, RollbackStrategy};
    use crate::types::ProposalId;

    fn make_intent(confidence: f64, risk: f64) -> SelfRegenerationIntent {
        SelfRegenerationIntent {
            id: IntentId::new(),
            derived_from: vec![MeaningId::new()],
            change_type: ChangeType::ConfigurationChange {
                parameter: "batch_size".into(),
                current_value: "32".into(),
                proposed_value: "64".into(),
                rationale: "improve throughput".into(),
            },
            proposal: RegenerationProposal {
                id: ProposalId::new(),
                summary: "Increase batch size".into(),
                rationale: "Current batch size is suboptimal".into(),
                affected_components: vec!["scheduler".into()],
                code_changes: vec![],
                required_tests: vec![],
                performance_gates: vec![],
                safety_checks: vec![],
                estimated_improvement: ImprovementEstimate {
                    metric: "throughput".into(),
                    current_value: 100.0,
                    projected_value: 150.0,
                    confidence: 0.8,
                    unit: "ops/s".into(),
                },
                risk_score: risk,
                rollback_plan: RollbackPlan {
                    strategy: RollbackStrategy::ConfigRestore,
                    steps: vec!["restore batch_size=32".into()],
                    estimated_duration_secs: 60,
                },
            },
            confidence,
            reversibility: ReversibilityLevel::FullyReversible,
            impact: ImpactAssessment {
                affected_components: vec!["scheduler".into()],
                risk_score: risk,
                risk_factors: vec![],
                blast_radius: "scheduler only".into(),
            },
            governance_tier: SubstrateTier::Tier0,
            estimated_improvement: ImprovementEstimate {
                metric: "throughput".into(),
                current_value: 100.0,
                projected_value: 150.0,
                confidence: 0.8,
                unit: "ops/s".into(),
            },
            stabilized_at: Utc::now(),
            status: IntentStatus::Validated,
        }
    }

    #[test]
    fn intent_status_display() {
        assert_eq!(IntentStatus::Forming.to_string(), "forming");
        assert_eq!(IntentStatus::Validated.to_string(), "validated");
        assert_eq!(
            IntentStatus::Deferred("too risky".into()).to_string(),
            "deferred: too risky"
        );
    }

    #[test]
    fn impact_acceptable_within_bounds() {
        let config = IntentConfig::default();
        let impact = ImpactAssessment {
            affected_components: vec!["a".into()],
            risk_score: 0.2,
            risk_factors: vec![],
            blast_radius: "small".into(),
        };
        assert!(impact.is_acceptable(&config));

        let high_risk = ImpactAssessment {
            risk_score: 0.9,
            ..impact
        };
        assert!(!high_risk.is_acceptable(&config));
    }

    #[test]
    fn improvement_factor_calculation() {
        let est = ImprovementEstimate {
            metric: "latency".into(),
            current_value: 100.0,
            projected_value: 80.0,
            confidence: 0.9,
            unit: "ms".into(),
        };
        assert!((est.improvement_factor() - (-0.2)).abs() < f64::EPSILON);

        // Zero current value → factor is 0
        let zero = ImprovementEstimate {
            current_value: 0.0,
            ..est
        };
        assert!((zero.improvement_factor()).abs() < f64::EPSILON);
    }

    #[test]
    fn intent_is_actionable() {
        let intent = make_intent(0.9, 0.2);
        assert!(intent.is_actionable());
        assert!(!intent.is_terminal());
    }

    #[test]
    fn intent_display_format() {
        let intent = make_intent(0.9, 0.2);
        let display = intent.to_string();
        assert!(display.contains("intent:"));
        assert!(display.contains("tier-0-config"));
        assert!(display.contains("0.90"));
    }

    #[test]
    fn intent_governance_observation_period() {
        let mut intent = make_intent(0.9, 0.2);
        assert_eq!(intent.required_observation_secs(), 1800); // Tier0 = 30min

        intent.governance_tier = SubstrateTier::Tier3;
        assert_eq!(intent.required_observation_secs(), 259200); // Tier3 = 72hr
    }
}
