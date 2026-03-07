//! Routing policy types for the MAPLE model router.
//!
//! Defines the policy structures that control how inference requests are
//! routed to backends: model preferences, data classification rules,
//! cost budgets, and fallback behavior.

use serde::{Deserialize, Serialize};

/// Routing policy that determines which backend to use for inference requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicy {
    /// Model preference order (candidates are sorted by priority, lowest first).
    pub model_preferences: Vec<ModelPreference>,
    /// Data classification to backend constraints mapping.
    pub classification_rules: Vec<ClassificationRule>,
    /// Optional cost budget constraints.
    pub cost_budget: Option<CostBudget>,
    /// Fallback behavior when preferred backends are unavailable.
    pub fallback: FallbackPolicy,
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self {
            model_preferences: Vec::new(),
            classification_rules: Vec::new(),
            cost_budget: None,
            fallback: FallbackPolicy::Fail,
        }
    }
}

/// A model preference entry mapping a model name pattern to a preferred backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreference {
    /// Model name pattern (e.g., "llama3*", "gpt-4o*", "claude*", "*").
    pub pattern: String,
    /// Preferred backend identifier.
    pub backend: String,
    /// Priority value (lower number = higher priority).
    pub priority: u32,
}

/// A rule mapping data classification levels to allowed backends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationRule {
    /// Data classification level (e.g., "public", "internal", "confidential", "regulated").
    pub classification: String,
    /// Backend identifiers permitted for this classification level.
    pub allowed_backends: Vec<String>,
    /// Capabilities the backend must support for this classification level.
    pub required_capabilities: Vec<String>,
}

/// Budget constraints for inference cost control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBudget {
    /// Maximum cost per 1K input tokens.
    pub max_input_cost_per_1k: Option<f64>,
    /// Maximum cost per 1K output tokens.
    pub max_output_cost_per_1k: Option<f64>,
    /// Maximum total cost per single request.
    pub max_per_request: Option<f64>,
    /// Maximum daily spend across all requests.
    pub max_daily_spend: Option<f64>,
}

/// Defines fallback behavior when the preferred backend is unavailable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FallbackPolicy {
    /// Fail immediately if the primary backend is unavailable.
    Fail,
    /// Try the next backend in preference order.
    NextPreference,
    /// Try the cheapest available backend.
    CheapestAvailable,
    /// Try the fastest available backend.
    FastestAvailable,
}

/// Matches a model name against a pattern.
///
/// Supports:
/// - Exact match: `"gpt-4o"` matches only `"gpt-4o"`
/// - Prefix wildcard: `"llama3*"` matches `"llama3.2:8b-q4"`, `"llama3-70b"`
/// - Universal wildcard: `"*"` matches everything
pub fn model_matches_pattern(model: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        model.starts_with(prefix)
    } else {
        model == pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_exact_match() {
        assert!(model_matches_pattern("gpt-4o", "gpt-4o"));
        assert!(!model_matches_pattern("gpt-4o-mini", "gpt-4o"));
    }

    #[test]
    fn test_pattern_prefix_wildcard() {
        assert!(model_matches_pattern("llama3.2:8b-q4", "llama3*"));
        assert!(model_matches_pattern("llama3-70b", "llama3*"));
        assert!(!model_matches_pattern("mistral-7b", "llama3*"));
    }

    #[test]
    fn test_pattern_universal_wildcard() {
        assert!(model_matches_pattern("anything", "*"));
        assert!(model_matches_pattern("", "*"));
    }

    #[test]
    fn test_routing_policy_serde_roundtrip() {
        let policy = RoutingPolicy {
            model_preferences: vec![ModelPreference {
                pattern: "llama3*".to_string(),
                backend: "ollama-local".to_string(),
                priority: 1,
            }],
            classification_rules: vec![ClassificationRule {
                classification: "confidential".to_string(),
                allowed_backends: vec!["ollama-local".to_string()],
                required_capabilities: vec!["chat".to_string()],
            }],
            cost_budget: Some(CostBudget {
                max_input_cost_per_1k: Some(0.01),
                max_output_cost_per_1k: Some(0.03),
                max_per_request: Some(1.0),
                max_daily_spend: Some(100.0),
            }),
            fallback: FallbackPolicy::NextPreference,
        };

        let json = serde_json::to_string(&policy).expect("serialize");
        let deserialized: RoutingPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.model_preferences.len(), 1);
        assert_eq!(deserialized.fallback, FallbackPolicy::NextPreference);
    }
}
