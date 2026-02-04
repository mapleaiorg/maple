//! State machine: manages node activation and workflow transitions
//!
//! The state machine is the heart of the workflow engine. It determines
//! which nodes to activate next, handles parallel fork/join semantics,
//! and enforces the receipt-gated transition model.

use crate::gate_evaluator::{EvaluationContext, GateEvaluator, GateResult};
use workflow_types::*;

/// Manages workflow node transitions and activation logic
#[derive(Clone, Debug)]
pub struct StateMachine {
    /// Gate evaluator for checking transition conditions
    gate_evaluator: GateEvaluator,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            gate_evaluator: GateEvaluator::new(),
        }
    }

    /// Initialize a workflow instance by activating the start node
    /// and any immediately reachable nodes via automatic transitions.
    pub fn initialize(
        &self,
        instance: &mut WorkflowInstance,
        definition: &WorkflowDefinition,
    ) -> WorkflowResult<()> {
        let start_node = definition.start_node().ok_or(WorkflowError::NoStartNode)?;

        // Activate the start node
        instance.activate_node(start_node.id.clone());

        // Immediately complete the start node (it's a control flow node)
        instance.complete_node(&start_node.id);

        // Advance from start node
        self.advance_from_node(instance, definition, &start_node.id)?;

        Ok(())
    }

    /// Advance the workflow from a completed node.
    ///
    /// Evaluates all outgoing edges from the node and activates
    /// target nodes whose gates are satisfied.
    pub fn advance_from_node(
        &self,
        instance: &mut WorkflowInstance,
        definition: &WorkflowDefinition,
        completed_node_id: &NodeId,
    ) -> WorkflowResult<Vec<NodeId>> {
        let outgoing = definition.outgoing_edges(completed_node_id);
        let mut activated_nodes = Vec::new();

        for edge in &outgoing {
            let context = self.build_context(instance, &edge.source);
            let result = self.gate_evaluator.evaluate(
                &edge.gate,
                &instance.receipts,
                &edge.source,
                &context,
            );

            if result.is_satisfied() {
                let target_node = definition
                    .get_node(&edge.target)
                    .ok_or_else(|| WorkflowError::NodeNotFound(edge.target.clone()))?;

                // Handle based on target node type
                match &target_node.node_type {
                    NodeType::End => {
                        // Reaching an end node — complete the workflow
                        instance.activate_node(target_node.id.clone());
                        instance.complete_node(&target_node.id);
                        instance.complete();
                        activated_nodes.push(target_node.id.clone());
                    }

                    NodeType::ParallelJoin => {
                        // Join node: only activate when ALL incoming edges are satisfied
                        if self.all_incoming_satisfied(instance, definition, &target_node.id) {
                            instance.activate_node(target_node.id.clone());
                            instance.complete_node(&target_node.id);
                            // Continue advancing from the join
                            let further =
                                self.advance_from_node(instance, definition, &target_node.id)?;
                            activated_nodes.push(target_node.id.clone());
                            activated_nodes.extend(further);
                        }
                    }

                    NodeType::ParallelFork => {
                        // Fork node: activate immediately, then activate all targets
                        instance.activate_node(target_node.id.clone());
                        instance.complete_node(&target_node.id);
                        let further =
                            self.advance_from_node(instance, definition, &target_node.id)?;
                        activated_nodes.push(target_node.id.clone());
                        activated_nodes.extend(further);
                    }

                    NodeType::Decision => {
                        // Decision node: activate immediately, then evaluate
                        // outgoing edges (sorted by priority)
                        instance.activate_node(target_node.id.clone());
                        instance.complete_node(&target_node.id);
                        let further =
                            self.advance_from_node(instance, definition, &target_node.id)?;
                        activated_nodes.push(target_node.id.clone());
                        activated_nodes.extend(further);
                    }

                    NodeType::Action | NodeType::SubWorkflow { .. } => {
                        // Action node: activate and wait for fulfillment
                        if !self.is_node_active_or_completed(instance, &target_node.id) {
                            instance.activate_node(target_node.id.clone());
                            activated_nodes.push(target_node.id.clone());
                        }
                    }

                    NodeType::Start => {
                        // Should never have an edge targeting start
                        return Err(WorkflowError::InvalidTransition(
                            "Cannot transition to start node".into(),
                        ));
                    }
                }
            }
        }

        Ok(activated_nodes)
    }

    /// Try to advance all active nodes in the workflow.
    ///
    /// This is called when a receipt is added or a timeout occurs.
    /// Returns the list of newly activated nodes.
    pub fn try_advance_all(
        &self,
        instance: &mut WorkflowInstance,
        definition: &WorkflowDefinition,
    ) -> WorkflowResult<Vec<NodeId>> {
        // Collect completed nodes that might have unsatisfied outgoing edges
        let completed: Vec<NodeId> = instance
            .node_states
            .iter()
            .filter(|(_, s)| s.status == NodeStatus::Completed)
            .map(|(id, _)| id.clone())
            .collect();

        let mut all_activated = Vec::new();
        for node_id in &completed {
            // Check if any outgoing edges are now satisfied
            let outgoing = definition.outgoing_edges(node_id);
            for edge in &outgoing {
                // Skip if target already active or completed
                if self.is_node_active_or_completed(instance, &edge.target) {
                    continue;
                }

                let context = self.build_context(instance, &edge.source);
                let result = self.gate_evaluator.evaluate(
                    &edge.gate,
                    &instance.receipts,
                    &edge.source,
                    &context,
                );

                if result.is_satisfied() {
                    let activated = self.advance_from_node(instance, definition, node_id)?;
                    all_activated.extend(activated);
                    break; // Only advance once from each node
                }
            }
        }

        Ok(all_activated)
    }

    /// Evaluate whether a specific edge's gate is satisfied
    pub fn evaluate_edge(&self, instance: &WorkflowInstance, edge: &WorkflowEdge) -> GateResult {
        let context = self.build_context(instance, &edge.source);
        self.gate_evaluator
            .evaluate(&edge.gate, &instance.receipts, &edge.source, &context)
    }

    // ── Internal helpers ─────────────────────────────────────────────

    /// Check if all incoming edges to a join node have their source nodes completed
    fn all_incoming_satisfied(
        &self,
        instance: &WorkflowInstance,
        definition: &WorkflowDefinition,
        join_node_id: &NodeId,
    ) -> bool {
        let incoming = definition.incoming_edges(join_node_id);
        incoming.iter().all(|edge| {
            instance
                .get_node_state(&edge.source)
                .map(|s| s.status == NodeStatus::Completed)
                .unwrap_or(false)
        })
    }

    /// Check if a node is already active or completed
    fn is_node_active_or_completed(&self, instance: &WorkflowInstance, node_id: &NodeId) -> bool {
        instance
            .get_node_state(node_id)
            .map(|s| matches!(s.status, NodeStatus::Active | NodeStatus::Completed))
            .unwrap_or(false)
    }

    /// Build evaluation context for a node
    fn build_context(&self, instance: &WorkflowInstance, node_id: &NodeId) -> EvaluationContext {
        let node_active_secs = instance
            .get_node_state(node_id)
            .and_then(|s| s.active_duration_secs())
            .unwrap_or(0);

        let mut ctx = EvaluationContext::new().with_active_secs(node_active_secs);

        // Add instance parameters as variables
        for (key, value) in &instance.parameters {
            ctx.variables.insert(key.clone(), value.clone());
        }

        ctx
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collective_types::{CollectiveId, ReceiptType, RoleId};
    use resonator_types::ResonatorId;

    fn make_linear_definition() -> WorkflowDefinition {
        let mut def =
            WorkflowDefinition::new("Linear", CollectiveId::new("c"), ResonatorId::new("author"));
        def.add_node(WorkflowNode::start("start")).unwrap();
        def.add_node(
            WorkflowNode::action("review", "Review").with_assigned_role(RoleId::new("reviewer")),
        )
        .unwrap();
        def.add_node(WorkflowNode::end("end")).unwrap();

        def.add_edge(WorkflowEdge::new(
            NodeId::new("start"),
            NodeId::new("review"),
        ))
        .unwrap();
        def.add_edge(WorkflowEdge::receipt_gated(
            NodeId::new("review"),
            NodeId::new("end"),
            ReceiptType::CommitmentFulfilled,
        ))
        .unwrap();

        def
    }

    fn make_instance(def: &WorkflowDefinition) -> WorkflowInstance {
        let mut inst = WorkflowInstance::new(
            def.id.clone(),
            def.collective_id.clone(),
            ResonatorId::new("initiator"),
        );
        inst.start();
        inst
    }

    #[test]
    fn test_initialize_linear() {
        let def = make_linear_definition();
        let mut inst = make_instance(&def);
        let sm = StateMachine::new();

        sm.initialize(&mut inst, &def).unwrap();

        // Start node should be completed, review should be active
        let start_state = inst.get_node_state(&NodeId::new("start")).unwrap();
        assert_eq!(start_state.status, NodeStatus::Completed);

        let review_state = inst.get_node_state(&NodeId::new("review")).unwrap();
        assert_eq!(review_state.status, NodeStatus::Active);

        assert!(inst.is_active());
    }

    #[test]
    fn test_advance_with_receipt() {
        let def = make_linear_definition();
        let mut inst = make_instance(&def);
        let sm = StateMachine::new();

        sm.initialize(&mut inst, &def).unwrap();

        // Add receipt for review node
        inst.complete_node(&NodeId::new("review"));
        inst.add_receipt(WorkflowReceipt::new(
            NodeId::new("review"),
            ReceiptType::CommitmentFulfilled,
            ResonatorId::new("reviewer-1"),
        ));

        // Advance from review — should reach end
        let activated = sm
            .advance_from_node(&mut inst, &def, &NodeId::new("review"))
            .unwrap();

        assert!(!activated.is_empty());
        assert!(inst.is_terminal());
        assert_eq!(inst.state, WorkflowState::Completed);
    }

    #[test]
    fn test_gate_blocks_without_receipt() {
        let def = make_linear_definition();
        let mut inst = make_instance(&def);
        let sm = StateMachine::new();

        sm.initialize(&mut inst, &def).unwrap();

        // Complete review but DON'T add receipt
        inst.complete_node(&NodeId::new("review"));

        // Try to advance — should not reach end (gate not satisfied)
        let activated = sm
            .advance_from_node(&mut inst, &def, &NodeId::new("review"))
            .unwrap();
        assert!(activated.is_empty());
        assert!(inst.is_active()); // Workflow still active
    }

    #[test]
    fn test_parallel_fork_join() {
        let mut def = WorkflowDefinition::new(
            "Parallel",
            CollectiveId::new("c"),
            ResonatorId::new("author"),
        );

        def.add_node(WorkflowNode::start("start")).unwrap();
        def.add_node(WorkflowNode::fork("fork")).unwrap();
        def.add_node(WorkflowNode::action("task_a", "Task A"))
            .unwrap();
        def.add_node(WorkflowNode::action("task_b", "Task B"))
            .unwrap();
        def.add_node(WorkflowNode::join("join")).unwrap();
        def.add_node(WorkflowNode::end("end")).unwrap();

        def.add_edge(WorkflowEdge::new(NodeId::new("start"), NodeId::new("fork")))
            .unwrap();
        def.add_edge(WorkflowEdge::new(
            NodeId::new("fork"),
            NodeId::new("task_a"),
        ))
        .unwrap();
        def.add_edge(WorkflowEdge::new(
            NodeId::new("fork"),
            NodeId::new("task_b"),
        ))
        .unwrap();
        def.add_edge(WorkflowEdge::new(
            NodeId::new("task_a"),
            NodeId::new("join"),
        ))
        .unwrap();
        def.add_edge(WorkflowEdge::new(
            NodeId::new("task_b"),
            NodeId::new("join"),
        ))
        .unwrap();
        def.add_edge(WorkflowEdge::new(NodeId::new("join"), NodeId::new("end")))
            .unwrap();

        let mut inst = make_instance(&def);
        let sm = StateMachine::new();

        sm.initialize(&mut inst, &def).unwrap();

        // Both task_a and task_b should be active
        let active = inst.active_nodes();
        assert_eq!(active.len(), 2);

        // Complete task_a
        inst.complete_node(&NodeId::new("task_a"));
        let activated = sm
            .advance_from_node(&mut inst, &def, &NodeId::new("task_a"))
            .unwrap();
        // Join should NOT activate yet (task_b still active)
        assert!(activated.is_empty());
        assert!(inst.is_active());

        // Complete task_b
        inst.complete_node(&NodeId::new("task_b"));
        let activated = sm
            .advance_from_node(&mut inst, &def, &NodeId::new("task_b"))
            .unwrap();
        // Join should activate and then reach end
        assert!(!activated.is_empty());
        assert!(inst.is_terminal());
    }

    #[test]
    fn test_decision_node() {
        let mut def = WorkflowDefinition::new(
            "Decision",
            CollectiveId::new("c"),
            ResonatorId::new("author"),
        );

        def.add_node(WorkflowNode::start("start")).unwrap();
        def.add_node(WorkflowNode::action("review", "Review"))
            .unwrap();
        def.add_node(WorkflowNode::decision("decide", "Approve?"))
            .unwrap();
        def.add_node(WorkflowNode::action("publish", "Publish"))
            .unwrap();
        def.add_node(WorkflowNode::action("revise", "Revise"))
            .unwrap();
        def.add_node(WorkflowNode::end("end_ok")).unwrap();
        def.add_node(WorkflowNode::end("end_revise")).unwrap();

        def.add_edge(WorkflowEdge::new(
            NodeId::new("start"),
            NodeId::new("review"),
        ))
        .unwrap();
        def.add_edge(WorkflowEdge::new(
            NodeId::new("review"),
            NodeId::new("decide"),
        ))
        .unwrap();
        def.add_edge(
            WorkflowEdge::conditional(
                NodeId::new("decide"),
                NodeId::new("publish"),
                "decision == approved",
            )
            .with_priority(1),
        )
        .unwrap();
        def.add_edge(
            WorkflowEdge::conditional(
                NodeId::new("decide"),
                NodeId::new("revise"),
                "decision == rejected",
            )
            .with_priority(2),
        )
        .unwrap();
        def.add_edge(WorkflowEdge::new(
            NodeId::new("publish"),
            NodeId::new("end_ok"),
        ))
        .unwrap();
        def.add_edge(WorkflowEdge::new(
            NodeId::new("revise"),
            NodeId::new("end_revise"),
        ))
        .unwrap();

        let mut inst = make_instance(&def);
        inst = inst.with_parameter("decision", "approved");
        let sm = StateMachine::new();

        sm.initialize(&mut inst, &def).unwrap();

        // Review should be active
        assert!(inst
            .get_node_state(&NodeId::new("review"))
            .unwrap()
            .is_active());

        // Complete review
        inst.complete_node(&NodeId::new("review"));
        let activated = sm
            .advance_from_node(&mut inst, &def, &NodeId::new("review"))
            .unwrap();

        // Decision should auto-advance to publish (decision == approved)
        assert!(activated.contains(&NodeId::new("decide")));
        assert!(activated.contains(&NodeId::new("publish")));
    }

    #[test]
    fn test_evaluate_edge() {
        let def = make_linear_definition();
        let inst = make_instance(&def);
        let sm = StateMachine::new();

        let edge = &def.edges[1]; // review -> end (receipt gated)
        let result = sm.evaluate_edge(&inst, edge);
        assert!(!result.is_satisfied());
    }
}
