//! Compiler: converts parsed DSL into WorkflowDefinition
//!
//! Takes a validated ParsedWorkflow and produces a
//! WorkflowDefinition from the workflow-types crate.

use crate::errors::{DslError, DslResult};
use crate::parser::{ParsedEdge, ParsedEscalation, ParsedNode, ParsedWorkflow, Parser};
use crate::validator;
use collective_types::{CollectiveId, ReceiptType, RoleId};
use resonator_types::ResonatorId;
use workflow_types::*;

/// Compile a DSL string directly into a WorkflowDefinition
pub fn compile(
    input: &str,
    collective_id: CollectiveId,
    author: ResonatorId,
) -> DslResult<WorkflowDefinition> {
    let parsed = Parser::parse(input)?;
    validator::validate(&parsed)?;
    compile_parsed(parsed, collective_id, author)
}

/// Compile a pre-parsed workflow into a WorkflowDefinition
fn compile_parsed(
    parsed: ParsedWorkflow,
    collective_id: CollectiveId,
    author: ResonatorId,
) -> DslResult<WorkflowDefinition> {
    let mut def = WorkflowDefinition::new(&parsed.name, collective_id, author);

    if let Some(version) = &parsed.version {
        def.version = version.parse::<u32>().unwrap_or(1);
    }

    if let Some(timeout) = parsed.timeout {
        def.max_duration_secs = Some(timeout);
    }

    // Compile roles
    for role in &parsed.roles {
        def.add_role(WorkflowRole::new(RoleId::new(&role.id), &role.description));
    }

    // Compile nodes
    for node in &parsed.nodes {
        let wf_node = compile_node(node)?;
        def.add_node(wf_node).map_err(|e| {
            DslError::CompilationError(format!("Failed to add node '{}': {}", node.id, e))
        })?;
    }

    // Compile edges
    for edge in &parsed.edges {
        let wf_edge = compile_edge(edge)?;
        def.add_edge(wf_edge).map_err(|e| {
            DslError::CompilationError(format!(
                "Failed to add edge '{}' -> '{}': {}",
                edge.from, edge.to, e
            ))
        })?;
    }

    // Compile default escalation
    if let Some(esc) = parsed.escalations.first() {
        let escalation = compile_escalation(esc)?;
        def.default_escalation = Some(escalation);
    }

    // Validate the compiled definition
    def.validate()?;

    Ok(def)
}

fn compile_node(node: &ParsedNode) -> DslResult<WorkflowNode> {
    let node_type = match node.node_type.as_str() {
        "start" => NodeType::Start,
        "end" => NodeType::End,
        "action" => NodeType::Action,
        "decision" => NodeType::Decision,
        "fork" => NodeType::ParallelFork,
        "join" => NodeType::ParallelJoin,
        "subworkflow" => NodeType::SubWorkflow {
            // For sub-workflows, COMMITMENT can carry an explicit workflow definition id.
            // When omitted we default to the node id so the compiled graph is deterministic.
            definition_id: WorkflowDefinitionId::new(
                node.commitment.as_deref().unwrap_or(node.id.as_str()),
            ),
        },
        other => return Err(DslError::UnknownNodeType(other.into())),
    };

    let mut wf_node = WorkflowNode::new(&node.id, &node.id, node_type);

    if let Some(role) = &node.role {
        wf_node = wf_node.with_assigned_role(RoleId::new(role));
    }

    if let Some(commitment) = &node.commitment {
        let template = CommitmentTemplate::new(commitment, CommitmentActionType::Execute);
        wf_node = wf_node.with_commitment_template(template);
    }

    if let Some(timeout) = node.timeout {
        wf_node = wf_node.with_timeout(timeout);
    }

    if let Some(esc_action) = &node.escalation_action {
        let escalation = compile_node_escalation(esc_action, node.escalation_param.as_deref())?;
        wf_node = wf_node.with_escalation(escalation);
    }

    Ok(wf_node)
}

fn compile_edge(edge: &ParsedEdge) -> DslResult<WorkflowEdge> {
    let source = NodeId::new(&edge.from);
    let target = NodeId::new(&edge.to);

    let gate = match (edge.gate_type.as_deref(), edge.gate_value.as_deref()) {
        (None, _) => TransitionGate::Automatic,
        (Some("receipt"), Some(receipt_type)) => {
            let rt = parse_receipt_type(receipt_type)?;
            TransitionGate::ReceiptEmitted { receipt_type: rt }
        }
        (Some("receipt"), None) => {
            return Err(DslError::MissingField(
                "Receipt type required for receipt gate".into(),
            ))
        }
        (Some("condition"), Some(expr)) => TransitionGate::Condition {
            expression: expr.to_string(),
        },
        (Some("condition"), None) => {
            return Err(DslError::MissingField(
                "Expression required for condition gate".into(),
            ))
        }
        (Some("timeout"), Some(secs)) => {
            let timeout = secs.parse::<u64>().map_err(|_| DslError::InvalidValue {
                field: "timeout".into(),
                message: format!("'{}' is not a valid timeout value", secs),
            })?;
            TransitionGate::Timeout {
                timeout_secs: timeout,
            }
        }
        (Some("timeout"), None) => {
            return Err(DslError::MissingField(
                "Duration required for timeout gate".into(),
            ))
        }
        (Some("threshold"), Some(count)) => {
            let min_sig = count.parse::<u32>().map_err(|_| DslError::InvalidValue {
                field: "threshold".into(),
                message: format!("'{}' is not a valid signature count", count),
            })?;
            TransitionGate::ThresholdMet {
                description: "Threshold approval required".into(),
                min_signatures: min_sig,
            }
        }
        (Some("threshold"), None) => {
            return Err(DslError::MissingField(
                "Min signatures required for threshold gate".into(),
            ))
        }
        (Some(other), _) => {
            return Err(DslError::UnknownGateType(other.into()));
        }
    };

    Ok(WorkflowEdge::new(source, target).with_gate(gate))
}

fn compile_escalation(esc: &ParsedEscalation) -> DslResult<EscalationPath> {
    let trigger = match esc.trigger.as_str() {
        "timeout" => EscalationTrigger::Timeout { timeout_secs: 0 },
        "commitment_broken" => EscalationTrigger::CommitmentBroken,
        "policy_violation" => EscalationTrigger::PolicyViolation {
            violation: esc.param.clone().unwrap_or_default(),
        },
        "manual" => EscalationTrigger::Manual,
        "budget_exhausted" => EscalationTrigger::BudgetExhausted,
        other => {
            return Err(DslError::ValidationError(format!(
                "Unknown escalation trigger: '{}'",
                other
            )))
        }
    };

    let action = match esc.action.as_str() {
        "abort" => EscalationAction::Abort {
            reason: esc
                .param
                .clone()
                .unwrap_or_else(|| "Escalation abort".into()),
        },
        "retry" => EscalationAction::Retry,
        "pause" => EscalationAction::Pause {
            reason: esc
                .param
                .clone()
                .unwrap_or_else(|| "Escalation pause".into()),
        },
        "skip" => EscalationAction::Skip,
        "escalate" => EscalationAction::EscalateToRole {
            role: RoleId::new(esc.param.as_deref().unwrap_or("admin")),
        },
        other => return Err(DslError::UnknownEscalationAction(other.into())),
    };

    let mut path = EscalationPath::new(trigger, action);
    if esc.action == "retry" {
        if let Some(param) = &esc.param {
            if let Ok(max) = param.parse::<u32>() {
                path.max_retries = max;
            }
        }
    }

    Ok(path)
}

fn compile_node_escalation(action: &str, param: Option<&str>) -> DslResult<EscalationPath> {
    match action {
        "timeout_abort" => Ok(EscalationPath::timeout_abort(0)),
        "timeout_retry" => {
            let max_retries = param.and_then(|p| p.parse::<u32>().ok()).unwrap_or(3);
            Ok(EscalationPath::timeout_retry(0, max_retries))
        }
        "escalate_to" => {
            let role = param.unwrap_or("admin");
            Ok(EscalationPath::escalate_to_role(
                EscalationTrigger::Timeout { timeout_secs: 0 },
                RoleId::new(role),
            ))
        }
        "skip" => Ok(EscalationPath::new(
            EscalationTrigger::Timeout { timeout_secs: 0 },
            EscalationAction::Skip,
        )),
        "pause" => Ok(EscalationPath::new(
            EscalationTrigger::Timeout { timeout_secs: 0 },
            EscalationAction::Pause {
                reason: param.unwrap_or("Paused via escalation").to_string(),
            },
        )),
        other => Err(DslError::UnknownEscalationAction(other.into())),
    }
}

fn parse_receipt_type(s: &str) -> DslResult<ReceiptType> {
    match s {
        "CommitmentFulfilled" => Ok(ReceiptType::CommitmentFulfilled),
        "CommitmentBroken" => Ok(ReceiptType::CommitmentBroken),
        "WorkflowStep" => Ok(ReceiptType::WorkflowStep),
        "Audit" => Ok(ReceiptType::Audit),
        "Financial" => Ok(ReceiptType::Financial),
        "DisputeResolution" => Ok(ReceiptType::DisputeResolution),
        "CollaborationCompleted" => Ok(ReceiptType::CollaborationCompleted),
        "DisputeWon" => Ok(ReceiptType::DisputeWon),
        "DisputeLost" => Ok(ReceiptType::DisputeLost),
        other => Err(DslError::UnknownReceiptType(other.into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c() -> CollectiveId {
        CollectiveId::new("test")
    }
    fn a() -> ResonatorId {
        ResonatorId::new("author")
    }

    #[test]
    fn test_compile_minimal() {
        let input = r#"
        WORKFLOW "Minimal" {
            NODE start TYPE start
            NODE end TYPE end
            EDGES {
                start -> end
            }
        }
        "#;

        let def = compile(input, c(), a()).unwrap();
        assert_eq!(def.name, "Minimal");
        assert_eq!(def.node_count(), 2);
        assert_eq!(def.edge_count(), 1);
        assert!(def.validate().is_ok());
    }

    #[test]
    fn test_compile_with_roles_and_nodes() {
        let input = r#"
        WORKFLOW "Review" {
            VERSION "2"
            TIMEOUT 86400

            ROLES {
                reviewer: "Reviews documents"
                approver: "Approves documents"
            }

            NODE start TYPE start
            NODE review TYPE action {
                ROLE reviewer
                COMMITMENT "Review the document"
                TIMEOUT 3600
            }
            NODE approve TYPE action {
                ROLE approver
                COMMITMENT "Approve the document"
            }
            NODE end TYPE end

            EDGES {
                start -> review
                review -> approve ON receipt CommitmentFulfilled
                approve -> end ON receipt CommitmentFulfilled
            }
        }
        "#;

        let def = compile(input, c(), a()).unwrap();
        assert_eq!(def.name, "Review");
        assert_eq!(def.version, 2);
        assert_eq!(def.max_duration_secs, Some(86400));
        assert_eq!(def.required_roles.len(), 2);
        assert_eq!(def.node_count(), 4);
        assert_eq!(def.edge_count(), 3);

        // Check review node
        let review = def.get_node(&NodeId::new("review")).unwrap();
        assert!(review.commitment_template.is_some());
        assert_eq!(review.timeout_secs, Some(3600));
        assert!(review.assigned_role.is_some());
    }

    #[test]
    fn test_compile_with_escalation() {
        let input = r#"
        WORKFLOW "Escalated" {
            NODE start TYPE start
            NODE task TYPE action {
                COMMITMENT "Do something"
                TIMEOUT 600
                ESCALATION timeout_retry 3
            }
            NODE end TYPE end

            EDGES {
                start -> task
                task -> end
            }

            ESCALATION {
                ON timeout -> abort "Global timeout"
            }
        }
        "#;

        let def = compile(input, c(), a()).unwrap();
        assert!(def.default_escalation.is_some());

        let task = def.get_node(&NodeId::new("task")).unwrap();
        assert!(task.escalation.is_some());
    }

    #[test]
    fn test_compile_decision_workflow() {
        let input = r#"
        WORKFLOW "Decision" {
            NODE start TYPE start
            NODE decide TYPE decision
            NODE yes TYPE action { COMMITMENT "Approved path" }
            NODE no TYPE action { COMMITMENT "Rejected path" }
            NODE end_yes TYPE end
            NODE end_no TYPE end

            EDGES {
                start -> decide
                decide -> yes ON condition "approved"
                decide -> no ON condition "rejected"
                yes -> end_yes
                no -> end_no
            }
        }
        "#;

        let def = compile(input, c(), a()).unwrap();
        assert_eq!(def.node_count(), 6);

        let decide = def.get_node(&NodeId::new("decide")).unwrap();
        assert_eq!(decide.node_type, NodeType::Decision);
    }

    #[test]
    fn test_compile_parallel_workflow() {
        let input = r#"
        WORKFLOW "Parallel" {
            NODE start TYPE start
            NODE fork TYPE fork
            NODE a TYPE action { COMMITMENT "Task A" }
            NODE b TYPE action { COMMITMENT "Task B" }
            NODE join TYPE join
            NODE end TYPE end

            EDGES {
                start -> fork
                fork -> a
                fork -> b
                a -> join
                b -> join
                join -> end
            }
        }
        "#;

        let def = compile(input, c(), a()).unwrap();
        assert_eq!(def.node_count(), 6);
        assert_eq!(def.edge_count(), 6);

        let fork = def.get_node(&NodeId::new("fork")).unwrap();
        assert_eq!(fork.node_type, NodeType::ParallelFork);
    }

    #[test]
    fn test_compile_timeout_edge() {
        let input = r#"
        WORKFLOW "Timeout" {
            NODE start TYPE start
            NODE wait TYPE action { COMMITMENT "Wait" }
            NODE timeout_handler TYPE action { COMMITMENT "Handle timeout" }
            NODE end TYPE end

            EDGES {
                start -> wait
                wait -> end ON receipt CommitmentFulfilled
                wait -> timeout_handler ON timeout 3600
                timeout_handler -> end
            }
        }
        "#;

        let def = compile(input, c(), a()).unwrap();
        let edges = def.outgoing_edges(&NodeId::new("wait"));
        assert_eq!(edges.len(), 2);

        // Find the timeout edge
        let timeout_edge = edges
            .iter()
            .find(|e| matches!(e.gate, TransitionGate::Timeout { .. }));
        assert!(timeout_edge.is_some());
    }

    #[test]
    fn test_compile_subworkflow_uses_explicit_definition_id() {
        let input = r#"
        WORKFLOW "Subflow" {
            NODE start TYPE start
            NODE call_kyc TYPE subworkflow {
                COMMITMENT "kyc-v2"
            }
            NODE end TYPE end

            EDGES {
                start -> call_kyc
                call_kyc -> end
            }
        }
        "#;

        let def = compile(input, c(), a()).unwrap();
        let call = def.get_node(&NodeId::new("call_kyc")).unwrap();
        match &call.node_type {
            NodeType::SubWorkflow { definition_id } => {
                assert_eq!(definition_id, &WorkflowDefinitionId::new("kyc-v2"));
            }
            other => panic!("Expected subworkflow node, got {:?}", other),
        }
    }

    #[test]
    fn test_compile_subworkflow_defaults_definition_id_to_node_id() {
        let input = r#"
        WORKFLOW "SubflowDefault" {
            NODE start TYPE start
            NODE child_flow TYPE subworkflow
            NODE end TYPE end

            EDGES {
                start -> child_flow
                child_flow -> end
            }
        }
        "#;

        let def = compile(input, c(), a()).unwrap();
        let call = def.get_node(&NodeId::new("child_flow")).unwrap();
        match &call.node_type {
            NodeType::SubWorkflow { definition_id } => {
                assert_eq!(definition_id, &WorkflowDefinitionId::new("child_flow"));
            }
            other => panic!("Expected subworkflow node, got {:?}", other),
        }
    }

    #[test]
    fn test_compile_invalid_receipt_type() {
        let input = r#"
        WORKFLOW "Bad" {
            NODE start TYPE start
            NODE end TYPE end
            EDGES {
                start -> end ON receipt InvalidType
            }
        }
        "#;

        let result = compile(input, c(), a());
        assert!(matches!(result, Err(DslError::UnknownReceiptType(_))));
    }

    #[test]
    fn test_compile_disconnected_graph_fails() {
        let input = r#"
        WORKFLOW "Disconnected" {
            NODE start TYPE start
            NODE island TYPE action { COMMITMENT "Isolated" }
            NODE end TYPE end
            EDGES {
                start -> end
            }
        }
        "#;

        let result = compile(input, c(), a());
        // WorkflowDefinition::validate catches disconnected graphs
        assert!(result.is_err());
    }

    #[test]
    fn test_round_trip_compile_validate() {
        let input = r#"
        WORKFLOW "Complete" {
            VERSION "1"
            TIMEOUT 7200

            ROLES {
                submitter: "Submits requests"
                reviewer: "Reviews requests"
            }

            NODE start TYPE start
            NODE submit TYPE action {
                ROLE submitter
                COMMITMENT "Submit request"
                TIMEOUT 1800
            }
            NODE review TYPE action {
                ROLE reviewer
                COMMITMENT "Review request"
                TIMEOUT 3600
                ESCALATION timeout_retry 2
            }
            NODE end TYPE end

            EDGES {
                start -> submit
                submit -> review ON receipt CommitmentFulfilled
                review -> end ON receipt CommitmentFulfilled
            }

            ESCALATION {
                ON timeout -> abort "Workflow timed out"
            }
        }
        "#;

        let def = compile(input, c(), a()).unwrap();

        // Validate the output is a proper WorkflowDefinition
        assert!(def.validate().is_ok());
        assert_eq!(def.name, "Complete");
        assert_eq!(def.version, 1);
        assert_eq!(def.node_count(), 4);
        assert_eq!(def.edge_count(), 3);
        assert_eq!(def.required_roles.len(), 2);
        assert!(def.default_escalation.is_some());
        assert!(def.start_node().is_some());
        assert_eq!(def.end_nodes().len(), 1);
    }

    #[test]
    fn test_compile_all_receipt_types() {
        let types = vec![
            "CommitmentFulfilled",
            "CommitmentBroken",
            "WorkflowStep",
            "Audit",
            "Financial",
            "DisputeResolution",
            "CollaborationCompleted",
            "DisputeWon",
            "DisputeLost",
        ];

        for rt in &types {
            assert!(
                parse_receipt_type(rt).is_ok(),
                "Receipt type '{}' should parse",
                rt
            );
        }
    }
}
