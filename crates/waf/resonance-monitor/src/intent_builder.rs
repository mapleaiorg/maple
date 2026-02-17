use crate::types::{DissonanceCategory, DissonanceEvent};
use maple_waf_context_graph::{GovernanceTier, IntentNode};
use worldline_types::EventId;

/// Converts dissonance events into intent nodes for the context graph.
pub struct IntentBuilder;

impl IntentBuilder {
    /// Convert a dissonance event into an IntentNode.
    pub fn from_dissonance(event: &DissonanceEvent) -> IntentNode {
        let governance_tier = Self::tier_for_category(event.category);
        let mut intent = IntentNode::new(
            EventId::new(),
            format!("[{}] {}", event.category, event.description),
            governance_tier,
        );
        intent = intent.with_metric(
            event.source_metric.clone(),
            event.threshold - event.current_value,
        );
        intent = intent.with_priority(Self::priority_for_severity(event.severity));
        intent
    }

    /// Map dissonance category to governance tier.
    fn tier_for_category(category: DissonanceCategory) -> GovernanceTier {
        match category {
            DissonanceCategory::Computational => GovernanceTier::Tier0,
            DissonanceCategory::Semantic => GovernanceTier::Tier1,
            DissonanceCategory::PolicyDrift => GovernanceTier::Tier2,
        }
    }

    /// Map severity to priority (lower number = higher priority).
    fn priority_for_severity(severity: f64) -> u32 {
        if severity >= 0.9 {
            0
        } else if severity >= 0.7 {
            1
        } else if severity >= 0.5 {
            2
        } else if severity >= 0.3 {
            3
        } else {
            5
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_computational_dissonance() {
        let event = DissonanceEvent::new(
            DissonanceCategory::Computational,
            0.8,
            "high CPU",
            "cpu_pct",
            95.0,
            80.0,
        );
        let intent = IntentBuilder::from_dissonance(&event);
        assert!(intent.description.contains("Computational"));
        assert_eq!(intent.governance_tier, GovernanceTier::Tier0);
        assert_eq!(intent.priority, 1); // severity 0.8 -> priority 1
    }

    #[test]
    fn from_semantic_dissonance() {
        let event = DissonanceEvent::new(
            DissonanceCategory::Semantic,
            0.5,
            "API friction",
            "friction",
            0.5,
            0.3,
        );
        let intent = IntentBuilder::from_dissonance(&event);
        assert_eq!(intent.governance_tier, GovernanceTier::Tier1);
        assert_eq!(intent.priority, 2);
    }

    #[test]
    fn from_policy_drift() {
        let event = DissonanceEvent::new(
            DissonanceCategory::PolicyDrift,
            0.95,
            "denial spike",
            "denials",
            0.5,
            0.1,
        );
        let intent = IntentBuilder::from_dissonance(&event);
        assert_eq!(intent.governance_tier, GovernanceTier::Tier2);
        assert_eq!(intent.priority, 0); // severity 0.95 -> priority 0
    }

    #[test]
    fn low_severity_low_priority() {
        let event = DissonanceEvent::new(
            DissonanceCategory::Computational,
            0.1,
            "minor",
            "m",
            81.0,
            80.0,
        );
        let intent = IntentBuilder::from_dissonance(&event);
        assert_eq!(intent.priority, 5);
    }

    #[test]
    fn target_metric_computed() {
        let event = DissonanceEvent::new(
            DissonanceCategory::Computational,
            0.5,
            "test",
            "cpu_pct",
            90.0,
            80.0,
        );
        let intent = IntentBuilder::from_dissonance(&event);
        let target = intent.target_metrics.get("cpu_pct").unwrap();
        assert_eq!(*target, -10.0); // threshold - current = 80 - 90 = -10
    }
}
