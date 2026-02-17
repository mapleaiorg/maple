//! Prompt contracts and tool schemas for WorldLine operator agents.

use serde::{Deserialize, Serialize};
use serde_json::json;
use worldline_operator_bot::GovernanceRunbook;

/// Tool contract that operator agents can call.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolContract {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Prompt pack that defines one operator interaction contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PromptPack {
    pub name: String,
    pub version: String,
    pub system_prompt: String,
    pub runbook_steps: Vec<String>,
    pub tools: Vec<ToolContract>,
}

/// Canonical operator system prompt for governance loops.
pub fn operator_system_prompt() -> &'static str {
    "You are the WorldLine operator bot. Observe metrics and ledger evidence, propose commitments with explicit rationale, and never execute irreversible effects without commitment receipts. No commitment means no consequence."
}

/// Canonical prompt contract for incident + policy runbooks.
pub fn operator_prompt_pack() -> PromptPack {
    PromptPack {
        name: "worldline-operator-governance".into(),
        version: "2026-02-17".into(),
        system_prompt: operator_system_prompt().into(),
        runbook_steps: vec![
            "Collect kernel metrics and latest audit index entries".into(),
            "Classify risk and choose runbook (incident response or policy review)".into(),
            "Draft commitment proposal with targets, capabilities, and evidence references".into(),
            "Submit commitment proposal to governance API and wait for decision receipt".into(),
            "Record outcome and produce audit summary".into(),
        ],
        tools: vec![
            ToolContract {
                name: "worldline_governance.list_policies".into(),
                description: "Read active governance policies".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
            },
            ToolContract {
                name: "worldline_governance.simulate_policy".into(),
                description: "Simulate policy evaluation for a proposed commitment".into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["effect_domain"],
                    "properties": {
                        "effect_domain": { "type": "string" },
                        "capabilities": { "type": "array", "items": { "type": "string" } },
                        "targets": { "type": "array", "items": { "type": "string" } }
                    },
                    "additionalProperties": true
                }),
            },
            ToolContract {
                name: "worldline_commitment.submit".into(),
                description: "Submit commitment declarations".into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["declaring_identity", "effect_domain", "targets", "capabilities"],
                    "properties": {
                        "declaring_identity": { "type": "string" },
                        "effect_domain": { "type": "string" },
                        "targets": { "type": "array", "items": { "type": "string" } },
                        "capabilities": { "type": "array", "items": { "type": "string" } },
                        "evidence": { "type": "array", "items": { "type": "string" } }
                    },
                    "additionalProperties": false
                }),
            },
        ],
    }
}

/// Runbooks referenced by the canonical operator prompt contract.
pub fn operator_runbook_catalog() -> Vec<GovernanceRunbook> {
    vec![
        GovernanceRunbook::IncidentResponse,
        GovernanceRunbook::PolicyReview,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_prompt_mentions_commitment_boundary() {
        assert!(operator_system_prompt().contains("No commitment means no consequence"));
    }

    #[test]
    fn operator_pack_contains_three_tools() {
        let pack = operator_prompt_pack();
        assert_eq!(pack.tools.len(), 3);
        assert!(pack
            .tools
            .iter()
            .any(|tool| tool.name == "worldline_governance.simulate_policy"));
    }

    #[test]
    fn promptkit_catalog_matches_operator_runbooks() {
        let catalog = operator_runbook_catalog();
        assert_eq!(catalog.len(), 2);
        assert!(catalog.contains(&GovernanceRunbook::IncidentResponse));
        assert!(catalog.contains(&GovernanceRunbook::PolicyReview));
    }
}
