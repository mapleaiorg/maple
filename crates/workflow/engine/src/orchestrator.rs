//! Workflow Orchestrator: the main entry point for the workflow engine
//!
//! The orchestrator coordinates commitment programs. It:
//! 1. Registers workflow definitions
//! 2. Launches workflow instances
//! 3. Processes receipts and advances workflows
//! 4. Handles escalation
//! 5. Maintains provenance
//!
//! **CRITICAL**: The orchestrator NEVER executes actions directly.
//! It declares commitments and waits for receipts. The actual work
//! is done by resonators assigned to workflow roles.

use crate::{
    escalation_handler::RecommendedAction, DefinitionRegistry, EscalationHandler,
    ProvenanceTracker, StateMachine,
};
use collective_types::CollectiveId;
use resonator_types::ResonatorId;
use std::collections::HashMap;
use workflow_types::*;

/// The Workflow Orchestrator — coordinates, never acts
#[derive(Clone, Debug)]
pub struct WorkflowOrchestrator {
    /// The collective this orchestrator belongs to
    collective_id: CollectiveId,
    /// Registry of workflow definitions
    definitions: DefinitionRegistry,
    /// Running and completed workflow instances
    instances: HashMap<WorkflowInstanceId, WorkflowInstance>,
    /// State machine for transition logic
    state_machine: StateMachine,
    /// Escalation handler
    escalation_handler: EscalationHandler,
    /// Provenance tracker
    provenance: ProvenanceTracker,
    /// Completed workflow records
    completed: Vec<CompletedWorkflow>,
}

impl WorkflowOrchestrator {
    /// Create a new workflow orchestrator for a collective
    pub fn new(collective_id: CollectiveId) -> Self {
        Self {
            collective_id,
            definitions: DefinitionRegistry::new(),
            instances: HashMap::new(),
            state_machine: StateMachine::new(),
            escalation_handler: EscalationHandler::new(),
            provenance: ProvenanceTracker::new(),
            completed: Vec::new(),
        }
    }

    // ── Definition Management ────────────────────────────────────────

    /// Register a workflow definition
    pub fn register_definition(
        &mut self,
        definition: WorkflowDefinition,
    ) -> WorkflowResult<WorkflowDefinitionId> {
        self.definitions.register(definition)
    }

    /// Get a workflow definition
    pub fn get_definition(&self, id: &WorkflowDefinitionId) -> WorkflowResult<&WorkflowDefinition> {
        self.definitions.get(id)
    }

    /// List all definitions
    pub fn list_definitions(&self) -> Vec<&WorkflowDefinition> {
        self.definitions.list()
    }

    /// Number of registered definitions
    pub fn definition_count(&self) -> usize {
        self.definitions.count()
    }

    // ── Instance Lifecycle ───────────────────────────────────────────

    /// Launch a new workflow instance from a registered definition.
    ///
    /// This initializes the instance, activates the start node,
    /// and advances through any automatic transitions.
    pub fn launch_instance(
        &mut self,
        definition_id: &WorkflowDefinitionId,
        initiator: ResonatorId,
    ) -> WorkflowResult<WorkflowInstanceId> {
        let definition = self.definitions.get(definition_id)?.clone();

        let mut instance = WorkflowInstance::new(
            definition_id.clone(),
            self.collective_id.clone(),
            initiator.clone(),
        );
        instance.start();

        let instance_id = instance.id.clone();

        // Record provenance
        self.provenance.record_started(&instance_id, &initiator);

        // Initialize the state machine (activates start node, advances)
        self.state_machine.initialize(&mut instance, &definition)?;

        // Record activated nodes
        for node_id in instance.active_nodes() {
            self.provenance.record_node_activated(&instance_id, node_id);
        }

        tracing::info!(
            instance_id = %instance_id,
            definition = %definition_id,
            "Workflow instance launched"
        );

        self.instances.insert(instance_id.clone(), instance);
        Ok(instance_id)
    }

    /// Launch an instance with parameters
    pub fn launch_instance_with_params(
        &mut self,
        definition_id: &WorkflowDefinitionId,
        initiator: ResonatorId,
        params: HashMap<String, String>,
    ) -> WorkflowResult<WorkflowInstanceId> {
        let definition = self.definitions.get(definition_id)?.clone();

        let mut instance = WorkflowInstance::new(
            definition_id.clone(),
            self.collective_id.clone(),
            initiator.clone(),
        );
        instance.parameters = params;
        instance.start();

        let instance_id = instance.id.clone();

        self.provenance.record_started(&instance_id, &initiator);
        self.state_machine.initialize(&mut instance, &definition)?;

        for node_id in instance.active_nodes() {
            self.provenance.record_node_activated(&instance_id, node_id);
        }

        self.instances.insert(instance_id.clone(), instance);
        Ok(instance_id)
    }

    /// Get a workflow instance
    pub fn get_instance(&self, id: &WorkflowInstanceId) -> WorkflowResult<&WorkflowInstance> {
        self.instances
            .get(id)
            .ok_or_else(|| WorkflowError::InstanceNotFound(id.clone()))
    }

    /// Get a mutable workflow instance
    fn get_instance_mut(
        &mut self,
        id: &WorkflowInstanceId,
    ) -> WorkflowResult<&mut WorkflowInstance> {
        self.instances
            .get_mut(id)
            .ok_or_else(|| WorkflowError::InstanceNotFound(id.clone()))
    }

    /// List all active instances
    pub fn active_instances(&self) -> Vec<&WorkflowInstance> {
        self.instances.values().filter(|i| i.is_active()).collect()
    }

    /// Total number of instances (active + terminal)
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    // ── Receipt Processing ───────────────────────────────────────────

    /// Submit a receipt for a workflow node.
    ///
    /// This is the primary way to advance a workflow. When a receipt
    /// is submitted, the engine checks if any transition gates are
    /// now satisfied and advances the workflow accordingly.
    pub fn submit_receipt(
        &mut self,
        instance_id: &WorkflowInstanceId,
        receipt: WorkflowReceipt,
    ) -> WorkflowResult<Vec<NodeId>> {
        // Get the definition
        let instance = self.get_instance(instance_id)?;
        if instance.is_terminal() {
            return Err(WorkflowError::AlreadyCompleted);
        }
        let def_id = instance.definition_id.clone();
        let definition = self.definitions.get(&def_id)?.clone();

        let receipt_node = receipt.node_id.clone();
        let receipt_id = receipt.receipt_id.clone();
        let emitter = receipt.emitter.clone();

        // Record provenance
        self.provenance
            .record_receipt_emitted(instance_id, &receipt_node, &receipt_id, &emitter);

        // Add receipt to instance
        let instance = self.get_instance_mut(instance_id)?;
        instance.add_receipt(receipt);

        // Complete the node that emitted the receipt
        instance.complete_node(&receipt_node);
        self.provenance
            .record_node_completed(instance_id, &receipt_node, Some(&emitter));

        // Advance the workflow from the completed node
        let instance = self
            .instances
            .get_mut(instance_id)
            .ok_or_else(|| WorkflowError::InstanceNotFound(instance_id.clone()))?;
        let activated =
            self.state_machine
                .advance_from_node(instance, &definition, &receipt_node)?;

        // Record provenance for activated nodes
        for node_id in &activated {
            self.provenance.record_node_activated(instance_id, node_id);
            self.provenance
                .record_transition(instance_id, &receipt_node, node_id);
        }

        // Check if workflow completed
        let instance = self
            .instances
            .get(instance_id)
            .ok_or_else(|| WorkflowError::InstanceNotFound(instance_id.clone()))?;
        if instance.is_terminal() {
            self.provenance.record_completed(instance_id);
            tracing::info!(
                instance_id = %instance_id,
                "Workflow completed"
            );
        }

        Ok(activated)
    }

    // ── Node Completion (without receipt) ────────────────────────────

    /// Complete a node without a receipt (for control flow nodes).
    pub fn complete_node(
        &mut self,
        instance_id: &WorkflowInstanceId,
        node_id: &NodeId,
    ) -> WorkflowResult<Vec<NodeId>> {
        let instance = self.get_instance(instance_id)?;
        if instance.is_terminal() {
            return Err(WorkflowError::AlreadyCompleted);
        }
        let def_id = instance.definition_id.clone();
        let definition = self.definitions.get(&def_id)?.clone();

        {
            let instance = self
                .instances
                .get_mut(instance_id)
                .ok_or_else(|| WorkflowError::InstanceNotFound(instance_id.clone()))?;
            instance.complete_node(node_id);
        }

        self.provenance
            .record_node_completed(instance_id, node_id, None);

        let instance = self
            .instances
            .get_mut(instance_id)
            .ok_or_else(|| WorkflowError::InstanceNotFound(instance_id.clone()))?;
        let activated = self
            .state_machine
            .advance_from_node(instance, &definition, node_id)?;

        for activated_node in &activated {
            self.provenance
                .record_node_activated(instance_id, activated_node);
            self.provenance
                .record_transition(instance_id, node_id, activated_node);
        }

        Ok(activated)
    }

    // ── Escalation ───────────────────────────────────────────────────

    /// Check all active instances for escalation conditions.
    ///
    /// Returns escalation decisions that the caller should act upon.
    pub fn check_escalations(
        &self,
    ) -> Vec<(
        WorkflowInstanceId,
        crate::escalation_handler::EscalationDecision,
    )> {
        let mut all_decisions = Vec::new();

        for (id, instance) in &self.instances {
            if !instance.is_active() {
                continue;
            }

            let def_id = &instance.definition_id;
            if let Ok(definition) = self.definitions.get(def_id) {
                // Check node-level escalations
                let decisions = self
                    .escalation_handler
                    .check_escalations(instance, definition);
                for decision in decisions {
                    all_decisions.push((id.clone(), decision));
                }

                // Check workflow-level timeout
                if let Some(decision) = self
                    .escalation_handler
                    .check_workflow_timeout(instance, definition)
                {
                    all_decisions.push((id.clone(), decision));
                }
            }
        }

        all_decisions
    }

    /// Apply an escalation action to an instance
    pub fn apply_escalation(
        &mut self,
        instance_id: &WorkflowInstanceId,
        node_id: &NodeId,
        action: &RecommendedAction,
    ) -> WorkflowResult<()> {
        let instance = self.get_instance(instance_id)?;
        let def_id = instance.definition_id.clone();

        match action {
            RecommendedAction::RetryNode => {
                let instance = self.get_instance_mut(instance_id)?;
                if let Some(state) = instance.node_states.get_mut(node_id) {
                    state.retry_count += 1;
                    state.escalation.record_retry();
                    state.status = NodeStatus::Active;
                    state.activated_at = Some(chrono::Utc::now());
                }
                self.provenance
                    .record_escalation(instance_id, node_id, "Retrying node");
            }

            RecommendedAction::SkipNode => {
                {
                    let instance = self
                        .instances
                        .get_mut(instance_id)
                        .ok_or_else(|| WorkflowError::InstanceNotFound(instance_id.clone()))?;
                    instance.skip_node(node_id);
                }

                let definition = self.definitions.get(&def_id)?.clone();
                let instance = self
                    .instances
                    .get_mut(instance_id)
                    .ok_or_else(|| WorkflowError::InstanceNotFound(instance_id.clone()))?;
                self.state_machine
                    .advance_from_node(instance, &definition, node_id)?;

                self.provenance.record_escalation(
                    instance_id,
                    node_id,
                    "Node skipped via escalation",
                );
            }

            RecommendedAction::AbortWorkflow { reason } => {
                let instance = self.get_instance_mut(instance_id)?;
                instance.fail(reason);
                self.provenance.record_failed(instance_id, reason);
            }

            RecommendedAction::PauseWorkflow { reason } => {
                let instance = self.get_instance_mut(instance_id)?;
                instance.pause(reason);
            }

            RecommendedAction::EscalateToRole { role } => {
                self.provenance.record_escalation(
                    instance_id,
                    node_id,
                    &format!("Escalated to role '{}'", role),
                );
            }

            RecommendedAction::RedirectToNode { node_id: target } => {
                let instance = self.get_instance_mut(instance_id)?;
                instance.skip_node(node_id);
                instance.activate_node(target.clone());
                self.provenance.record_escalation(
                    instance_id,
                    node_id,
                    &format!("Redirected to node '{}'", target),
                );
            }

            RecommendedAction::Compensate { description } => {
                self.provenance.record_escalation(
                    instance_id,
                    node_id,
                    &format!("Compensation: {}", description),
                );
            }
        }

        Ok(())
    }

    // ── Workflow Control ─────────────────────────────────────────────

    /// Pause an active workflow
    pub fn pause_instance(
        &mut self,
        instance_id: &WorkflowInstanceId,
        reason: impl Into<String>,
    ) -> WorkflowResult<()> {
        let instance = self.get_instance_mut(instance_id)?;
        if !instance.is_active() {
            return Err(WorkflowError::NotActive);
        }
        instance.pause(reason);
        Ok(())
    }

    /// Resume a paused workflow
    pub fn resume_instance(&mut self, instance_id: &WorkflowInstanceId) -> WorkflowResult<()> {
        let instance = self.get_instance_mut(instance_id)?;
        if instance.state != WorkflowState::Paused {
            return Err(WorkflowError::NotActive);
        }
        instance.resume();
        Ok(())
    }

    /// Cancel a workflow
    pub fn cancel_instance(
        &mut self,
        instance_id: &WorkflowInstanceId,
        reason: impl Into<String>,
    ) -> WorkflowResult<()> {
        let reason = reason.into();
        let instance = self.get_instance_mut(instance_id)?;
        if instance.is_terminal() {
            return Err(WorkflowError::AlreadyCompleted);
        }
        instance.cancel(&reason);
        self.provenance.record_failed(instance_id, &reason);
        Ok(())
    }

    // ── Role Management ──────────────────────────────────────────────

    /// Assign a resonator to a role in a workflow instance
    pub fn assign_role(
        &mut self,
        instance_id: &WorkflowInstanceId,
        role: collective_types::RoleId,
        resonator: ResonatorId,
    ) -> WorkflowResult<()> {
        let instance = self.get_instance_mut(instance_id)?;
        instance.assign_role(role, resonator);
        Ok(())
    }

    /// Assign a resonator to a specific node in a workflow instance
    pub fn assign_to_node(
        &mut self,
        instance_id: &WorkflowInstanceId,
        node_id: &NodeId,
        resonator: ResonatorId,
    ) -> WorkflowResult<()> {
        let instance = self.get_instance_mut(instance_id)?;
        instance.assign_resonator_to_node(node_id, resonator);
        Ok(())
    }

    // ── Query ────────────────────────────────────────────────────────

    /// Get the collective this orchestrator belongs to
    pub fn collective_id(&self) -> &CollectiveId {
        &self.collective_id
    }

    /// Get provenance for an instance
    pub fn provenance_for(
        &self,
        instance_id: &WorkflowInstanceId,
    ) -> Vec<&crate::provenance_tracker::ProvenanceRecord> {
        self.provenance.events_for(instance_id)
    }

    /// Get completed workflow records
    pub fn completed_workflows(&self) -> &[CompletedWorkflow] {
        &self.completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collective_types::ReceiptType;

    fn make_orchestrator() -> WorkflowOrchestrator {
        WorkflowOrchestrator::new(CollectiveId::new("test-collective"))
    }

    fn make_simple_definition() -> WorkflowDefinition {
        let mut def = WorkflowDefinition::new(
            "Simple Review",
            CollectiveId::new("test-collective"),
            ResonatorId::new("author"),
        );
        def.add_node(WorkflowNode::start("start")).unwrap();
        def.add_node(WorkflowNode::action("review", "Review Document").with_timeout(3600))
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

    #[test]
    fn test_register_and_launch() {
        let mut orch = make_orchestrator();
        let def = make_simple_definition();
        let def_id = orch.register_definition(def).unwrap();

        assert_eq!(orch.definition_count(), 1);

        let inst_id = orch
            .launch_instance(&def_id, ResonatorId::new("initiator"))
            .unwrap();

        let inst = orch.get_instance(&inst_id).unwrap();
        assert!(inst.is_active());
        assert_eq!(inst.active_nodes().len(), 1);
        assert_eq!(orch.instance_count(), 1);
    }

    #[test]
    fn test_submit_receipt_completes_workflow() {
        let mut orch = make_orchestrator();
        let def = make_simple_definition();
        let def_id = orch.register_definition(def).unwrap();
        let inst_id = orch
            .launch_instance(&def_id, ResonatorId::new("initiator"))
            .unwrap();

        // Submit receipt for review node
        let receipt = WorkflowReceipt::new(
            NodeId::new("review"),
            ReceiptType::CommitmentFulfilled,
            ResonatorId::new("reviewer-1"),
        );
        let activated = orch.submit_receipt(&inst_id, receipt).unwrap();

        // Should have activated end node
        assert!(activated.contains(&NodeId::new("end")));

        // Workflow should be completed
        let inst = orch.get_instance(&inst_id).unwrap();
        assert!(inst.is_terminal());
        assert_eq!(inst.state, WorkflowState::Completed);
    }

    #[test]
    fn test_wrong_receipt_doesnt_advance() {
        let mut orch = make_orchestrator();
        let def = make_simple_definition();
        let def_id = orch.register_definition(def).unwrap();
        let inst_id = orch
            .launch_instance(&def_id, ResonatorId::new("initiator"))
            .unwrap();

        // Submit wrong receipt type
        let receipt = WorkflowReceipt::new(
            NodeId::new("review"),
            ReceiptType::CommitmentBroken, // Wrong type!
            ResonatorId::new("reviewer-1"),
        );
        let activated = orch.submit_receipt(&inst_id, receipt).unwrap();

        // Should NOT have activated end node (gate requires CommitmentFulfilled)
        assert!(!activated.contains(&NodeId::new("end")));

        // Workflow still active (review completed but can't transition)
        let inst = orch.get_instance(&inst_id).unwrap();
        assert!(inst.is_active() || !inst.is_terminal());
    }

    #[test]
    fn test_launch_nonexistent_definition() {
        let mut orch = make_orchestrator();
        let result = orch.launch_instance(
            &WorkflowDefinitionId::new("nonexistent"),
            ResonatorId::new("init"),
        );
        assert!(matches!(result, Err(WorkflowError::DefinitionNotFound(_))));
    }

    #[test]
    fn test_submit_receipt_to_completed_workflow() {
        let mut orch = make_orchestrator();
        let def = make_simple_definition();
        let def_id = orch.register_definition(def).unwrap();
        let inst_id = orch
            .launch_instance(&def_id, ResonatorId::new("init"))
            .unwrap();

        // Complete the workflow
        let receipt = WorkflowReceipt::new(
            NodeId::new("review"),
            ReceiptType::CommitmentFulfilled,
            ResonatorId::new("r1"),
        );
        orch.submit_receipt(&inst_id, receipt).unwrap();

        // Try to submit another receipt
        let receipt2 = WorkflowReceipt::new(
            NodeId::new("review"),
            ReceiptType::Audit,
            ResonatorId::new("r2"),
        );
        let result = orch.submit_receipt(&inst_id, receipt2);
        assert!(matches!(result, Err(WorkflowError::AlreadyCompleted)));
    }

    #[test]
    fn test_pause_resume() {
        let mut orch = make_orchestrator();
        let def = make_simple_definition();
        let def_id = orch.register_definition(def).unwrap();
        let inst_id = orch
            .launch_instance(&def_id, ResonatorId::new("init"))
            .unwrap();

        orch.pause_instance(&inst_id, "Need approval").unwrap();
        let inst = orch.get_instance(&inst_id).unwrap();
        assert_eq!(inst.state, WorkflowState::Paused);

        orch.resume_instance(&inst_id).unwrap();
        let inst = orch.get_instance(&inst_id).unwrap();
        assert!(inst.is_active());
    }

    #[test]
    fn test_cancel() {
        let mut orch = make_orchestrator();
        let def = make_simple_definition();
        let def_id = orch.register_definition(def).unwrap();
        let inst_id = orch
            .launch_instance(&def_id, ResonatorId::new("init"))
            .unwrap();

        orch.cancel_instance(&inst_id, "No longer needed").unwrap();
        let inst = orch.get_instance(&inst_id).unwrap();
        assert!(inst.is_terminal());
        assert_eq!(inst.state, WorkflowState::Cancelled);
    }

    #[test]
    fn test_assign_role() {
        let mut orch = make_orchestrator();
        let def = make_simple_definition();
        let def_id = orch.register_definition(def).unwrap();
        let inst_id = orch
            .launch_instance(&def_id, ResonatorId::new("init"))
            .unwrap();

        orch.assign_role(
            &inst_id,
            collective_types::RoleId::new("reviewer"),
            ResonatorId::new("reviewer-1"),
        )
        .unwrap();

        let inst = orch.get_instance(&inst_id).unwrap();
        let reviewers = inst.resonators_for_role(&collective_types::RoleId::new("reviewer"));
        assert_eq!(reviewers.len(), 1);
    }

    #[test]
    fn test_provenance_tracking() {
        let mut orch = make_orchestrator();
        let def = make_simple_definition();
        let def_id = orch.register_definition(def).unwrap();
        let inst_id = orch
            .launch_instance(&def_id, ResonatorId::new("init"))
            .unwrap();

        // Submit receipt to complete
        let receipt = WorkflowReceipt::new(
            NodeId::new("review"),
            ReceiptType::CommitmentFulfilled,
            ResonatorId::new("r1"),
        );
        orch.submit_receipt(&inst_id, receipt).unwrap();

        let provenance = orch.provenance_for(&inst_id);
        assert!(provenance.len() >= 4); // started, node_activated, receipt, completed
    }

    #[test]
    fn test_active_instances() {
        let mut orch = make_orchestrator();
        let def = make_simple_definition();
        let def_id = orch.register_definition(def).unwrap();

        orch.launch_instance(&def_id, ResonatorId::new("init-1"))
            .unwrap();
        let inst_id_2 = orch
            .launch_instance(&def_id, ResonatorId::new("init-2"))
            .unwrap();

        assert_eq!(orch.active_instances().len(), 2);

        // Complete one
        let receipt = WorkflowReceipt::new(
            NodeId::new("review"),
            ReceiptType::CommitmentFulfilled,
            ResonatorId::new("r1"),
        );
        orch.submit_receipt(&inst_id_2, receipt).unwrap();

        assert_eq!(orch.active_instances().len(), 1);
    }

    #[test]
    fn test_launch_with_params() {
        let mut orch = make_orchestrator();
        let def = make_simple_definition();
        let def_id = orch.register_definition(def).unwrap();

        let mut params = HashMap::new();
        params.insert("document_id".to_string(), "doc-123".to_string());

        let inst_id = orch
            .launch_instance_with_params(&def_id, ResonatorId::new("init"), params)
            .unwrap();

        let inst = orch.get_instance(&inst_id).unwrap();
        assert_eq!(inst.parameters.get("document_id").unwrap(), "doc-123");
    }

    #[test]
    fn test_apply_abort_escalation() {
        let mut orch = make_orchestrator();
        let def = make_simple_definition();
        let def_id = orch.register_definition(def).unwrap();
        let inst_id = orch
            .launch_instance(&def_id, ResonatorId::new("init"))
            .unwrap();

        orch.apply_escalation(
            &inst_id,
            &NodeId::new("review"),
            &RecommendedAction::AbortWorkflow {
                reason: "Timed out".into(),
            },
        )
        .unwrap();

        let inst = orch.get_instance(&inst_id).unwrap();
        assert!(inst.is_terminal());
        assert_eq!(inst.state, WorkflowState::Failed);
    }

    #[test]
    fn test_apply_pause_escalation() {
        let mut orch = make_orchestrator();
        let def = make_simple_definition();
        let def_id = orch.register_definition(def).unwrap();
        let inst_id = orch
            .launch_instance(&def_id, ResonatorId::new("init"))
            .unwrap();

        orch.apply_escalation(
            &inst_id,
            &NodeId::new("review"),
            &RecommendedAction::PauseWorkflow {
                reason: "Needs manual review".into(),
            },
        )
        .unwrap();

        let inst = orch.get_instance(&inst_id).unwrap();
        assert_eq!(inst.state, WorkflowState::Paused);
    }

    #[test]
    fn test_complete_node_directly() {
        let mut orch = make_orchestrator();

        // Create a definition with automatic edges (no receipt gates)
        let mut def = WorkflowDefinition::new(
            "Auto",
            CollectiveId::new("test-collective"),
            ResonatorId::new("author"),
        );
        def.add_node(WorkflowNode::start("start")).unwrap();
        def.add_node(WorkflowNode::action("task", "Task")).unwrap();
        def.add_node(WorkflowNode::end("end")).unwrap();
        def.add_edge(WorkflowEdge::new(NodeId::new("start"), NodeId::new("task")))
            .unwrap();
        def.add_edge(WorkflowEdge::new(NodeId::new("task"), NodeId::new("end")))
            .unwrap();

        let def_id = orch.register_definition(def).unwrap();
        let inst_id = orch
            .launch_instance(&def_id, ResonatorId::new("init"))
            .unwrap();

        // Complete task directly (no receipt needed for automatic edge)
        let activated = orch.complete_node(&inst_id, &NodeId::new("task")).unwrap();

        assert!(activated.contains(&NodeId::new("end")));
        let inst = orch.get_instance(&inst_id).unwrap();
        assert!(inst.is_terminal());
    }

    #[test]
    fn test_collective_id() {
        let orch = make_orchestrator();
        assert_eq!(orch.collective_id(), &CollectiveId::new("test-collective"));
    }
}
