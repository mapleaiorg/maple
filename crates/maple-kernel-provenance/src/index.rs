use std::collections::{HashMap, HashSet, VecDeque};

use maple_kernel_fabric::{EventPayload, KernelEvent, ResonanceStage};
use maple_mwl_types::{CommitmentId, EventId, TemporalAnchor, WorldlineId};
use tracing::{debug, info};

use crate::checkpoint::{Checkpoint, CheckpointRef};
use crate::error::ProvenanceError;
use crate::node::ProvenanceNode;
use crate::reports::{ContagionReport, ImpactReport};

/// Provenance Index — causal DAG of all events.
///
/// Per I.4 (Causal Provenance): No event without lineage. Every event
/// references its causal parents.
///
/// The index maintains both forward (parent→child) and backward (child→parent)
/// edges, enabling efficient traversal in both directions.
pub struct ProvenanceIndex {
    nodes: HashMap<EventId, ProvenanceNode>,
    /// Track checkpoints
    checkpoints: Vec<Checkpoint>,
    /// Genesis event IDs (events with no parents, which are allowed)
    genesis_events: HashSet<EventId>,
}

impl ProvenanceIndex {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            checkpoints: Vec::new(),
            genesis_events: HashSet::new(),
        }
    }

    /// Add an event to the provenance index.
    ///
    /// Creates a ProvenanceNode and maintains parent→child edges.
    /// Genesis events (no parents) are only allowed for System-stage events
    /// (e.g., WorldlineCreated). All other events MUST have parents per I.4.
    pub fn add_event(&mut self, event: &KernelEvent) -> Result<(), ProvenanceError> {
        if self.nodes.contains_key(&event.id) {
            return Err(ProvenanceError::DuplicateEvent(event.id.clone()));
        }

        // I.4: No event without lineage (except genesis events)
        let is_genesis = event.parents.is_empty();
        if is_genesis {
            match event.stage {
                ResonanceStage::System | ResonanceStage::Presence => {
                    // System and Presence events can be genesis
                    self.genesis_events.insert(event.id.clone());
                }
                _ => {
                    return Err(ProvenanceError::NoParentsNonGenesis(event.id.clone()));
                }
            }
        }

        // Extract commitment_id and policy_id from payload for indexed queries
        let (commitment_id, policy_id) = extract_metadata(&event.payload);

        // Convert HlcTimestamp to TemporalAnchor
        let timestamp = TemporalAnchor::new(
            event.timestamp.physical,
            event.timestamp.logical,
            event.timestamp.node_id.0,
        );

        let node = ProvenanceNode {
            event_id: event.id.clone(),
            parents: event.parents.clone(),
            children: vec![],
            worldline: event.worldline_id.clone(),
            resonance_stage: event.stage,
            timestamp,
            checkpoint: None,
            commitment_id,
            policy_id,
        };

        // Update parent→child edges
        for parent_id in &event.parents {
            if let Some(parent_node) = self.nodes.get_mut(parent_id) {
                parent_node.children.push(event.id.clone());
            }
            // Note: we allow dangling parent refs for events ingested out of order
            // (e.g., during recovery). The parent may arrive later.
        }

        self.nodes.insert(event.id.clone(), node);
        debug!(event_id = %event.id, parents = event.parents.len(), "Indexed provenance node");

        Ok(())
    }

    // =========================================================================
    // 8 QUERY TYPES
    // =========================================================================

    /// Query 1: Ancestors — all events that causally precede this event.
    ///
    /// Walks backward through parent edges. Optional depth limit.
    pub fn ancestors(&self, id: &EventId, depth: Option<u32>) -> Vec<&ProvenanceNode> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        if let Some(node) = self.nodes.get(id) {
            for parent_id in &node.parents {
                queue.push_back((parent_id.clone(), 1u32));
            }
        }

        while let Some((current_id, current_depth)) = queue.pop_front() {
            if let Some(max_depth) = depth {
                if current_depth > max_depth {
                    continue;
                }
            }
            if !visited.insert(current_id.clone()) {
                continue;
            }
            if let Some(node) = self.nodes.get(&current_id) {
                result.push(node);
                for parent_id in &node.parents {
                    queue.push_back((parent_id.clone(), current_depth + 1));
                }
            }
        }

        result
    }

    /// Query 2: Descendants — all events causally downstream of this event.
    ///
    /// Walks forward through child edges. Optional depth limit.
    pub fn descendants(&self, id: &EventId, depth: Option<u32>) -> Vec<&ProvenanceNode> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        if let Some(node) = self.nodes.get(id) {
            for child_id in &node.children {
                queue.push_back((child_id.clone(), 1u32));
            }
        }

        while let Some((current_id, current_depth)) = queue.pop_front() {
            if let Some(max_depth) = depth {
                if current_depth > max_depth {
                    continue;
                }
            }
            if !visited.insert(current_id.clone()) {
                continue;
            }
            if let Some(node) = self.nodes.get(&current_id) {
                result.push(node);
                for child_id in &node.children {
                    queue.push_back((child_id.clone(), current_depth + 1));
                }
            }
        }

        result
    }

    /// Query 3: Causal path — find a path from one event to another.
    ///
    /// Uses BFS through child edges to find the shortest causal path.
    /// Returns None if no causal path exists.
    pub fn causal_path(&self, from: &EventId, to: &EventId) -> Option<Vec<EventId>> {
        if from == to {
            return Some(vec![from.clone()]);
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        // Track the parent in the BFS tree for path reconstruction
        let mut came_from: HashMap<EventId, EventId> = HashMap::new();

        visited.insert(from.clone());
        queue.push_back(from.clone());

        while let Some(current_id) = queue.pop_front() {
            if let Some(node) = self.nodes.get(&current_id) {
                for child_id in &node.children {
                    if !visited.insert(child_id.clone()) {
                        continue;
                    }
                    came_from.insert(child_id.clone(), current_id.clone());

                    if child_id == to {
                        // Reconstruct path
                        let mut path = vec![to.clone()];
                        let mut cur = to.clone();
                        while let Some(prev) = came_from.get(&cur) {
                            path.push(prev.clone());
                            cur = prev.clone();
                        }
                        path.reverse();
                        return Some(path);
                    }

                    queue.push_back(child_id.clone());
                }
            }
        }

        None
    }

    /// Query 4: Audit trail — full provenance chain for a commitment.
    ///
    /// Returns all events related to a specific commitment_id,
    /// ordered by timestamp.
    pub fn audit_trail(&self, commitment_id: &CommitmentId) -> Vec<&ProvenanceNode> {
        let mut nodes: Vec<&ProvenanceNode> = self
            .nodes
            .values()
            .filter(|n| n.commitment_id.as_ref() == Some(commitment_id))
            .collect();

        nodes.sort_by_key(|n| n.timestamp);
        nodes
    }

    /// Query 5: WorldLine history — all events from a specific worldline.
    ///
    /// Optional time range filter.
    pub fn worldline_history(
        &self,
        wid: &WorldlineId,
        range: Option<(TemporalAnchor, TemporalAnchor)>,
    ) -> Vec<&ProvenanceNode> {
        let mut nodes: Vec<&ProvenanceNode> = self
            .nodes
            .values()
            .filter(|n| {
                if n.worldline != *wid {
                    return false;
                }
                if let Some((from, to)) = &range {
                    n.timestamp >= *from && n.timestamp <= *to
                } else {
                    true
                }
            })
            .collect();

        nodes.sort_by_key(|n| n.timestamp);
        nodes
    }

    /// Query 6: Regulatory slice — all events related to a specific policy.
    pub fn regulatory_slice(&self, policy_id: &str) -> Vec<&ProvenanceNode> {
        let mut nodes: Vec<&ProvenanceNode> = self
            .nodes
            .values()
            .filter(|n| n.policy_id.as_deref() == Some(policy_id))
            .collect();

        nodes.sort_by_key(|n| n.timestamp);
        nodes
    }

    /// Query 7: Impact analysis — how an event's consequences rippled.
    pub fn impact_analysis(&self, event_id: &EventId) -> ImpactReport {
        let descendants = self.descendants(event_id, None);

        let mut affected_worldlines: HashSet<WorldlineId> = HashSet::new();
        let mut stage_counts: HashMap<String, usize> = HashMap::new();
        let mut max_depth = 0u32;

        // Compute max depth via BFS
        let mut depth_queue = VecDeque::new();
        let mut depth_visited = HashSet::new();
        if let Some(node) = self.nodes.get(event_id) {
            for child_id in &node.children {
                depth_queue.push_back((child_id.clone(), 1u32));
            }
        }
        while let Some((cid, d)) = depth_queue.pop_front() {
            if !depth_visited.insert(cid.clone()) {
                continue;
            }
            if d > max_depth {
                max_depth = d;
            }
            if let Some(n) = self.nodes.get(&cid) {
                for child_id in &n.children {
                    depth_queue.push_back((child_id.clone(), d + 1));
                }
            }
        }

        for desc in &descendants {
            affected_worldlines.insert(desc.worldline.clone());
            *stage_counts
                .entry(format!("{:?}", desc.resonance_stage))
                .or_insert(0) += 1;
        }

        let stage_breakdown: Vec<(String, usize)> = stage_counts.into_iter().collect();

        ImpactReport {
            event_id: event_id.clone(),
            total_descendants: descendants.len(),
            affected_worldlines: affected_worldlines.into_iter().collect(),
            stage_breakdown,
            max_depth,
        }
    }

    /// Query 8: Risk contagion — causal connections of a worldline.
    pub fn risk_contagion(&self, wid: &WorldlineId) -> ContagionReport {
        let worldline_events: Vec<&ProvenanceNode> = self
            .nodes
            .values()
            .filter(|n| n.worldline == *wid)
            .collect();

        let mut downstream: HashSet<WorldlineId> = HashSet::new();
        let mut upstream: HashSet<WorldlineId> = HashSet::new();
        let mut highest_stage: Option<ResonanceStage> = None;

        for node in &worldline_events {
            // Downstream: descendants from other worldlines
            let descs = self.descendants(&node.event_id, Some(3));
            for desc in &descs {
                if desc.worldline != *wid {
                    downstream.insert(desc.worldline.clone());
                }
            }

            // Upstream: ancestors from other worldlines
            let ancs = self.ancestors(&node.event_id, Some(3));
            for anc in &ancs {
                if anc.worldline != *wid {
                    upstream.insert(anc.worldline.clone());
                }
            }

            // Track highest resonance stage
            let stage_ord = match node.resonance_stage {
                ResonanceStage::Presence => 0,
                ResonanceStage::Coupling => 1,
                ResonanceStage::Meaning => 2,
                ResonanceStage::Intent => 3,
                ResonanceStage::Commitment => 4,
                ResonanceStage::Consequence => 5,
                ResonanceStage::Governance => 6,
                ResonanceStage::System => 7,
            };
            let current_max = highest_stage.map(|s| match s {
                ResonanceStage::Presence => 0,
                ResonanceStage::Coupling => 1,
                ResonanceStage::Meaning => 2,
                ResonanceStage::Intent => 3,
                ResonanceStage::Commitment => 4,
                ResonanceStage::Consequence => 5,
                ResonanceStage::Governance => 6,
                ResonanceStage::System => 7,
            }).unwrap_or(0);
            if stage_ord >= current_max {
                highest_stage = Some(node.resonance_stage);
            }
        }

        let total_connections = downstream.len() + upstream.len();

        ContagionReport {
            worldline: wid.clone(),
            downstream_worldlines: downstream.into_iter().collect(),
            upstream_worldlines: upstream.into_iter().collect(),
            total_connections,
            highest_stage,
        }
    }

    // =========================================================================
    // CHECKPOINT
    // =========================================================================

    /// Create a checkpoint: compress events before a given temporal anchor.
    ///
    /// Per I.PVP-1 (Compression Integrity): Checkpoint compression preserves
    /// causal linkage. Boundary events (those with children outside the
    /// checkpoint range) are preserved.
    pub fn checkpoint(
        &mut self,
        before: &TemporalAnchor,
    ) -> Result<CheckpointRef, ProvenanceError> {
        // Find all events before the timestamp
        let events_to_compress: Vec<EventId> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.timestamp < *before && n.checkpoint.is_none())
            .map(|(id, _)| id.clone())
            .collect();

        if events_to_compress.is_empty() {
            return Err(ProvenanceError::CheckpointError(
                "No events to compress".into(),
            ));
        }

        let compress_set: HashSet<EventId> = events_to_compress.iter().cloned().collect();

        // Identify boundary events: events in the compress set that have
        // children outside the compress set
        let mut boundary_events = Vec::new();
        for eid in &events_to_compress {
            if let Some(node) = self.nodes.get(eid) {
                let has_external_child = node
                    .children
                    .iter()
                    .any(|child| !compress_set.contains(child));
                if has_external_child {
                    boundary_events.push(eid.clone());
                }
            }
        }

        let checkpoint_id = uuid::Uuid::new_v4();
        let checkpoint_ref = CheckpointRef {
            checkpoint_id,
            created_at: TemporalAnchor::now(0),
            event_count: events_to_compress.len(),
        };

        let checkpoint = Checkpoint {
            id: checkpoint_id,
            before: *before,
            boundary_events: boundary_events.clone(),
            compressed_count: events_to_compress.len(),
            created_at: TemporalAnchor::now(0),
        };

        // Mark compressed events with checkpoint reference
        // Remove non-boundary events, keep boundary events
        let non_boundary: HashSet<EventId> = compress_set
            .difference(&boundary_events.iter().cloned().collect::<HashSet<_>>())
            .cloned()
            .collect();

        // Mark all compressed events
        for eid in &events_to_compress {
            if let Some(node) = self.nodes.get_mut(eid) {
                node.checkpoint = Some(checkpoint_ref.clone());
            }
        }

        // Remove non-boundary nodes (boundary nodes stay for causal linkage)
        for eid in &non_boundary {
            self.nodes.remove(eid);
        }

        self.checkpoints.push(checkpoint);

        info!(
            checkpoint_id = %checkpoint_id,
            compressed = events_to_compress.len(),
            boundary = boundary_events.len(),
            "Checkpoint created"
        );

        Ok(checkpoint_ref)
    }

    // =========================================================================
    // UTILITY
    // =========================================================================

    /// Get a specific node.
    pub fn get(&self, id: &EventId) -> Option<&ProvenanceNode> {
        self.nodes.get(id)
    }

    /// Total number of nodes currently in the index.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Is the index empty?
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Number of checkpoints created.
    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    /// Get all checkpoints.
    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }
}

impl Default for ProvenanceIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract commitment_id and policy_id from event payload for indexed queries.
fn extract_metadata(payload: &EventPayload) -> (Option<CommitmentId>, Option<String>) {
    match payload {
        EventPayload::CommitmentDeclared { commitment_id, .. }
        | EventPayload::CommitmentApproved { commitment_id, .. }
        | EventPayload::CommitmentDenied { commitment_id, .. }
        | EventPayload::CommitmentFulfilled { commitment_id }
        | EventPayload::CommitmentFailed { commitment_id, .. }
        | EventPayload::ConsequenceObserved { commitment_id, .. } => {
            (Some(commitment_id.clone()), None)
        }
        EventPayload::PolicyEvaluated { policy_id, .. } => (None, Some(policy_id.clone())),
        EventPayload::InvariantChecked { invariant_id, .. } => {
            (None, Some(invariant_id.clone()))
        }
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_kernel_fabric::hlc::HlcTimestamp;
    use maple_mwl_types::{IdentityMaterial, NodeId};

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn other_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]))
    }

    fn make_event(
        parents: Vec<EventId>,
        stage: ResonanceStage,
        wid: WorldlineId,
        time_ms: u64,
    ) -> KernelEvent {
        KernelEvent::new(
            EventId::new(),
            HlcTimestamp {
                physical: time_ms,
                logical: 0,
                node_id: NodeId(1),
            },
            wid,
            stage,
            EventPayload::MeaningFormed {
                interpretation_count: 1,
                confidence: 0.9,
                ambiguity_preserved: true,
            },
            parents,
        )
    }

    fn make_commitment_event(
        parents: Vec<EventId>,
        wid: WorldlineId,
        time_ms: u64,
        cid: CommitmentId,
    ) -> KernelEvent {
        KernelEvent::new(
            EventId::new(),
            HlcTimestamp {
                physical: time_ms,
                logical: 0,
                node_id: NodeId(1),
            },
            wid,
            ResonanceStage::Commitment,
            EventPayload::CommitmentDeclared {
                commitment_id: cid,
                scope: serde_json::json!({}),
                parties: vec![],
            },
            parents,
        )
    }

    fn make_genesis(wid: WorldlineId, time_ms: u64) -> KernelEvent {
        KernelEvent::new(
            EventId::new(),
            HlcTimestamp {
                physical: time_ms,
                logical: 0,
                node_id: NodeId(1),
            },
            wid,
            ResonanceStage::System,
            EventPayload::WorldlineCreated {
                profile: "test".into(),
            },
            vec![],
        )
    }

    fn make_policy_event(
        parents: Vec<EventId>,
        wid: WorldlineId,
        time_ms: u64,
        policy_id: &str,
    ) -> KernelEvent {
        KernelEvent::new(
            EventId::new(),
            HlcTimestamp {
                physical: time_ms,
                logical: 0,
                node_id: NodeId(1),
            },
            wid,
            ResonanceStage::Governance,
            EventPayload::PolicyEvaluated {
                policy_id: policy_id.into(),
                result: "pass".into(),
            },
            parents,
        )
    }

    // =========================================================================
    // DAG Formation
    // =========================================================================

    #[test]
    fn dag_formation_from_event_chain() {
        let mut index = ProvenanceIndex::new();
        let wid = test_worldline();

        let genesis = make_genesis(wid.clone(), 100);
        index.add_event(&genesis).unwrap();

        let e1 = make_event(vec![genesis.id.clone()], ResonanceStage::Meaning, wid.clone(), 200);
        index.add_event(&e1).unwrap();

        let e2 = make_event(vec![e1.id.clone()], ResonanceStage::Intent, wid.clone(), 300);
        index.add_event(&e2).unwrap();

        assert_eq!(index.len(), 3);

        // Check parent→child links
        let genesis_node = index.get(&genesis.id).unwrap();
        assert_eq!(genesis_node.children.len(), 1);
        assert_eq!(genesis_node.children[0], e1.id);

        let e1_node = index.get(&e1.id).unwrap();
        assert_eq!(e1_node.parents.len(), 1);
        assert_eq!(e1_node.children.len(), 1);
    }

    #[test]
    fn reject_duplicate_event() {
        let mut index = ProvenanceIndex::new();
        let genesis = make_genesis(test_worldline(), 100);
        index.add_event(&genesis).unwrap();
        assert!(index.add_event(&genesis).is_err());
    }

    #[test]
    fn reject_non_genesis_without_parents() {
        let mut index = ProvenanceIndex::new();
        let event = make_event(vec![], ResonanceStage::Meaning, test_worldline(), 100);
        assert!(matches!(
            index.add_event(&event),
            Err(ProvenanceError::NoParentsNonGenesis(_))
        ));
    }

    #[test]
    fn genesis_events_allowed_without_parents() {
        let mut index = ProvenanceIndex::new();
        let genesis = make_genesis(test_worldline(), 100);
        assert!(index.add_event(&genesis).is_ok());
        assert!(index.get(&genesis.id).unwrap().is_genesis());
    }

    // =========================================================================
    // Query 1: Ancestors
    // =========================================================================

    #[test]
    fn ancestors_returns_correct_subgraph() {
        let mut index = ProvenanceIndex::new();
        let wid = test_worldline();

        let g = make_genesis(wid.clone(), 100);
        index.add_event(&g).unwrap();

        let e1 = make_event(vec![g.id.clone()], ResonanceStage::Meaning, wid.clone(), 200);
        index.add_event(&e1).unwrap();

        let e2 = make_event(vec![e1.id.clone()], ResonanceStage::Intent, wid.clone(), 300);
        index.add_event(&e2).unwrap();

        let e3 = make_event(vec![e2.id.clone()], ResonanceStage::Commitment, wid.clone(), 400);
        index.add_event(&e3).unwrap();

        let ancestors = index.ancestors(&e3.id, None);
        assert_eq!(ancestors.len(), 3); // e2, e1, g

        // With depth limit
        let ancestors_1 = index.ancestors(&e3.id, Some(1));
        assert_eq!(ancestors_1.len(), 1); // only e2
    }

    // =========================================================================
    // Query 2: Descendants
    // =========================================================================

    #[test]
    fn descendants_returns_correct_subgraph() {
        let mut index = ProvenanceIndex::new();
        let wid = test_worldline();

        let g = make_genesis(wid.clone(), 100);
        index.add_event(&g).unwrap();

        let e1 = make_event(vec![g.id.clone()], ResonanceStage::Meaning, wid.clone(), 200);
        index.add_event(&e1).unwrap();

        let e2 = make_event(vec![e1.id.clone()], ResonanceStage::Intent, wid.clone(), 300);
        index.add_event(&e2).unwrap();

        let descendants = index.descendants(&g.id, None);
        assert_eq!(descendants.len(), 2); // e1, e2

        let descendants_1 = index.descendants(&g.id, Some(1));
        assert_eq!(descendants_1.len(), 1); // only e1
    }

    // =========================================================================
    // Query 3: Causal Path
    // =========================================================================

    #[test]
    fn causal_path_finds_shortest_path() {
        let mut index = ProvenanceIndex::new();
        let wid = test_worldline();

        let g = make_genesis(wid.clone(), 100);
        index.add_event(&g).unwrap();

        let e1 = make_event(vec![g.id.clone()], ResonanceStage::Meaning, wid.clone(), 200);
        index.add_event(&e1).unwrap();

        let e2 = make_event(vec![e1.id.clone()], ResonanceStage::Intent, wid.clone(), 300);
        index.add_event(&e2).unwrap();

        let e3 = make_event(vec![e2.id.clone()], ResonanceStage::Commitment, wid.clone(), 400);
        index.add_event(&e3).unwrap();

        let path = index.causal_path(&g.id, &e3.id).unwrap();
        assert_eq!(path.len(), 4); // g → e1 → e2 → e3
        assert_eq!(path[0], g.id);
        assert_eq!(path[3], e3.id);
    }

    #[test]
    fn causal_path_returns_none_when_no_path() {
        let mut index = ProvenanceIndex::new();
        let wid = test_worldline();

        let g1 = make_genesis(wid.clone(), 100);
        index.add_event(&g1).unwrap();

        let g2 = make_genesis(wid.clone(), 200);
        index.add_event(&g2).unwrap();

        // No path between unrelated genesis events
        assert!(index.causal_path(&g1.id, &g2.id).is_none());
    }

    #[test]
    fn causal_path_same_event() {
        let mut index = ProvenanceIndex::new();
        let g = make_genesis(test_worldline(), 100);
        index.add_event(&g).unwrap();

        let path = index.causal_path(&g.id, &g.id).unwrap();
        assert_eq!(path.len(), 1);
    }

    // =========================================================================
    // Query 4: Audit Trail
    // =========================================================================

    #[test]
    fn audit_trail_for_commitment() {
        let mut index = ProvenanceIndex::new();
        let wid = test_worldline();
        let cid = CommitmentId::new();

        let g = make_genesis(wid.clone(), 100);
        index.add_event(&g).unwrap();

        let c1 = make_commitment_event(vec![g.id.clone()], wid.clone(), 200, cid.clone());
        index.add_event(&c1).unwrap();

        // Approval event for same commitment
        let c2 = KernelEvent::new(
            EventId::new(),
            HlcTimestamp { physical: 300, logical: 0, node_id: NodeId(1) },
            wid.clone(),
            ResonanceStage::Commitment,
            EventPayload::CommitmentApproved {
                commitment_id: cid.clone(),
                decision_card: serde_json::json!({}),
            },
            vec![c1.id.clone()],
        );
        index.add_event(&c2).unwrap();

        // Unrelated event
        let other = make_event(vec![g.id.clone()], ResonanceStage::Meaning, wid.clone(), 250);
        index.add_event(&other).unwrap();

        let trail = index.audit_trail(&cid);
        assert_eq!(trail.len(), 2); // c1 and c2
        assert!(trail[0].timestamp <= trail[1].timestamp);
    }

    // =========================================================================
    // Query 5: WorldLine History
    // =========================================================================

    #[test]
    fn worldline_history() {
        let mut index = ProvenanceIndex::new();
        let wid1 = test_worldline();
        let wid2 = other_worldline();

        let g1 = make_genesis(wid1.clone(), 100);
        index.add_event(&g1).unwrap();

        let g2 = make_genesis(wid2.clone(), 150);
        index.add_event(&g2).unwrap();

        let e1 = make_event(vec![g1.id.clone()], ResonanceStage::Meaning, wid1.clone(), 200);
        index.add_event(&e1).unwrap();

        let e2 = make_event(vec![g2.id.clone()], ResonanceStage::Intent, wid2.clone(), 250);
        index.add_event(&e2).unwrap();

        let history1 = index.worldline_history(&wid1, None);
        assert_eq!(history1.len(), 2); // g1, e1

        let history2 = index.worldline_history(&wid2, None);
        assert_eq!(history2.len(), 2); // g2, e2
    }

    #[test]
    fn worldline_history_with_time_range() {
        let mut index = ProvenanceIndex::new();
        let wid = test_worldline();

        let g = make_genesis(wid.clone(), 100);
        index.add_event(&g).unwrap();

        let e1 = make_event(vec![g.id.clone()], ResonanceStage::Meaning, wid.clone(), 200);
        index.add_event(&e1).unwrap();

        let e2 = make_event(vec![e1.id.clone()], ResonanceStage::Intent, wid.clone(), 300);
        index.add_event(&e2).unwrap();

        let e3 = make_event(vec![e2.id.clone()], ResonanceStage::Commitment, wid.clone(), 400);
        index.add_event(&e3).unwrap();

        let range = (
            TemporalAnchor::new(150, 0, 0),
            TemporalAnchor::new(350, 0, 0),
        );
        let history = index.worldline_history(&wid, Some(range));
        assert_eq!(history.len(), 2); // e1 (200), e2 (300)
    }

    // =========================================================================
    // Query 6: Regulatory Slice
    // =========================================================================

    #[test]
    fn regulatory_slice() {
        let mut index = ProvenanceIndex::new();
        let wid = test_worldline();

        let g = make_genesis(wid.clone(), 100);
        index.add_event(&g).unwrap();

        let p1 = make_policy_event(vec![g.id.clone()], wid.clone(), 200, "POL-001");
        index.add_event(&p1).unwrap();

        let p2 = make_policy_event(vec![p1.id.clone()], wid.clone(), 300, "POL-001");
        index.add_event(&p2).unwrap();

        let p3 = make_policy_event(vec![g.id.clone()], wid.clone(), 250, "POL-002");
        index.add_event(&p3).unwrap();

        let slice = index.regulatory_slice("POL-001");
        assert_eq!(slice.len(), 2);

        let slice2 = index.regulatory_slice("POL-002");
        assert_eq!(slice2.len(), 1);
    }

    // =========================================================================
    // Query 7: Impact Analysis
    // =========================================================================

    #[test]
    fn impact_analysis_report() {
        let mut index = ProvenanceIndex::new();
        let wid1 = test_worldline();
        let wid2 = other_worldline();

        let g = make_genesis(wid1.clone(), 100);
        index.add_event(&g).unwrap();

        let e1 = make_event(vec![g.id.clone()], ResonanceStage::Meaning, wid1.clone(), 200);
        index.add_event(&e1).unwrap();

        let e2 = make_event(vec![e1.id.clone()], ResonanceStage::Intent, wid2.clone(), 300);
        index.add_event(&e2).unwrap();

        let e3 = make_event(vec![e2.id.clone()], ResonanceStage::Commitment, wid1.clone(), 400);
        index.add_event(&e3).unwrap();

        let report = index.impact_analysis(&g.id);
        assert_eq!(report.total_descendants, 3);
        assert_eq!(report.affected_worldlines.len(), 2);
        assert_eq!(report.max_depth, 3);
    }

    // =========================================================================
    // Query 8: Risk Contagion
    // =========================================================================

    #[test]
    fn risk_contagion_report() {
        let mut index = ProvenanceIndex::new();
        let wid1 = test_worldline();
        let wid2 = other_worldline();

        let g1 = make_genesis(wid1.clone(), 100);
        index.add_event(&g1).unwrap();

        let g2 = make_genesis(wid2.clone(), 150);
        index.add_event(&g2).unwrap();

        // wid1 event causes wid2 event
        let e1 = make_event(vec![g1.id.clone()], ResonanceStage::Meaning, wid1.clone(), 200);
        index.add_event(&e1).unwrap();

        let e2 = make_event(vec![e1.id.clone()], ResonanceStage::Intent, wid2.clone(), 300);
        index.add_event(&e2).unwrap();

        let report = index.risk_contagion(&wid1);
        assert!(report.downstream_worldlines.contains(&wid2));
        assert!(report.total_connections > 0);
    }

    // =========================================================================
    // Checkpoint
    // =========================================================================

    #[test]
    fn checkpoint_compression_preserves_causal_paths() {
        let mut index = ProvenanceIndex::new();
        let wid = test_worldline();

        let g = make_genesis(wid.clone(), 100);
        index.add_event(&g).unwrap();

        let e1 = make_event(vec![g.id.clone()], ResonanceStage::Meaning, wid.clone(), 200);
        index.add_event(&e1).unwrap();

        let e2 = make_event(vec![e1.id.clone()], ResonanceStage::Intent, wid.clone(), 300);
        index.add_event(&e2).unwrap();

        let e3 = make_event(vec![e2.id.clone()], ResonanceStage::Commitment, wid.clone(), 400);
        index.add_event(&e3).unwrap();

        let before_len = index.len();
        assert_eq!(before_len, 4);

        // Checkpoint: compress events before timestamp 250
        // This should compress g (100) and e1 (200), keeping e1 as boundary
        // because e1 has child e2 which is outside the range
        let cp = index.checkpoint(&TemporalAnchor::new(250, 0, 0)).unwrap();
        assert!(cp.event_count > 0);

        // e1 should be preserved as boundary (has child e2 outside range)
        // g should be removed (no children outside range... wait, g has child e1
        // which IS in the range, so g is not boundary)
        // Actually: g's child e1 is also in the compress set, so g is non-boundary.
        // e1's child e2 is NOT in the compress set, so e1 IS boundary.

        // After checkpoint: e1 (boundary), e2, e3 should remain
        assert!(index.get(&e1.id).is_some(), "Boundary event e1 should be preserved");
        assert!(index.get(&e2.id).is_some());
        assert!(index.get(&e3.id).is_some());
        assert!(index.get(&g.id).is_none(), "Non-boundary event g should be removed");

        // Causal path from e1 to e3 still works
        let path = index.causal_path(&e1.id, &e3.id).unwrap();
        assert_eq!(path.len(), 3); // e1 → e2 → e3

        assert_eq!(index.checkpoint_count(), 1);
    }

    #[test]
    fn checkpoint_empty_range_errors() {
        let mut index = ProvenanceIndex::new();
        let result = index.checkpoint(&TemporalAnchor::new(100, 0, 0));
        assert!(result.is_err());
    }

    // =========================================================================
    // Fabric Integration
    // =========================================================================

    #[tokio::test]
    async fn integration_with_event_fabric() {
        use maple_kernel_fabric::{EventFabric, FabricConfig};

        let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();
        let mut index = ProvenanceIndex::new();

        let wid = test_worldline();

        // Emit genesis
        let genesis = fabric
            .emit(
                wid.clone(),
                ResonanceStage::System,
                EventPayload::WorldlineCreated { profile: "test".into() },
                vec![],
            )
            .await
            .unwrap();
        index.add_event(&genesis).unwrap();

        // Emit meaning with genesis as parent
        let meaning = fabric
            .emit(
                wid.clone(),
                ResonanceStage::Meaning,
                EventPayload::MeaningFormed {
                    interpretation_count: 1,
                    confidence: 0.9,
                    ambiguity_preserved: true,
                },
                vec![genesis.id.clone()],
            )
            .await
            .unwrap();
        index.add_event(&meaning).unwrap();

        // Emit intent with meaning as parent
        let intent = fabric
            .emit(
                wid.clone(),
                ResonanceStage::Intent,
                EventPayload::IntentStabilized {
                    direction: "forward".into(),
                    confidence: 0.8,
                    conditions: vec![],
                },
                vec![meaning.id.clone()],
            )
            .await
            .unwrap();
        index.add_event(&intent).unwrap();

        assert_eq!(index.len(), 3);

        // Full causal path
        let path = index.causal_path(&genesis.id, &intent.id).unwrap();
        assert_eq!(path.len(), 3);

        // Ancestors of intent
        let ancs = index.ancestors(&intent.id, None);
        assert_eq!(ancs.len(), 2); // meaning, genesis
    }
}
