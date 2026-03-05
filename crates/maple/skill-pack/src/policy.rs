//! Skill-specific policies, parsed from `policy.toml`.
//!
//! Policies define deny-first rules that constrain how the skill can be used.
//! These policies are registered with the governance engine when the skill is loaded.

use serde::{Deserialize, Serialize};

/// A skill-specific policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPolicy {
    /// Policy name (unique within a skill).
    pub name: String,
    /// Policy effect (allow or deny).
    pub effect: PolicyEffect,
    /// Condition that triggers this policy.
    pub condition: PolicyCondition,
    /// Human-readable reason for the policy.
    pub reason: String,
}

/// Policy effect: what happens when the condition matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyEffect {
    /// Allow the action (explicit allowance).
    Allow,
    /// Deny the action.
    Deny,
}

/// Condition that triggers a policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyCondition {
    /// Rate exceeds threshold.
    RateExceeds {
        /// Resource being rate-limited.
        resource: String,
        /// Maximum invocations per minute.
        max_per_minute: u32,
    },
    /// Input field validation.
    InputInvalid {
        /// Field name to validate.
        field: String,
        /// Maximum allowed length.
        #[serde(default)]
        max_length: Option<usize>,
        /// Regex pattern the field must match.
        #[serde(default)]
        pattern: Option<String>,
    },
    /// Resource budget exceeded.
    BudgetExceeds {
        /// Resource name.
        resource: String,
        /// Maximum allowed value.
        max_value: u64,
    },
    /// Time window restriction.
    TimeWindow {
        /// Start time (HH:MM format).
        start: String,
        /// End time (HH:MM format).
        end: String,
    },
}

/// Container for multiple policies (matches `policy.toml` structure).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPolicyFile {
    /// The policies defined in this file.
    #[serde(rename = "policies")]
    pub policies: Vec<SkillPolicy>,
}

impl SkillPolicyFile {
    /// Parse policies from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, crate::SkillError> {
        toml::from_str(toml_str)
            .map_err(|e| crate::SkillError::InvalidPolicy(format!("TOML parse error: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_POLICY_TOML: &str = r#"
[[policies]]
name = "rate-limit-search"
effect = "deny"
reason = "Web search rate limit exceeded"

[policies.condition]
type = "rate_exceeds"
resource = "web-search"
max_per_minute = 30

[[policies]]
name = "require-query-validation"
effect = "deny"
reason = "Query too long"

[policies.condition]
type = "input_invalid"
field = "query"
max_length = 1000
"#;

    #[test]
    fn parse_policy_toml() {
        let file = SkillPolicyFile::from_toml(SAMPLE_POLICY_TOML).unwrap();
        assert_eq!(file.policies.len(), 2);

        let p0 = &file.policies[0];
        assert_eq!(p0.name, "rate-limit-search");
        assert_eq!(p0.effect, PolicyEffect::Deny);
        match &p0.condition {
            PolicyCondition::RateExceeds {
                resource,
                max_per_minute,
            } => {
                assert_eq!(resource, "web-search");
                assert_eq!(*max_per_minute, 30);
            }
            _ => panic!("expected RateExceeds"),
        }

        let p1 = &file.policies[1];
        assert_eq!(p1.name, "require-query-validation");
        match &p1.condition {
            PolicyCondition::InputInvalid {
                field, max_length, ..
            } => {
                assert_eq!(field, "query");
                assert_eq!(*max_length, Some(1000));
            }
            _ => panic!("expected InputInvalid"),
        }
    }

    #[test]
    fn policy_effect_serde() {
        let allow: PolicyEffect = serde_json::from_str("\"allow\"").unwrap();
        assert_eq!(allow, PolicyEffect::Allow);
        let deny: PolicyEffect = serde_json::from_str("\"deny\"").unwrap();
        assert_eq!(deny, PolicyEffect::Deny);
    }

    #[test]
    fn budget_exceeds_condition() {
        let toml_str = r#"
[[policies]]
name = "budget-limit"
effect = "deny"
reason = "Budget exceeded"

[policies.condition]
type = "budget_exceeds"
resource = "llm-tokens"
max_value = 100000
"#;
        let file = SkillPolicyFile::from_toml(toml_str).unwrap();
        match &file.policies[0].condition {
            PolicyCondition::BudgetExceeds {
                resource,
                max_value,
            } => {
                assert_eq!(resource, "llm-tokens");
                assert_eq!(*max_value, 100_000);
            }
            _ => panic!("expected BudgetExceeds"),
        }
    }
}
