//! Validator: checks parsed workflows for semantic correctness
//!
//! Validation happens after parsing but before compilation.
//! It catches errors that are syntactically valid but semantically wrong.

use crate::errors::{DslError, DslResult};
use crate::parser::ParsedWorkflow;
use std::collections::HashSet;

/// Validate a parsed workflow for semantic correctness
pub fn validate(workflow: &ParsedWorkflow) -> DslResult<()> {
    validate_has_nodes(workflow)?;
    validate_has_start_node(workflow)?;
    validate_has_end_node(workflow)?;
    validate_unique_node_ids(workflow)?;
    validate_unique_role_ids(workflow)?;
    validate_edges_reference_valid_nodes(workflow)?;
    validate_node_types(workflow)?;
    validate_edge_gate_types(workflow)?;
    Ok(())
}

fn validate_has_nodes(workflow: &ParsedWorkflow) -> DslResult<()> {
    if workflow.nodes.is_empty() {
        return Err(DslError::ValidationError(
            "Workflow must have at least one node".into(),
        ));
    }
    Ok(())
}

fn validate_has_start_node(workflow: &ParsedWorkflow) -> DslResult<()> {
    let start_count = workflow
        .nodes
        .iter()
        .filter(|n| n.node_type == "start")
        .count();

    if start_count == 0 {
        return Err(DslError::ValidationError(
            "Workflow must have a start node (NODE ... TYPE start)".into(),
        ));
    }
    if start_count > 1 {
        return Err(DslError::ValidationError(
            "Workflow must have exactly one start node".into(),
        ));
    }
    Ok(())
}

fn validate_has_end_node(workflow: &ParsedWorkflow) -> DslResult<()> {
    let end_count = workflow
        .nodes
        .iter()
        .filter(|n| n.node_type == "end")
        .count();

    if end_count == 0 {
        return Err(DslError::ValidationError(
            "Workflow must have at least one end node (NODE ... TYPE end)".into(),
        ));
    }
    Ok(())
}

fn validate_unique_node_ids(workflow: &ParsedWorkflow) -> DslResult<()> {
    let mut seen = HashSet::new();
    for node in &workflow.nodes {
        if !seen.insert(&node.id) {
            return Err(DslError::DuplicateNodeId(node.id.clone()));
        }
    }
    Ok(())
}

fn validate_unique_role_ids(workflow: &ParsedWorkflow) -> DslResult<()> {
    let mut seen = HashSet::new();
    for role in &workflow.roles {
        if !seen.insert(&role.id) {
            return Err(DslError::DuplicateRoleId(role.id.clone()));
        }
    }
    Ok(())
}

fn validate_edges_reference_valid_nodes(workflow: &ParsedWorkflow) -> DslResult<()> {
    let node_ids: HashSet<&str> = workflow.nodes.iter().map(|n| n.id.as_str()).collect();

    for edge in &workflow.edges {
        if !node_ids.contains(edge.from.as_str()) {
            return Err(DslError::ValidationError(format!(
                "Edge references non-existent source node: '{}'",
                edge.from
            )));
        }
        if !node_ids.contains(edge.to.as_str()) {
            return Err(DslError::ValidationError(format!(
                "Edge references non-existent target node: '{}'",
                edge.to
            )));
        }
    }
    Ok(())
}

fn validate_node_types(workflow: &ParsedWorkflow) -> DslResult<()> {
    let valid_types = [
        "start",
        "end",
        "action",
        "decision",
        "fork",
        "join",
        "subworkflow",
    ];

    for node in &workflow.nodes {
        if !valid_types.contains(&node.node_type.as_str()) {
            return Err(DslError::UnknownNodeType(node.node_type.clone()));
        }
    }
    Ok(())
}

fn validate_edge_gate_types(workflow: &ParsedWorkflow) -> DslResult<()> {
    let valid_gates = ["receipt", "condition", "timeout", "threshold", "all", "any"];

    for edge in &workflow.edges {
        if let Some(gate_type) = &edge.gate_type {
            if !valid_gates.contains(&gate_type.as_str()) {
                return Err(DslError::UnknownGateType(gate_type.clone()));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ParsedEdge, ParsedNode, ParsedRole};

    fn minimal_workflow() -> ParsedWorkflow {
        ParsedWorkflow {
            name: "Test".into(),
            version: None,
            timeout: None,
            roles: Vec::new(),
            nodes: vec![
                ParsedNode {
                    id: "start".into(),
                    node_type: "start".into(),
                    role: None,
                    commitment: None,
                    receipt: None,
                    timeout: None,
                    escalation_action: None,
                    escalation_param: None,
                },
                ParsedNode {
                    id: "end".into(),
                    node_type: "end".into(),
                    role: None,
                    commitment: None,
                    receipt: None,
                    timeout: None,
                    escalation_action: None,
                    escalation_param: None,
                },
            ],
            edges: vec![ParsedEdge {
                from: "start".into(),
                to: "end".into(),
                gate_type: None,
                gate_value: None,
            }],
            escalations: Vec::new(),
        }
    }

    #[test]
    fn test_valid_minimal() {
        assert!(validate(&minimal_workflow()).is_ok());
    }

    #[test]
    fn test_no_nodes() {
        let mut wf = minimal_workflow();
        wf.nodes.clear();
        wf.edges.clear();
        assert!(matches!(validate(&wf), Err(DslError::ValidationError(_))));
    }

    #[test]
    fn test_no_start_node() {
        let mut wf = minimal_workflow();
        wf.nodes[0].node_type = "action".into();
        assert!(matches!(validate(&wf), Err(DslError::ValidationError(_))));
    }

    #[test]
    fn test_no_end_node() {
        let mut wf = minimal_workflow();
        wf.nodes[1].node_type = "action".into();
        assert!(matches!(validate(&wf), Err(DslError::ValidationError(_))));
    }

    #[test]
    fn test_duplicate_node_ids() {
        let mut wf = minimal_workflow();
        wf.nodes[1].id = "start".into(); // duplicate
        assert!(matches!(validate(&wf), Err(DslError::DuplicateNodeId(_))));
    }

    #[test]
    fn test_duplicate_role_ids() {
        let mut wf = minimal_workflow();
        wf.roles = vec![
            ParsedRole {
                id: "admin".into(),
                description: "Admin".into(),
            },
            ParsedRole {
                id: "admin".into(),
                description: "Duplicate".into(),
            },
        ];
        assert!(matches!(validate(&wf), Err(DslError::DuplicateRoleId(_))));
    }

    #[test]
    fn test_edge_to_nonexistent_node() {
        let mut wf = minimal_workflow();
        wf.edges[0].to = "nonexistent".into();
        assert!(matches!(validate(&wf), Err(DslError::ValidationError(_))));
    }

    #[test]
    fn test_unknown_node_type() {
        let mut wf = minimal_workflow();
        wf.nodes.push(ParsedNode {
            id: "bad".into(),
            node_type: "invalid_type".into(),
            role: None,
            commitment: None,
            receipt: None,
            timeout: None,
            escalation_action: None,
            escalation_param: None,
        });
        assert!(matches!(validate(&wf), Err(DslError::UnknownNodeType(_))));
    }

    #[test]
    fn test_unknown_gate_type() {
        let mut wf = minimal_workflow();
        wf.edges[0].gate_type = Some("unknown_gate".into());
        assert!(matches!(validate(&wf), Err(DslError::UnknownGateType(_))));
    }

    #[test]
    fn test_valid_gate_types() {
        let mut wf = minimal_workflow();
        for gate in &["receipt", "condition", "timeout", "threshold", "all", "any"] {
            wf.edges[0].gate_type = Some(gate.to_string());
            assert!(
                validate(&wf).is_ok(),
                "Gate type '{}' should be valid",
                gate
            );
        }
    }

    #[test]
    fn test_multiple_end_nodes_ok() {
        let mut wf = minimal_workflow();
        wf.nodes.push(ParsedNode {
            id: "end2".into(),
            node_type: "end".into(),
            role: None,
            commitment: None,
            receipt: None,
            timeout: None,
            escalation_action: None,
            escalation_param: None,
        });
        assert!(validate(&wf).is_ok());
    }

    #[test]
    fn test_multiple_start_nodes_invalid() {
        let mut wf = minimal_workflow();
        wf.nodes.push(ParsedNode {
            id: "start2".into(),
            node_type: "start".into(),
            role: None,
            commitment: None,
            receipt: None,
            timeout: None,
            escalation_action: None,
            escalation_param: None,
        });
        assert!(matches!(validate(&wf), Err(DslError::ValidationError(_))));
    }
}
