use crate::types::GovernanceTier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use worldline_types::EventId;

/// Captures what we want: the problem, the goal, the target metrics.
/// Created by the Resonance Monitor when dissonance is detected.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentNode {
    /// ID of the dissonance event that triggered this intent.
    pub source_resonance_id: EventId,
    /// Human/agent-readable description of the goal.
    pub description: String,
    /// Target metrics to improve (e.g., {"latency_ms": -50.0, "memory_mb": -200.0}).
    pub target_metrics: HashMap<String, f64>,
    /// Priority level (0 = highest).
    pub priority: u32,
    /// Maximum time budget for this evolution step.
    pub time_budget_secs: u64,
    /// Governance tier required for this change.
    pub governance_tier: GovernanceTier,
}

impl IntentNode {
    pub fn new(
        source_resonance_id: EventId,
        description: impl Into<String>,
        governance_tier: GovernanceTier,
    ) -> Self {
        Self {
            source_resonance_id,
            description: description.into(),
            target_metrics: HashMap::new(),
            priority: 5,
            time_budget_secs: 300,
            governance_tier,
        }
    }

    pub fn with_metric(mut self, name: impl Into<String>, target: f64) -> Self {
        self.target_metrics.insert(name.into(), target);
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_time_budget(mut self, budget: Duration) -> Self {
        self.time_budget_secs = budget.as_secs();
        self
    }

    pub fn time_budget(&self) -> Duration {
        Duration::from_secs(self.time_budget_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_node_builder() {
        let intent = IntentNode::new(EventId::new(), "reduce latency", GovernanceTier::Tier1)
            .with_metric("latency_ms", -50.0)
            .with_metric("memory_mb", -200.0)
            .with_priority(2)
            .with_time_budget(Duration::from_secs(600));

        assert_eq!(intent.description, "reduce latency");
        assert_eq!(intent.priority, 2);
        assert_eq!(intent.time_budget_secs, 600);
        assert_eq!(intent.target_metrics.len(), 2);
        assert_eq!(intent.governance_tier, GovernanceTier::Tier1);
    }

    #[test]
    fn intent_node_serde_roundtrip() {
        let intent = IntentNode::new(EventId::new(), "test", GovernanceTier::Tier0);
        let json = serde_json::to_string(&intent).unwrap();
        let restored: IntentNode = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.description, "test");
    }

    #[test]
    fn intent_time_budget() {
        let intent = IntentNode::new(EventId::new(), "x", GovernanceTier::Tier0);
        assert_eq!(intent.time_budget(), Duration::from_secs(300));
    }
}
