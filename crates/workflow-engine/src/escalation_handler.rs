//! Escalation handler: monitors timeouts and triggers escalation
//!
//! The escalation handler checks active nodes for timeout conditions
//! and applies the configured escalation paths when triggered.
//! It does NOT take action — it returns escalation decisions for
//! the orchestrator to act upon.

use workflow_types::*;

/// Handles escalation detection and decision-making
#[derive(Clone, Debug)]
pub struct EscalationHandler;

impl EscalationHandler {
    pub fn new() -> Self {
        Self
    }

    /// Check all active nodes for escalation conditions.
    ///
    /// Returns a list of escalation decisions for nodes that need
    /// attention. The orchestrator is responsible for acting on these.
    pub fn check_escalations(
        &self,
        instance: &WorkflowInstance,
        definition: &WorkflowDefinition,
    ) -> Vec<EscalationDecision> {
        let mut decisions = Vec::new();

        for (node_id, node_state) in &instance.node_states {
            if node_state.status != NodeStatus::Active {
                continue;
            }

            // Find the node definition
            let node_def = match definition.get_node(node_id) {
                Some(n) => n,
                None => continue,
            };

            // Check for timeout
            if let Some(timeout_secs) = node_def.timeout_secs {
                if node_state.is_timed_out(timeout_secs) {
                    let escalation_path = node_def
                        .escalation
                        .clone()
                        .or_else(|| definition.default_escalation.clone());

                    decisions.push(EscalationDecision {
                        node_id: node_id.clone(),
                        trigger: EscalationTrigger::Timeout { timeout_secs },
                        recommended_action: self.determine_action(&escalation_path, node_state),
                        escalation_path,
                    });
                }
            }
        }

        decisions
    }

    /// Check a specific node for escalation
    pub fn check_node(
        &self,
        instance: &WorkflowInstance,
        definition: &WorkflowDefinition,
        node_id: &NodeId,
    ) -> Option<EscalationDecision> {
        let node_state = instance.get_node_state(node_id)?;
        if node_state.status != NodeStatus::Active {
            return None;
        }

        let node_def = definition.get_node(node_id)?;

        if let Some(timeout_secs) = node_def.timeout_secs {
            if node_state.is_timed_out(timeout_secs) {
                let escalation_path = node_def
                    .escalation
                    .clone()
                    .or_else(|| definition.default_escalation.clone());

                return Some(EscalationDecision {
                    node_id: node_id.clone(),
                    trigger: EscalationTrigger::Timeout { timeout_secs },
                    recommended_action: self.determine_action(&escalation_path, node_state),
                    escalation_path,
                });
            }
        }

        None
    }

    /// Check if the entire workflow has timed out
    pub fn check_workflow_timeout(
        &self,
        instance: &WorkflowInstance,
        definition: &WorkflowDefinition,
    ) -> Option<EscalationDecision> {
        if let Some(max_duration) = definition.max_duration_secs {
            if instance.elapsed_secs() >= max_duration as i64 {
                return Some(EscalationDecision {
                    node_id: NodeId::new("__workflow__"),
                    trigger: EscalationTrigger::Timeout {
                        timeout_secs: max_duration,
                    },
                    recommended_action: RecommendedAction::AbortWorkflow {
                        reason: format!(
                            "Workflow exceeded maximum duration of {} seconds",
                            max_duration
                        ),
                    },
                    escalation_path: definition.default_escalation.clone(),
                });
            }
        }
        None
    }

    // ── Internal ─────────────────────────────────────────────────────

    /// Determine the recommended action based on the escalation path
    fn determine_action(
        &self,
        path: &Option<EscalationPath>,
        node_state: &NodeState,
    ) -> RecommendedAction {
        match path {
            Some(path) => {
                match &path.action {
                    EscalationAction::Retry => {
                        if node_state.retry_count < path.max_retries {
                            RecommendedAction::RetryNode
                        } else {
                            // Max retries exceeded — escalate
                            RecommendedAction::AbortWorkflow {
                                reason: format!("Max retries ({}) exceeded", path.max_retries),
                            }
                        }
                    }
                    EscalationAction::Abort { reason } => RecommendedAction::AbortWorkflow {
                        reason: reason.clone(),
                    },
                    EscalationAction::EscalateToRole { role } => {
                        RecommendedAction::EscalateToRole { role: role.clone() }
                    }
                    EscalationAction::RedirectToNode { node_id } => {
                        RecommendedAction::RedirectToNode {
                            node_id: NodeId::new(node_id.clone()),
                        }
                    }
                    EscalationAction::Pause { reason } => RecommendedAction::PauseWorkflow {
                        reason: reason.clone(),
                    },
                    EscalationAction::Skip => RecommendedAction::SkipNode,
                    EscalationAction::Compensate { description } => RecommendedAction::Compensate {
                        description: description.clone(),
                    },
                }
            }
            None => {
                // No escalation path configured — default to abort
                RecommendedAction::AbortWorkflow {
                    reason: "No escalation path configured".into(),
                }
            }
        }
    }
}

impl Default for EscalationHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// An escalation decision for the orchestrator to act upon
#[derive(Clone, Debug)]
pub struct EscalationDecision {
    /// The node that triggered escalation
    pub node_id: NodeId,
    /// What triggered the escalation
    pub trigger: EscalationTrigger,
    /// What the handler recommends doing
    pub recommended_action: RecommendedAction,
    /// The configured escalation path (if any)
    pub escalation_path: Option<EscalationPath>,
}

/// Recommended action from the escalation handler
#[derive(Clone, Debug)]
pub enum RecommendedAction {
    /// Retry the node
    RetryNode,
    /// Skip the node and move on
    SkipNode,
    /// Abort the entire workflow
    AbortWorkflow { reason: String },
    /// Escalate to a specific role
    EscalateToRole { role: collective_types::RoleId },
    /// Redirect to a different node
    RedirectToNode { node_id: NodeId },
    /// Pause the workflow
    PauseWorkflow { reason: String },
    /// Compensate (undo previous actions)
    Compensate { description: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use collective_types::{CollectiveId, RoleId};
    use resonator_types::ResonatorId;

    fn make_definition_with_timeout() -> WorkflowDefinition {
        let mut def =
            WorkflowDefinition::new("Timed", CollectiveId::new("c"), ResonatorId::new("author"));

        def.add_node(WorkflowNode::start("start")).unwrap();
        def.add_node(
            WorkflowNode::action("task", "Timed Task")
                .with_timeout(60) // 60 second timeout
                .with_escalation(EscalationPath::timeout_retry(60, 3)),
        )
        .unwrap();
        def.add_node(WorkflowNode::end("end")).unwrap();

        def.add_edge(WorkflowEdge::new(NodeId::new("start"), NodeId::new("task")))
            .unwrap();
        def.add_edge(WorkflowEdge::new(NodeId::new("task"), NodeId::new("end")))
            .unwrap();

        def
    }

    #[test]
    fn test_no_escalation_when_not_timed_out() {
        let def = make_definition_with_timeout();
        let mut inst = WorkflowInstance::new(
            def.id.clone(),
            def.collective_id.clone(),
            ResonatorId::new("init"),
        );
        inst.start();
        inst.activate_node(NodeId::new("task"));

        let handler = EscalationHandler::new();
        let decisions = handler.check_escalations(&inst, &def);
        // Just activated — shouldn't be timed out
        assert!(decisions.is_empty());
    }

    #[test]
    fn test_escalation_retry_recommendation() {
        let handler = EscalationHandler::new();

        let path = EscalationPath::timeout_retry(60, 3);
        let node_state = NodeState {
            status: NodeStatus::Active,
            activated_at: None,
            completed_at: None,
            assigned_resonator: None,
            escalation: EscalationState::new(),
            retry_count: 0,
        };

        let action = handler.determine_action(&Some(path), &node_state);
        assert!(matches!(action, RecommendedAction::RetryNode));
    }

    #[test]
    fn test_escalation_max_retries_exceeded() {
        let handler = EscalationHandler::new();

        let path = EscalationPath::timeout_retry(60, 2);
        let node_state = NodeState {
            status: NodeStatus::Active,
            activated_at: None,
            completed_at: None,
            assigned_resonator: None,
            escalation: EscalationState::new(),
            retry_count: 3, // exceeded max_retries of 2
        };

        let action = handler.determine_action(&Some(path), &node_state);
        assert!(matches!(action, RecommendedAction::AbortWorkflow { .. }));
    }

    #[test]
    fn test_escalation_abort_action() {
        let handler = EscalationHandler::new();

        let path = EscalationPath::timeout_abort(300);
        let node_state = NodeState {
            status: NodeStatus::Active,
            activated_at: None,
            completed_at: None,
            assigned_resonator: None,
            escalation: EscalationState::new(),
            retry_count: 0,
        };

        let action = handler.determine_action(&Some(path), &node_state);
        assert!(matches!(action, RecommendedAction::AbortWorkflow { .. }));
    }

    #[test]
    fn test_escalation_to_role() {
        let handler = EscalationHandler::new();

        let path = EscalationPath::escalate_to_role(
            EscalationTrigger::CommitmentBroken,
            RoleId::new("supervisor"),
        );
        let node_state = NodeState {
            status: NodeStatus::Active,
            activated_at: None,
            completed_at: None,
            assigned_resonator: None,
            escalation: EscalationState::new(),
            retry_count: 0,
        };

        let action = handler.determine_action(&Some(path), &node_state);
        assert!(matches!(action, RecommendedAction::EscalateToRole { .. }));
    }

    #[test]
    fn test_no_escalation_path_defaults_to_abort() {
        let handler = EscalationHandler::new();

        let node_state = NodeState {
            status: NodeStatus::Active,
            activated_at: None,
            completed_at: None,
            assigned_resonator: None,
            escalation: EscalationState::new(),
            retry_count: 0,
        };

        let action = handler.determine_action(&None, &node_state);
        assert!(matches!(action, RecommendedAction::AbortWorkflow { .. }));
    }

    #[test]
    fn test_skip_action() {
        let handler = EscalationHandler::new();

        let path = EscalationPath::new(
            EscalationTrigger::Timeout { timeout_secs: 60 },
            EscalationAction::Skip,
        );
        let node_state = NodeState {
            status: NodeStatus::Active,
            activated_at: None,
            completed_at: None,
            assigned_resonator: None,
            escalation: EscalationState::new(),
            retry_count: 0,
        };

        let action = handler.determine_action(&Some(path), &node_state);
        assert!(matches!(action, RecommendedAction::SkipNode));
    }

    #[test]
    fn test_pause_action() {
        let handler = EscalationHandler::new();

        let path = EscalationPath::new(
            EscalationTrigger::Timeout { timeout_secs: 60 },
            EscalationAction::Pause {
                reason: "Needs human".into(),
            },
        );
        let node_state = NodeState {
            status: NodeStatus::Active,
            activated_at: None,
            completed_at: None,
            assigned_resonator: None,
            escalation: EscalationState::new(),
            retry_count: 0,
        };

        let action = handler.determine_action(&Some(path), &node_state);
        assert!(matches!(action, RecommendedAction::PauseWorkflow { .. }));
    }

    #[test]
    fn test_check_node_not_active() {
        let def = make_definition_with_timeout();
        let mut inst = WorkflowInstance::new(
            def.id.clone(),
            def.collective_id.clone(),
            ResonatorId::new("init"),
        );
        inst.start();
        inst.activate_node(NodeId::new("task"));
        inst.complete_node(&NodeId::new("task"));

        let handler = EscalationHandler::new();
        let decision = handler.check_node(&inst, &def, &NodeId::new("task"));
        assert!(decision.is_none()); // Not active, no escalation
    }
}
