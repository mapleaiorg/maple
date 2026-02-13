//! Ambiguity manager — preserves competing interpretations.
//!
//! CRITICAL: WorldLine preserves ambiguity. Multiple competing interpretations
//! coexist until evidence resolves them or action requires a decision. Premature
//! disambiguation is a failure mode per Resonance Architecture §5.4.

use serde::{Deserialize, Serialize};

use crate::types::{MeaningConfig, MeaningId, SelfMeaning};

// ── Ambiguity Decision ──────────────────────────────────────────────────

/// Decision about how to handle ambiguity for a meaning.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AmbiguityDecision {
    /// Keep all competing interpretations alive.
    Preserve {
        /// Why we're preserving ambiguity.
        reason: String,
        /// How long (seconds) before reviewing again.
        review_after_secs: u64,
    },

    /// Evidence strongly favors one interpretation — ready for intent.
    ReadyForIntent {
        /// The winning meaning's ID.
        winning_meaning_id: MeaningId,
        /// Confidence level of the winning meaning.
        confidence: f64,
    },

    /// High-ambiguity safety concern — escalate to governance.
    Escalated {
        /// IDs of the competing meanings.
        meaning_ids: Vec<MeaningId>,
        /// Description of the safety concern.
        safety_concern: String,
    },

    /// Need more evidence before deciding.
    GatherMore {
        /// Descriptions of needed evidence.
        needed_descriptions: Vec<String>,
        /// Timeout in seconds before forcing a decision.
        timeout_secs: u64,
    },
}

/// A request for specific evidence that would help resolve ambiguity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvidenceRequest {
    /// What evidence is needed.
    pub description: String,
    /// Priority (higher = more urgent).
    pub priority: f64,
}

// ── Ambiguity Manager ───────────────────────────────────────────────────

/// Manages ambiguity in meaning formation.
///
/// Determines whether to preserve competing interpretations, declare one
/// ready for intent formation, escalate safety concerns, or request more evidence.
pub struct AmbiguityManager {
    /// Below this ambiguity level, meaning is considered "resolved".
    pub resolution_threshold: f64,
    /// Safety-relevant meanings have a stricter threshold.
    pub safety_resolution_threshold: f64,
    /// Minimum time (seconds) competing meanings must coexist before resolution.
    pub min_coexistence_secs: u64,
}

impl Default for AmbiguityManager {
    fn default() -> Self {
        Self {
            resolution_threshold: 0.2,
            safety_resolution_threshold: 0.1,
            min_coexistence_secs: 3600,
        }
    }
}

impl AmbiguityManager {
    /// Create an ambiguity manager from a meaning configuration.
    pub fn from_config(config: &MeaningConfig) -> Self {
        Self {
            resolution_threshold: config.resolution_threshold,
            safety_resolution_threshold: config.safety_resolution_threshold,
            min_coexistence_secs: config.min_observation_secs,
        }
    }

    /// Evaluate all active meanings and produce ambiguity decisions.
    ///
    /// Returns a decision for each meaning that requires action.
    pub fn evaluate(
        &self,
        meanings: &[SelfMeaning],
        config: &MeaningConfig,
    ) -> Vec<(MeaningId, AmbiguityDecision)> {
        let mut decisions = Vec::new();

        for meaning in meanings {
            let decision = self.evaluate_single(meaning, meanings, config);
            decisions.push((meaning.id.clone(), decision));
        }

        decisions
    }

    /// Evaluate a single meaning in the context of all active meanings.
    fn evaluate_single(
        &self,
        meaning: &SelfMeaning,
        all_meanings: &[SelfMeaning],
        config: &MeaningConfig,
    ) -> AmbiguityDecision {
        let threshold = if meaning.is_safety_relevant() {
            self.safety_resolution_threshold
        } else {
            self.resolution_threshold
        };

        // Check: safety-relevant with high ambiguity → escalate
        if meaning.is_safety_relevant() && meaning.ambiguity > 0.5 {
            let competing_ids: Vec<MeaningId> = meaning
                .competing_with
                .iter()
                .cloned()
                .chain(std::iter::once(meaning.id.clone()))
                .collect();
            return AmbiguityDecision::Escalated {
                meaning_ids: competing_ids,
                safety_concern: format!(
                    "Safety-relevant meaning '{}' has high ambiguity ({:.2})",
                    meaning.category, meaning.ambiguity
                ),
            };
        }

        // Check: insufficient evidence → gather more
        if !meaning.has_sufficient_evidence(config.min_evidence_count) {
            return AmbiguityDecision::GatherMore {
                needed_descriptions: vec![format!(
                    "Need {} more evidence items for '{}'",
                    config.min_evidence_count - meaning.evidence.len(),
                    meaning.category
                )],
                timeout_secs: config.min_observation_secs,
            };
        }

        // Check: converged and high confidence → ready for intent
        if meaning.converged
            && meaning.confidence > (1.0 - threshold)
            && meaning.competing_with.is_empty()
        {
            return AmbiguityDecision::ReadyForIntent {
                winning_meaning_id: meaning.id.clone(),
                confidence: meaning.confidence,
            };
        }

        // Check: converged with competitors → check if we clearly dominate
        if meaning.converged && !meaning.competing_with.is_empty() {
            let competitors: Vec<&SelfMeaning> = all_meanings
                .iter()
                .filter(|m| meaning.competing_with.contains(&m.id))
                .collect();

            let all_weaker = competitors
                .iter()
                .all(|c| c.confidence < meaning.confidence - 0.2);

            if all_weaker && meaning.confidence > (1.0 - threshold) {
                return AmbiguityDecision::ReadyForIntent {
                    winning_meaning_id: meaning.id.clone(),
                    confidence: meaning.confidence,
                };
            }
        }

        // Default: preserve ambiguity
        AmbiguityDecision::Preserve {
            reason: format!(
                "Meaning '{}' still forming (confidence={:.2}, ambiguity={:.2}, competing={})",
                meaning.category,
                meaning.confidence,
                meaning.ambiguity,
                meaning.competing_with.len()
            ),
            review_after_secs: 300,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Evidence, EvidenceCategory, GrowthModel, SelfMeaningCategory,
    };
    use chrono::Utc;

    fn make_evidence(n: usize) -> Vec<Evidence> {
        (0..n)
            .map(|i| Evidence {
                source: format!("source-{}", i),
                strength: 0.7,
                timestamp: Utc::now(),
                description: format!("evidence {}", i),
                category: EvidenceCategory::Anomaly,
            })
            .collect()
    }

    fn make_meaning(
        category: SelfMeaningCategory,
        confidence: f64,
        ambiguity: f64,
        evidence_count: usize,
        converged: bool,
    ) -> SelfMeaning {
        SelfMeaning {
            id: MeaningId::new(),
            category,
            evidence: make_evidence(evidence_count),
            confidence,
            ambiguity,
            formed_at: Utc::now(),
            temporal_stability_secs: 7200.0,
            competing_with: vec![],
            converged,
        }
    }

    #[test]
    fn ready_for_intent_when_converged_high_confidence() {
        let mgr = AmbiguityManager::default();
        let config = MeaningConfig::default();

        let meaning = make_meaning(
            SelfMeaningCategory::PerformanceBottleneck {
                component: "gate".into(),
                severity: 0.8,
                root_causes: vec![],
            },
            0.9,  // high confidence
            0.1,  // low ambiguity
            15,   // enough evidence
            true, // converged
        );

        let decisions = mgr.evaluate(&[meaning], &config);
        assert_eq!(decisions.len(), 1);
        assert!(matches!(
            decisions[0].1,
            AmbiguityDecision::ReadyForIntent { .. }
        ));
    }

    #[test]
    fn gather_more_when_insufficient_evidence() {
        let mgr = AmbiguityManager::default();
        let config = MeaningConfig::default();

        let meaning = make_meaning(
            SelfMeaningCategory::PerformanceBottleneck {
                component: "gate".into(),
                severity: 0.8,
                root_causes: vec![],
            },
            0.9,
            0.1,
            3,     // not enough evidence (min is 10)
            false,
        );

        let decisions = mgr.evaluate(&[meaning], &config);
        assert_eq!(decisions.len(), 1);
        assert!(matches!(
            decisions[0].1,
            AmbiguityDecision::GatherMore { .. }
        ));
    }

    #[test]
    fn escalate_safety_relevant_high_ambiguity() {
        let mgr = AmbiguityManager::default();
        let config = MeaningConfig::default();

        let meaning = make_meaning(
            SelfMeaningCategory::CapacityForecast {
                resource: "memory".into(),
                current_utilization: 0.95,
                projected_exhaustion_hours: Some(2.0),
                growth_model: GrowthModel::Exponential,
            },
            0.6,
            0.7,  // high ambiguity
            15,
            false,
        );

        let decisions = mgr.evaluate(&[meaning], &config);
        assert_eq!(decisions.len(), 1);
        assert!(matches!(
            decisions[0].1,
            AmbiguityDecision::Escalated { .. }
        ));
    }

    #[test]
    fn preserve_when_competing_meanings() {
        let mgr = AmbiguityManager::default();
        let config = MeaningConfig::default();

        let competitor_id = MeaningId::new();
        let mut meaning = make_meaning(
            SelfMeaningCategory::PerformanceBottleneck {
                component: "gate".into(),
                severity: 0.7,
                root_causes: vec![],
            },
            0.7,
            0.3,
            15,
            true,
        );
        meaning.competing_with = vec![competitor_id.clone()];

        let competitor = SelfMeaning {
            id: competitor_id,
            category: SelfMeaningCategory::ArchitecturalInsight {
                insight_type: crate::types::ArchitecturalInsightType::CouplingTooTight,
                affected_components: vec!["gate".into(), "fabric".into()],
                structural_pressure: "test".into(),
            },
            evidence: make_evidence(15),
            confidence: 0.6,
            ambiguity: 0.4,
            formed_at: Utc::now(),
            temporal_stability_secs: 3600.0,
            competing_with: vec![meaning.id.clone()],
            converged: true,
        };

        let decisions = mgr.evaluate(&[meaning, competitor], &config);
        // Both should be "Preserve" since neither clearly dominates
        assert_eq!(decisions.len(), 2);
        assert!(matches!(decisions[0].1, AmbiguityDecision::Preserve { .. }));
    }

    #[test]
    fn ready_for_intent_when_clearly_dominating() {
        let mgr = AmbiguityManager::default();
        let config = MeaningConfig::default();

        let competitor_id = MeaningId::new();
        let mut winner = make_meaning(
            SelfMeaningCategory::PerformanceBottleneck {
                component: "gate".into(),
                severity: 0.8,
                root_causes: vec![],
            },
            0.9,
            0.1,
            15,
            true,
        );
        winner.competing_with = vec![competitor_id.clone()];

        let loser = SelfMeaning {
            id: competitor_id,
            category: SelfMeaningCategory::ArchitecturalInsight {
                insight_type: crate::types::ArchitecturalInsightType::CouplingTooTight,
                affected_components: vec![],
                structural_pressure: "test".into(),
            },
            evidence: make_evidence(15),
            confidence: 0.3,  // much lower than winner
            ambiguity: 0.6,
            formed_at: Utc::now(),
            temporal_stability_secs: 3600.0,
            competing_with: vec![winner.id.clone()],
            converged: false,
        };

        let decisions = mgr.evaluate(&[winner, loser], &config);
        assert!(matches!(
            decisions[0].1,
            AmbiguityDecision::ReadyForIntent { .. }
        ));
    }

    #[test]
    fn non_safety_uses_standard_threshold() {
        let mgr = AmbiguityManager::default();
        let config = MeaningConfig::default();

        // Non-safety with moderate ambiguity — should NOT escalate
        let meaning = make_meaning(
            SelfMeaningCategory::ApiDesignInsight {
                pattern: "test".into(),
                improvement_direction: "test".into(),
            },
            0.6,
            0.6,  // high ambiguity, but not safety-relevant
            15,
            false,
        );

        let decisions = mgr.evaluate(&[meaning], &config);
        assert_eq!(decisions.len(), 1);
        assert!(
            !matches!(decisions[0].1, AmbiguityDecision::Escalated { .. }),
            "Non-safety meanings should not be escalated"
        );
    }

    #[test]
    fn from_config_matches_config_values() {
        let config = MeaningConfig {
            resolution_threshold: 0.15,
            safety_resolution_threshold: 0.05,
            min_observation_secs: 7200,
            ..MeaningConfig::default()
        };
        let mgr = AmbiguityManager::from_config(&config);
        assert!((mgr.resolution_threshold - 0.15).abs() < f64::EPSILON);
        assert!((mgr.safety_resolution_threshold - 0.05).abs() < f64::EPSILON);
        assert_eq!(mgr.min_coexistence_secs, 7200);
    }

    #[test]
    fn ambiguity_decision_serialization() {
        let decision = AmbiguityDecision::ReadyForIntent {
            winning_meaning_id: MeaningId::new(),
            confidence: 0.85,
        };
        let json = serde_json::to_string(&decision).unwrap();
        let restored: AmbiguityDecision = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            restored,
            AmbiguityDecision::ReadyForIntent { .. }
        ));
    }
}
