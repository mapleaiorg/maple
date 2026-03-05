//! Golden test traces for skill pack conformance testing.
//!
//! Golden traces define expected input/output pairs that a skill must satisfy.
//! They serve as conformance tests ensuring the skill behaves as documented.

use serde::{Deserialize, Serialize};

/// A golden test trace — an expected input/output pair with constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenTrace {
    /// Human-readable name for this test case.
    pub name: String,
    /// Description of what this trace tests.
    #[serde(default)]
    pub description: String,
    /// Input JSON to pass to the skill.
    pub input: serde_json::Value,
    /// Expected output JSON (matched structurally).
    pub expected_output: serde_json::Value,
    /// Capabilities that should be required for this execution.
    #[serde(default)]
    pub expected_capabilities: Vec<String>,
    /// Expected resource budget consumed.
    pub expected_budget: Option<GoldenBudget>,
}

/// Expected resource consumption in a golden trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenBudget {
    /// Expected max compute time in ms.
    pub max_compute_ms: Option<u64>,
    /// Expected max memory in bytes.
    pub max_memory_bytes: Option<u64>,
}

impl GoldenTrace {
    /// Parse a golden trace from a JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, crate::SkillError> {
        serde_json::from_str(json_str)
            .map_err(|e| crate::SkillError::GoldenTrace(format!("JSON parse error: {e}")))
    }

    /// Parse multiple golden traces from a JSON array string.
    pub fn from_json_array(json_str: &str) -> Result<Vec<Self>, crate::SkillError> {
        serde_json::from_str(json_str)
            .map_err(|e| crate::SkillError::GoldenTrace(format!("JSON parse error: {e}")))
    }

    /// Check if an actual output structurally matches the expected output.
    ///
    /// Structural matching means:
    /// - All keys in `expected` must exist in `actual`
    /// - Values must match (recursively for objects/arrays)
    /// - Extra keys in `actual` are allowed (non-strict)
    pub fn matches_output(&self, actual: &serde_json::Value) -> bool {
        structural_match(&self.expected_output, actual)
    }
}

/// Recursively check if `expected` is structurally contained in `actual`.
fn structural_match(expected: &serde_json::Value, actual: &serde_json::Value) -> bool {
    match (expected, actual) {
        (serde_json::Value::Object(exp), serde_json::Value::Object(act)) => {
            exp.iter()
                .all(|(k, v)| act.get(k).is_some_and(|av| structural_match(v, av)))
        }
        (serde_json::Value::Array(exp), serde_json::Value::Array(act)) => {
            exp.len() == act.len()
                && exp
                    .iter()
                    .zip(act.iter())
                    .all(|(e, a)| structural_match(e, a))
        }
        _ => expected == actual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_trace_parse() {
        let json = r#"{
            "name": "basic-search",
            "description": "Simple search test",
            "input": {"query": "rust programming"},
            "expected_output": {"total": 5},
            "expected_capabilities": ["cap-web-search"]
        }"#;
        let trace = GoldenTrace::from_json(json).unwrap();
        assert_eq!(trace.name, "basic-search");
        assert_eq!(trace.expected_capabilities, vec!["cap-web-search"]);
    }

    #[test]
    fn golden_trace_array_parse() {
        let json = r#"[
            {"name": "test-1", "input": {}, "expected_output": {"ok": true}},
            {"name": "test-2", "input": {"x": 1}, "expected_output": {"ok": false}}
        ]"#;
        let traces = GoldenTrace::from_json_array(json).unwrap();
        assert_eq!(traces.len(), 2);
        assert_eq!(traces[0].name, "test-1");
        assert_eq!(traces[1].name, "test-2");
    }

    #[test]
    fn structural_match_exact() {
        let expected = serde_json::json!({"total": 5});
        let actual = serde_json::json!({"total": 5, "extra": "field"});
        assert!(structural_match(&expected, &actual));
    }

    #[test]
    fn structural_match_missing_key() {
        let expected = serde_json::json!({"total": 5, "missing": true});
        let actual = serde_json::json!({"total": 5});
        assert!(!structural_match(&expected, &actual));
    }

    #[test]
    fn structural_match_nested() {
        let expected = serde_json::json!({"data": {"count": 3}});
        let actual = serde_json::json!({"data": {"count": 3, "items": []}, "meta": {}});
        assert!(structural_match(&expected, &actual));
    }

    #[test]
    fn structural_match_array() {
        let expected = serde_json::json!([1, 2, 3]);
        let actual = serde_json::json!([1, 2, 3]);
        assert!(structural_match(&expected, &actual));
    }

    #[test]
    fn structural_match_array_length_mismatch() {
        let expected = serde_json::json!([1, 2]);
        let actual = serde_json::json!([1, 2, 3]);
        assert!(!structural_match(&expected, &actual));
    }

    #[test]
    fn matches_output_method() {
        let trace = GoldenTrace {
            name: "test".into(),
            description: String::new(),
            input: serde_json::json!({}),
            expected_output: serde_json::json!({"status": "ok"}),
            expected_capabilities: vec![],
            expected_budget: None,
        };
        assert!(trace.matches_output(&serde_json::json!({"status": "ok", "data": 42})));
        assert!(!trace.matches_output(&serde_json::json!({"status": "error"})));
    }
}
