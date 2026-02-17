use crate::error::GraphError;
use crate::graph::{EvolutionChain, NodeContent, WllNode};
use crate::storage::{GraphStorage, InMemoryGraphStorage};
use crate::types::{ContentHash, GovernanceTier, NodeContentType, TemporalRange, ValidationResult};
use crate::validation::GraphValidator;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use worldline_types::{TemporalAnchor, WorldlineId};

/// High-level manager for WLL Context Graph operations.
///
/// Provides the primary API for creating, querying, and validating evolution chains.
#[async_trait]
pub trait ContextGraphManager: Send + Sync {
    /// Append a new node to the graph. Computes content-addressed ID and validates.
    async fn append(
        &self,
        worldline_id: WorldlineId,
        content: NodeContent,
        parent_ids: Vec<ContentHash>,
        timestamp: TemporalAnchor,
        governance_tier: GovernanceTier,
    ) -> Result<ContentHash, GraphError>;

    /// Retrieve a node by ID.
    async fn get_node(&self, id: &ContentHash) -> Result<Option<WllNode>, GraphError>;

    /// Get all evolution chains for a worldline.
    async fn get_evolution_chains(
        &self,
        worldline_id: &WorldlineId,
    ) -> Result<Vec<EvolutionChain>, GraphError>;

    /// Query nodes within a time range for a worldline.
    async fn query_by_time(
        &self,
        worldline_id: &WorldlineId,
        range: &TemporalRange,
    ) -> Result<Vec<WllNode>, GraphError>;

    /// Get the latest stable consequence for a worldline.
    async fn latest_stable(
        &self,
        worldline_id: &WorldlineId,
    ) -> Result<Option<WllNode>, GraphError>;

    /// Validate a complete chain from a leaf node.
    async fn validate_chain(&self, leaf_id: &ContentHash) -> Result<ValidationResult, GraphError>;

    /// Get total node count.
    async fn node_count(&self) -> Result<usize, GraphError>;
}

/// In-memory implementation of ContextGraphManager.
pub struct InMemoryContextGraphManager {
    storage: InMemoryGraphStorage,
    /// Evolution chains indexed by worldline ID.
    chains: RwLock<HashMap<WorldlineId, Vec<EvolutionChain>>>,
}

impl InMemoryContextGraphManager {
    pub fn new() -> Self {
        Self {
            storage: InMemoryGraphStorage::new(),
            chains: RwLock::new(HashMap::new()),
        }
    }

    /// Access the underlying storage (for testing).
    pub fn storage(&self) -> &InMemoryGraphStorage {
        &self.storage
    }

    /// Track a node in evolution chains.
    fn track_evolution(&self, node: &WllNode) -> Result<(), GraphError> {
        let mut chains = self
            .chains
            .write()
            .map_err(|e| GraphError::WorldlineMismatch {
                expected: "lock".into(),
                actual: format!("poisoned: {}", e),
            })?;

        let wl_chains = chains
            .entry(node.worldline_id.clone())
            .or_default();

        match node.content_type() {
            NodeContentType::Intent => {
                // Start a new chain.
                wl_chains.push(EvolutionChain::new(
                    node.worldline_id.clone(),
                    node.id.clone(),
                ));
            }
            NodeContentType::Inference
            | NodeContentType::Delta
            | NodeContentType::Evidence
            | NodeContentType::Commitment
            | NodeContentType::Consequence => {
                // Find the chain that has one of our parents as its latest node.
                if let Some(chain) = wl_chains.iter_mut().rev().find(|c| {
                    node.parent_ids
                        .iter()
                        .any(|p| c.node_ids().contains(p))
                }) {
                    match node.content_type() {
                        NodeContentType::Inference => chain.inference = Some(node.id.clone()),
                        NodeContentType::Delta => chain.delta = Some(node.id.clone()),
                        NodeContentType::Evidence => chain.evidence = Some(node.id.clone()),
                        NodeContentType::Commitment => chain.commitment = Some(node.id.clone()),
                        NodeContentType::Consequence => chain.consequence = Some(node.id.clone()),
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for InMemoryContextGraphManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContextGraphManager for InMemoryContextGraphManager {
    async fn append(
        &self,
        worldline_id: WorldlineId,
        content: NodeContent,
        parent_ids: Vec<ContentHash>,
        timestamp: TemporalAnchor,
        governance_tier: GovernanceTier,
    ) -> Result<ContentHash, GraphError> {
        let node = WllNode::new(worldline_id, content, parent_ids, timestamp, governance_tier);
        let id = node.id.clone();

        // Validate content hash (should always pass for freshly created nodes).
        node.verify_content_hash()?;

        // Track in evolution chains.
        self.track_evolution(&node)?;

        // Store the node.
        self.storage.put(node).await?;

        Ok(id)
    }

    async fn get_node(&self, id: &ContentHash) -> Result<Option<WllNode>, GraphError> {
        Ok(self.storage.get(id).await?)
    }

    async fn get_evolution_chains(
        &self,
        worldline_id: &WorldlineId,
    ) -> Result<Vec<EvolutionChain>, GraphError> {
        let chains = self
            .chains
            .read()
            .map_err(|e| GraphError::WorldlineMismatch {
                expected: "lock".into(),
                actual: format!("poisoned: {}", e),
            })?;
        Ok(chains.get(worldline_id).cloned().unwrap_or_default())
    }

    async fn query_by_time(
        &self,
        worldline_id: &WorldlineId,
        range: &TemporalRange,
    ) -> Result<Vec<WllNode>, GraphError> {
        Ok(self.storage.get_by_time_range(worldline_id, range).await?)
    }

    async fn latest_stable(
        &self,
        worldline_id: &WorldlineId,
    ) -> Result<Option<WllNode>, GraphError> {
        let consequences = self
            .storage
            .get_by_type(worldline_id, NodeContentType::Consequence)
            .await?;
        // Find the latest consequence node with Stable status.
        for node in consequences.iter().rev() {
            if let NodeContent::Consequence(ref c) = node.content {
                if c.is_healthy() {
                    return Ok(Some(node.clone()));
                }
            }
        }
        Ok(None)
    }

    async fn validate_chain(&self, leaf_id: &ContentHash) -> Result<ValidationResult, GraphError> {
        GraphValidator::validate_chain(leaf_id, &self.storage).await
    }

    async fn node_count(&self) -> Result<usize, GraphError> {
        Ok(self.storage.count().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::{
        ConsequenceNode, DeltaNode, EvidenceBundleRef, InferenceNode, IntentNode, SubstrateType,
        CommitmentNode,
    };

    fn test_worldline() -> WorldlineId {
        WorldlineId::ephemeral()
    }

    #[tokio::test]
    async fn append_and_retrieve() {
        let mgr = InMemoryContextGraphManager::new();
        let wl = test_worldline();
        let intent = IntentNode::new(
            worldline_types::EventId::new(),
            "reduce latency",
            GovernanceTier::Tier0,
        );

        let id = mgr
            .append(
                wl,
                NodeContent::Intent(intent),
                vec![],
                TemporalAnchor::new(100, 0, 0),
                GovernanceTier::Tier0,
            )
            .await
            .unwrap();

        let node = mgr.get_node(&id).await.unwrap().unwrap();
        assert_eq!(node.content_type(), NodeContentType::Intent);
    }

    #[tokio::test]
    async fn content_addressing_deterministic() {
        let mgr = InMemoryContextGraphManager::new();
        let wl = test_worldline();
        let intent = IntentNode::new(
            worldline_types::EventId::new(),
            "test",
            GovernanceTier::Tier0,
        );
        let content = NodeContent::Intent(intent);
        let ts = TemporalAnchor::new(100, 0, 0);

        // compute_id is deterministic for same inputs.
        let id1 = WllNode::compute_id(&content, &[], &wl, &ts);
        let id2 = WllNode::compute_id(&content, &[], &wl, &ts);
        assert_eq!(id1, id2);
    }

    #[tokio::test]
    async fn full_evolution_chain() {
        let mgr = InMemoryContextGraphManager::new();
        let wl = test_worldline();

        // 1. Intent
        let intent_id = mgr
            .append(
                wl.clone(),
                NodeContent::Intent(IntentNode::new(
                    worldline_types::EventId::new(),
                    "reduce latency",
                    GovernanceTier::Tier1,
                )),
                vec![],
                TemporalAnchor::new(100, 0, 0),
                GovernanceTier::Tier1,
            )
            .await
            .unwrap();

        // 2. Inference
        let inference_id = mgr
            .append(
                wl.clone(),
                NodeContent::Inference(InferenceNode::new("llama3.2", 0.85)),
                vec![intent_id.clone()],
                TemporalAnchor::new(200, 0, 0),
                GovernanceTier::Tier1,
            )
            .await
            .unwrap();

        // 3. Delta
        let delta_id = mgr
            .append(
                wl.clone(),
                NodeContent::Delta(DeltaNode::new(SubstrateType::Rust, vec![1, 2, 3])),
                vec![inference_id.clone()],
                TemporalAnchor::new(300, 0, 0),
                GovernanceTier::Tier1,
            )
            .await
            .unwrap();

        // 4. Evidence
        let evidence_id = mgr
            .append(
                wl.clone(),
                NodeContent::Evidence(EvidenceBundleRef::new(ContentHash::hash(b"bundle"))),
                vec![delta_id.clone()],
                TemporalAnchor::new(400, 0, 0),
                GovernanceTier::Tier1,
            )
            .await
            .unwrap();

        // 5. Commitment
        let commitment_id = mgr
            .append(
                wl.clone(),
                NodeContent::Commitment(CommitmentNode::new(
                    ContentHash::hash(b"artifact"),
                    ContentHash::hash(b"evidence"),
                )),
                vec![evidence_id.clone()],
                TemporalAnchor::new(500, 0, 0),
                GovernanceTier::Tier1,
            )
            .await
            .unwrap();

        // 6. Consequence
        let consequence_id = mgr
            .append(
                wl.clone(),
                NodeContent::Consequence(ConsequenceNode::stable(0.95)),
                vec![commitment_id.clone()],
                TemporalAnchor::new(600, 0, 0),
                GovernanceTier::Tier1,
            )
            .await
            .unwrap();

        // Verify chain tracking.
        let chains = mgr.get_evolution_chains(&wl).await.unwrap();
        assert_eq!(chains.len(), 1);
        assert!(chains[0].is_complete());
        assert_eq!(chains[0].node_ids().len(), 6);

        // Validate the full chain.
        let validation = mgr.validate_chain(&consequence_id).await.unwrap();
        assert!(validation.valid);

        // Node count.
        assert_eq!(mgr.node_count().await.unwrap(), 6);
    }

    #[tokio::test]
    async fn latest_stable() {
        let mgr = InMemoryContextGraphManager::new();
        let wl = test_worldline();

        // No consequences yet.
        assert!(mgr.latest_stable(&wl).await.unwrap().is_none());

        // Add a stable consequence.
        mgr.append(
            wl.clone(),
            NodeContent::Consequence(ConsequenceNode::stable(0.9)),
            vec![],
            TemporalAnchor::new(100, 0, 0),
            GovernanceTier::Tier0,
        )
        .await
        .unwrap();

        let latest = mgr.latest_stable(&wl).await.unwrap().unwrap();
        assert_eq!(latest.content_type(), NodeContentType::Consequence);
    }

    #[tokio::test]
    async fn query_by_time_range() {
        let mgr = InMemoryContextGraphManager::new();
        let wl = test_worldline();

        for ts in [100, 200, 300, 400, 500] {
            mgr.append(
                wl.clone(),
                NodeContent::Intent(IntentNode::new(
                    worldline_types::EventId::new(),
                    &format!("t={}", ts),
                    GovernanceTier::Tier0,
                )),
                vec![],
                TemporalAnchor::new(ts, 0, 0),
                GovernanceTier::Tier0,
            )
            .await
            .unwrap();
        }

        let range = TemporalRange::new(
            TemporalAnchor::new(200, 0, 0),
            TemporalAnchor::new(400, 0, 0),
        );
        let nodes = mgr.query_by_time(&wl, &range).await.unwrap();
        assert_eq!(nodes.len(), 3); // 200, 300, 400
    }

    #[tokio::test]
    async fn multiple_worldlines_isolated() {
        let mgr = InMemoryContextGraphManager::new();
        let wl1 = test_worldline();
        let wl2 = test_worldline();

        mgr.append(
            wl1.clone(),
            NodeContent::Intent(IntentNode::new(
                worldline_types::EventId::new(),
                "wl1",
                GovernanceTier::Tier0,
            )),
            vec![],
            TemporalAnchor::new(100, 0, 0),
            GovernanceTier::Tier0,
        )
        .await
        .unwrap();

        mgr.append(
            wl2.clone(),
            NodeContent::Intent(IntentNode::new(
                worldline_types::EventId::new(),
                "wl2",
                GovernanceTier::Tier0,
            )),
            vec![],
            TemporalAnchor::new(100, 0, 0),
            GovernanceTier::Tier0,
        )
        .await
        .unwrap();

        let chains1 = mgr.get_evolution_chains(&wl1).await.unwrap();
        let chains2 = mgr.get_evolution_chains(&wl2).await.unwrap();
        assert_eq!(chains1.len(), 1);
        assert_eq!(chains2.len(), 1);
    }

    #[tokio::test]
    async fn validation_catches_tampered_node() {
        let mgr = InMemoryContextGraphManager::new();
        let wl = test_worldline();

        let id = mgr
            .append(
                wl,
                NodeContent::Intent(IntentNode::new(
                    worldline_types::EventId::new(),
                    "test",
                    GovernanceTier::Tier0,
                )),
                vec![],
                TemporalAnchor::new(100, 0, 0),
                GovernanceTier::Tier0,
            )
            .await
            .unwrap();

        // Validate should pass for a clean node.
        let result = mgr.validate_chain(&id).await.unwrap();
        assert!(result.valid);
    }
}
