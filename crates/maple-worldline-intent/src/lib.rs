//! # maple-worldline-intent
//!
//! Self-Intent Stabilization: transforms converged self-meanings into
//! validated, prioritized self-modification proposals.
//!
//! ## Architecture
//!
//! ```text
//! maple-worldline-meaning             maple-worldline-intent
//! ┌──────────────────────┐            ┌────────────────────────────────────┐
//! │  SelfMeaningEngine   │            │  IntentStabilizationEngine         │
//! │  ┌────────────────┐  │            │  ┌──────────────────────────────┐  │
//! │  │ converged      │──┼──bridge──▶ │  │ IntentGenerator (×4)        │  │
//! │  │ meanings       │  │            │  │  ├─ PerformanceIntentGen     │  │
//! │  └────────────────┘  │            │  │  ├─ CapacityIntentGen        │  │
//! └──────────────────────┘            │  │  ├─ CodeQualityIntentGen     │  │
//!                                     │  │  └─ ArchitectureIntentGen    │  │
//!                                     │  └──────────┬───────────────────┘  │
//!                                     │             ▼                      │
//!                                     │  ┌──────────────────────────────┐  │
//!                                     │  │ IntentValidator              │  │
//!                                     │  │  (confidence, risk, safety)  │  │
//!                                     │  └──────────┬───────────────────┘  │
//!                                     │             ▼                      │
//!                                     │  ┌──────────────────────────────┐  │
//!                                     │  │ IntentPrioritizer            │  │
//!                                     │  │  (weighted composite score)  │  │
//!                                     │  └──────────┬───────────────────┘  │
//!                                     │             ▼                      │
//!                                     │  ┌──────────────────────────────┐  │
//!                                     │  │ DeferralManager              │  │
//!                                     │  │  (load, cooldown, concur.)   │  │
//!                                     │  └──────────┬───────────────────┘  │
//!                                     │             ▼                      │
//!                                     │  stabilized intents → commitment  │
//!                                     └────────────────────────────────────┘
//! ```
//!
//! ## Governance Tiers
//!
//! | Tier | Scope          | Observation | Confidence |
//! |------|----------------|-------------|------------|
//! | 0    | Configuration  | 30 min      | 0.70       |
//! | 1    | Operator       | 1 hour      | 0.80       |
//! | 2    | Kernel module  | 24 hours    | 0.85       |
//! | 3    | Architecture   | 72 hours    | 0.90       |

#![deny(unsafe_code)]

pub mod deferral;
pub mod engine;
pub mod error;
pub mod generator;
pub mod intent;
pub mod prioritizer;
pub mod proposal;
pub mod types;
pub mod validator;

// ── Re-exports ─────────────────────────────────────────────────────────

pub use engine::IntentStabilizationEngine;
pub use error::{IntentError, IntentResult};
pub use intent::{ImpactAssessment, ImprovementEstimate, IntentStatus, SelfRegenerationIntent};
pub use proposal::RegenerationProposal;
pub use types::{
    ChangeType, CodeChangeType, IntentConfig, IntentId, ProposalId, ReversibilityLevel,
    SubstrateTier,
};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use maple_worldline_meaning::types::{
        Evidence, EvidenceCategory, MeaningId, SelfMeaning, SelfMeaningCategory,
    };

    fn make_evidence(n: usize) -> Vec<Evidence> {
        (0..n)
            .map(|i| Evidence {
                source: format!("src-{}", i),
                strength: 0.7,
                timestamp: Utc::now(),
                description: format!("evidence {}", i),
                category: EvidenceCategory::Anomaly,
            })
            .collect()
    }

    #[test]
    fn integration_full_pipeline() {
        let mut engine = IntentStabilizationEngine::with_defaults();

        let meanings = vec![
            SelfMeaning {
                id: MeaningId::new(),
                category: SelfMeaningCategory::PerformanceBottleneck {
                    component: "gate".into(),
                    severity: 0.8,
                    root_causes: vec![],
                },
                evidence: make_evidence(15),
                confidence: 0.9,
                ambiguity: 0.1,
                formed_at: Utc::now(),
                temporal_stability_secs: 7200.0,
                competing_with: vec![],
                converged: true,
            },
            SelfMeaning {
                id: MeaningId::new(),
                category: SelfMeaningCategory::CapacityForecast {
                    resource: "memory".into(),
                    current_utilization: 0.85,
                    projected_exhaustion_hours: Some(12.0),
                    growth_model: maple_worldline_meaning::types::GrowthModel::Linear,
                },
                evidence: make_evidence(12),
                confidence: 0.85,
                ambiguity: 0.15,
                formed_at: Utc::now(),
                temporal_stability_secs: 3600.0,
                competing_with: vec![],
                converged: true,
            },
        ];

        let stabilized = engine
            .process_meaning_list(&meanings, 0.4)
            .expect("pipeline should succeed");

        // At least one intent should be produced
        assert!(!stabilized.is_empty(), "should produce at least one intent");

        // Active intents should be sorted by priority
        let active = engine.active_intents();
        assert!(!active.is_empty());

        // All active intents should be stabilized
        for intent in active {
            assert!(
                matches!(intent.status, IntentStatus::Stabilized),
                "active intent should be stabilized"
            );
        }
    }

    #[test]
    fn integration_deferral_and_promotion() {
        let mut engine = IntentStabilizationEngine::with_defaults();

        let meaning = SelfMeaning {
            id: MeaningId::new(),
            category: SelfMeaningCategory::PerformanceBottleneck {
                component: "fabric".into(),
                severity: 0.6,
                root_causes: vec![],
            },
            evidence: make_evidence(10),
            confidence: 0.85,
            ambiguity: 0.1,
            formed_at: Utc::now(),
            temporal_stability_secs: 5400.0,
            competing_with: vec![],
            converged: true,
        };

        // Process under high load → defer
        let stabilized = engine
            .process_meaning_list(&[meaning], 0.95)
            .expect("should succeed");
        assert!(stabilized.is_empty());
        assert_eq!(engine.deferred_intents().len(), 1);

        // Re-evaluate under low load → promote
        let promoted = engine.reevaluate_deferred(0.3);
        assert!(!promoted.is_empty());
        assert!(engine.deferred_intents().is_empty());
        assert!(!engine.active_intents().is_empty());
    }

    #[test]
    fn integration_architectural_change_high_tier() {
        let mut engine = IntentStabilizationEngine::new(IntentConfig {
            max_risk: 0.6, // allow higher risk for arch changes
            ..IntentConfig::default()
        });

        let meaning = SelfMeaning {
            id: MeaningId::new(),
            category: SelfMeaningCategory::ArchitecturalInsight {
                insight_type: maple_worldline_meaning::types::ArchitecturalInsightType::CouplingTooTight,
                affected_components: vec!["gate".into(), "fabric".into()],
                structural_pressure: "high coupling".into(),
            },
            evidence: make_evidence(20),
            confidence: 0.92,
            ambiguity: 0.05,
            formed_at: Utc::now(),
            temporal_stability_secs: 86400.0,
            competing_with: vec![],
            converged: true,
        };

        let stabilized = engine
            .process_meaning_list(&[meaning], 0.3)
            .expect("should succeed");

        assert!(!stabilized.is_empty());
        let intent = &stabilized[0];
        assert_eq!(intent.governance_tier, SubstrateTier::Tier3);
        assert_eq!(intent.required_observation_secs(), 259200); // 72h
    }
}
