//! Deferral manager — decides when intents should be deferred.
//!
//! Evaluates system conditions and intent properties to determine
//! whether an intent should proceed or be deferred. Deferral is
//! non-destructive: deferred intents can be re-evaluated later.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::intent::SelfRegenerationIntent;
use crate::types::IntentConfig;

// ── Deferral Reason ────────────────────────────────────────────────────

/// Reason an intent was deferred.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeferralReason {
    /// Confidence is below the minimum threshold.
    InsufficientConfidence { current: f64, required: f64 },
    /// Not enough evidence backing the intent.
    InsufficientEvidence { current: usize, required: usize },
    /// Risk exceeds the acceptable threshold.
    RiskExceedsThreshold { risk: f64, threshold: f64 },
    /// System is currently under heavy load.
    SystemUnderLoad { current_load: f64, max_load: f64 },
    /// Too many concurrent regenerations in progress.
    ConcurrentRegeneration { active: usize, max: usize },
    /// A recent modification is still within cooldown period.
    RecentModification { cooldown_remaining_secs: u64 },
    /// Safety constraint prevents proceeding.
    SafetyConstraint { description: String },
    /// Human review is required for this tier.
    HumanReviewRequired { governance_tier: String },
    /// Source meaning is still forming.
    MeaningStillForming { meaning_id: String },
}

impl std::fmt::Display for DeferralReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientConfidence { current, required } => {
                write!(f, "confidence {:.2} < required {:.2}", current, required)
            }
            Self::InsufficientEvidence { current, required } => {
                write!(f, "evidence count {} < required {}", current, required)
            }
            Self::RiskExceedsThreshold { risk, threshold } => {
                write!(f, "risk {:.2} > threshold {:.2}", risk, threshold)
            }
            Self::SystemUnderLoad {
                current_load,
                max_load,
            } => {
                write!(f, "system load {:.2} > max {:.2}", current_load, max_load)
            }
            Self::ConcurrentRegeneration { active, max } => {
                write!(f, "concurrent regenerations {} >= max {}", active, max)
            }
            Self::RecentModification {
                cooldown_remaining_secs,
            } => {
                write!(f, "cooldown remaining: {}s", cooldown_remaining_secs)
            }
            Self::SafetyConstraint { description } => {
                write!(f, "safety constraint: {}", description)
            }
            Self::HumanReviewRequired { governance_tier } => {
                write!(f, "human review required for {}", governance_tier)
            }
            Self::MeaningStillForming { meaning_id } => {
                write!(f, "meaning {} still forming", meaning_id)
            }
        }
    }
}

// ── Deferral Decision ──────────────────────────────────────────────────

/// Decision about whether an intent should proceed or be deferred.
#[derive(Clone, Debug)]
pub enum DeferralDecision {
    /// Intent can proceed.
    Proceed,
    /// Intent should be deferred.
    Defer(DeferralReason),
}

impl DeferralDecision {
    /// Whether the decision is to proceed.
    pub fn should_proceed(&self) -> bool {
        matches!(self, Self::Proceed)
    }
}

// ── Deferral Manager ───────────────────────────────────────────────────

/// Evaluates deferral conditions for intents.
pub struct DeferralManager {
    /// Maximum number of concurrent active intents.
    pub max_concurrent_intents: usize,
    /// Cool-down period after a modification (seconds).
    pub post_modification_cooldown_secs: u64,
    /// Maximum system load to allow regeneration.
    pub max_system_load: f64,
}

impl Default for DeferralManager {
    fn default() -> Self {
        Self {
            max_concurrent_intents: 3,
            post_modification_cooldown_secs: 3600,
            max_system_load: 0.8,
        }
    }
}

impl DeferralManager {
    /// Create a deferral manager from an intent configuration.
    pub fn from_config(config: &IntentConfig) -> Self {
        Self {
            max_concurrent_intents: config.max_concurrent_intents,
            post_modification_cooldown_secs: config.post_modification_cooldown_secs,
            max_system_load: config.max_system_load,
        }
    }

    /// Evaluate whether an intent should be deferred.
    ///
    /// Checks: concurrent limit, system load, post-modification cooldown,
    /// confidence threshold, risk threshold.
    pub fn evaluate(
        &self,
        intent: &SelfRegenerationIntent,
        active_count: usize,
        last_modification: Option<DateTime<Utc>>,
        system_load: f64,
    ) -> DeferralDecision {
        // Check: concurrent regeneration limit
        if active_count >= self.max_concurrent_intents {
            return DeferralDecision::Defer(DeferralReason::ConcurrentRegeneration {
                active: active_count,
                max: self.max_concurrent_intents,
            });
        }

        // Check: system load
        if system_load > self.max_system_load {
            return DeferralDecision::Defer(DeferralReason::SystemUnderLoad {
                current_load: system_load,
                max_load: self.max_system_load,
            });
        }

        // Check: post-modification cooldown
        if let Some(last_mod) = last_modification {
            let elapsed = (Utc::now() - last_mod).num_seconds().max(0) as u64;
            if elapsed < self.post_modification_cooldown_secs {
                return DeferralDecision::Defer(DeferralReason::RecentModification {
                    cooldown_remaining_secs: self.post_modification_cooldown_secs - elapsed,
                });
            }
        }

        // Check: tier-based confidence
        let tier_min = intent.governance_tier.min_confidence();
        if intent.confidence < tier_min {
            return DeferralDecision::Defer(DeferralReason::InsufficientConfidence {
                current: intent.confidence,
                required: tier_min,
            });
        }

        DeferralDecision::Proceed
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
    use maple_worldline_meaning::MeaningId;

    fn make_intent(confidence: f64, tier: SubstrateTier) -> SelfRegenerationIntent {
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
                    metric: "test".into(),
                    current_value: 100.0,
                    projected_value: 80.0,
                    confidence,
                    unit: "ms".into(),
                },
                risk_score: 0.1,
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
                risk_score: 0.1,
                risk_factors: vec![],
                blast_radius: "test".into(),
            },
            governance_tier: tier,
            estimated_improvement: ImprovementEstimate {
                metric: "test".into(),
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
    fn proceed_when_all_conditions_met() {
        let mgr = DeferralManager::default();
        let intent = make_intent(0.9, SubstrateTier::Tier0);
        let decision = mgr.evaluate(&intent, 0, None, 0.5);
        assert!(decision.should_proceed());
    }

    #[test]
    fn defer_when_too_many_concurrent() {
        let mgr = DeferralManager {
            max_concurrent_intents: 2,
            ..DeferralManager::default()
        };
        let intent = make_intent(0.9, SubstrateTier::Tier0);
        let decision = mgr.evaluate(&intent, 3, None, 0.5);
        assert!(!decision.should_proceed());
        assert!(matches!(
            decision,
            DeferralDecision::Defer(DeferralReason::ConcurrentRegeneration { .. })
        ));
    }

    #[test]
    fn defer_when_system_overloaded() {
        let mgr = DeferralManager::default();
        let intent = make_intent(0.9, SubstrateTier::Tier0);
        let decision = mgr.evaluate(&intent, 0, None, 0.95);
        assert!(!decision.should_proceed());
        assert!(matches!(
            decision,
            DeferralDecision::Defer(DeferralReason::SystemUnderLoad { .. })
        ));
    }

    #[test]
    fn defer_when_cooldown_active() {
        let mgr = DeferralManager {
            post_modification_cooldown_secs: 3600,
            ..DeferralManager::default()
        };
        let intent = make_intent(0.9, SubstrateTier::Tier0);
        let recent = Utc::now(); // just modified
        let decision = mgr.evaluate(&intent, 0, Some(recent), 0.5);
        assert!(!decision.should_proceed());
        assert!(matches!(
            decision,
            DeferralDecision::Defer(DeferralReason::RecentModification { .. })
        ));
    }

    #[test]
    fn defer_when_confidence_below_tier_minimum() {
        let mgr = DeferralManager::default();
        // Tier3 requires 0.9 confidence
        let intent = make_intent(0.8, SubstrateTier::Tier3);
        let decision = mgr.evaluate(&intent, 0, None, 0.5);
        assert!(!decision.should_proceed());
        assert!(matches!(
            decision,
            DeferralDecision::Defer(DeferralReason::InsufficientConfidence { .. })
        ));
    }

    #[test]
    fn deferral_reason_display() {
        let reason = DeferralReason::ConcurrentRegeneration {
            active: 3,
            max: 2,
        };
        assert!(reason.to_string().contains("concurrent"));

        let reason = DeferralReason::SystemUnderLoad {
            current_load: 0.95,
            max_load: 0.8,
        };
        assert!(reason.to_string().contains("0.95"));
    }

    #[test]
    fn from_config_applies_settings() {
        let config = IntentConfig {
            max_concurrent_intents: 5,
            post_modification_cooldown_secs: 7200,
            max_system_load: 0.9,
            ..IntentConfig::default()
        };
        let mgr = DeferralManager::from_config(&config);
        assert_eq!(mgr.max_concurrent_intents, 5);
        assert_eq!(mgr.post_modification_cooldown_secs, 7200);
        assert!((mgr.max_system_load - 0.9).abs() < f64::EPSILON);
    }
}
