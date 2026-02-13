//! Meaning-to-Intent bridge — connects meaning formation to intent stabilization.
//!
//! When meanings converge with sufficient confidence, they become candidates
//! for intent formation (Prompt 14). The bridge trait provides the interface
//! for querying which meanings are ready, still forming, or abandoned.

use crate::types::SelfMeaning;

/// Bridge between meaning formation and intent stabilization.
///
/// Implementations classify active meanings into three categories:
/// - **Ready for intent**: Converged, high confidence, unambiguous
/// - **Still forming**: Active but not yet stable enough
/// - **Abandoned**: Evidence collapsed, no longer viable
pub trait MeaningIntentBridge {
    /// Get meanings that have converged and are ready for intent formation.
    fn ready_for_intent(&self) -> Vec<&SelfMeaning>;

    /// Get meanings that are still forming (not ready yet).
    fn still_forming(&self) -> Vec<&SelfMeaning>;

    /// Get meanings that were abandoned (evidence collapsed).
    fn abandoned(&self) -> Vec<&SelfMeaning>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal bridge implementation for testing
    struct TestBridge {
        meanings: Vec<SelfMeaning>,
        abandoned_meanings: Vec<SelfMeaning>,
    }

    impl MeaningIntentBridge for TestBridge {
        fn ready_for_intent(&self) -> Vec<&SelfMeaning> {
            self.meanings
                .iter()
                .filter(|m| m.converged && m.confidence > 0.8)
                .collect()
        }

        fn still_forming(&self) -> Vec<&SelfMeaning> {
            self.meanings
                .iter()
                .filter(|m| !m.converged || m.confidence <= 0.8)
                .collect()
        }

        fn abandoned(&self) -> Vec<&SelfMeaning> {
            self.abandoned_meanings.iter().collect()
        }
    }

    #[test]
    fn bridge_trait_is_object_safe() {
        // Verify trait can be used as trait object
        fn _takes_bridge(_bridge: &dyn MeaningIntentBridge) {}
    }

    #[test]
    fn bridge_classifies_meanings() {
        use crate::types::{MeaningId, SelfMeaningCategory};
        use chrono::Utc;

        let bridge = TestBridge {
            meanings: vec![
                SelfMeaning {
                    id: MeaningId::new(),
                    category: SelfMeaningCategory::PerformanceBottleneck {
                        component: "gate".into(),
                        severity: 0.8,
                        root_causes: vec![],
                    },
                    evidence: vec![],
                    confidence: 0.9,
                    ambiguity: 0.1,
                    formed_at: Utc::now(),
                    temporal_stability_secs: 7200.0,
                    competing_with: vec![],
                    converged: true,
                },
                SelfMeaning {
                    id: MeaningId::new(),
                    category: SelfMeaningCategory::ApiDesignInsight {
                        pattern: "test".into(),
                        improvement_direction: "test".into(),
                    },
                    evidence: vec![],
                    confidence: 0.4,
                    ambiguity: 0.6,
                    formed_at: Utc::now(),
                    temporal_stability_secs: 300.0,
                    competing_with: vec![],
                    converged: false,
                },
            ],
            abandoned_meanings: vec![],
        };

        assert_eq!(bridge.ready_for_intent().len(), 1);
        assert_eq!(bridge.still_forming().len(), 1);
        assert_eq!(bridge.abandoned().len(), 0);
    }
}
