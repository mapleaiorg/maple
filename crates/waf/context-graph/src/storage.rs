use crate::error::StorageError;
use crate::graph::WllNode;
use crate::types::{ContentHash, NodeContentType, TemporalRange};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use worldline_types::WorldlineId;

/// Pluggable storage backend for the WLL Context Graph.
///
/// Implementations may use in-memory stores, SQLite, RocksDB, etc.
#[async_trait]
pub trait GraphStorage: Send + Sync {
    /// Store a node. Fails if node with same ID already exists.
    async fn put(&self, node: WllNode) -> Result<(), StorageError>;

    /// Retrieve a node by content hash.
    async fn get(&self, id: &ContentHash) -> Result<Option<WllNode>, StorageError>;

    /// Check if a node exists.
    async fn contains(&self, id: &ContentHash) -> Result<bool, StorageError>;

    /// Get all nodes for a worldline.
    async fn get_by_worldline(&self, worldline_id: &WorldlineId) -> Result<Vec<WllNode>, StorageError>;

    /// Get nodes by content type.
    async fn get_by_type(
        &self,
        worldline_id: &WorldlineId,
        content_type: NodeContentType,
    ) -> Result<Vec<WllNode>, StorageError>;

    /// Get nodes within a temporal range.
    async fn get_by_time_range(
        &self,
        worldline_id: &WorldlineId,
        range: &TemporalRange,
    ) -> Result<Vec<WllNode>, StorageError>;

    /// Get the children of a node (nodes that have this node as a parent).
    async fn get_children(&self, parent_id: &ContentHash) -> Result<Vec<WllNode>, StorageError>;

    /// Total node count.
    async fn count(&self) -> Result<usize, StorageError>;

    /// Total node count for a worldline.
    async fn count_for_worldline(&self, worldline_id: &WorldlineId) -> Result<usize, StorageError>;
}

/// In-memory graph storage for testing and development.
#[derive(Clone)]
pub struct InMemoryGraphStorage {
    nodes: Arc<RwLock<HashMap<ContentHash, WllNode>>>,
}

impl InMemoryGraphStorage {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryGraphStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GraphStorage for InMemoryGraphStorage {
    async fn put(&self, node: WllNode) -> Result<(), StorageError> {
        let mut store = self
            .nodes
            .write()
            .map_err(|e| StorageError::Io(format!("lock poisoned: {}", e)))?;
        if store.contains_key(&node.id) {
            return Err(StorageError::AlreadyExists(node.id.clone()));
        }
        store.insert(node.id.clone(), node);
        Ok(())
    }

    async fn get(&self, id: &ContentHash) -> Result<Option<WllNode>, StorageError> {
        let store = self
            .nodes
            .read()
            .map_err(|e| StorageError::Io(format!("lock poisoned: {}", e)))?;
        Ok(store.get(id).cloned())
    }

    async fn contains(&self, id: &ContentHash) -> Result<bool, StorageError> {
        let store = self
            .nodes
            .read()
            .map_err(|e| StorageError::Io(format!("lock poisoned: {}", e)))?;
        Ok(store.contains_key(id))
    }

    async fn get_by_worldline(
        &self,
        worldline_id: &WorldlineId,
    ) -> Result<Vec<WllNode>, StorageError> {
        let store = self
            .nodes
            .read()
            .map_err(|e| StorageError::Io(format!("lock poisoned: {}", e)))?;
        let mut nodes: Vec<WllNode> = store
            .values()
            .filter(|n| n.worldline_id == *worldline_id)
            .cloned()
            .collect();
        nodes.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(nodes)
    }

    async fn get_by_type(
        &self,
        worldline_id: &WorldlineId,
        content_type: NodeContentType,
    ) -> Result<Vec<WllNode>, StorageError> {
        let store = self
            .nodes
            .read()
            .map_err(|e| StorageError::Io(format!("lock poisoned: {}", e)))?;
        let mut nodes: Vec<WllNode> = store
            .values()
            .filter(|n| n.worldline_id == *worldline_id && n.content_type() == content_type)
            .cloned()
            .collect();
        nodes.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(nodes)
    }

    async fn get_by_time_range(
        &self,
        worldline_id: &WorldlineId,
        range: &TemporalRange,
    ) -> Result<Vec<WllNode>, StorageError> {
        let store = self
            .nodes
            .read()
            .map_err(|e| StorageError::Io(format!("lock poisoned: {}", e)))?;
        let mut nodes: Vec<WllNode> = store
            .values()
            .filter(|n| n.worldline_id == *worldline_id && range.contains(&n.timestamp))
            .cloned()
            .collect();
        nodes.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(nodes)
    }

    async fn get_children(&self, parent_id: &ContentHash) -> Result<Vec<WllNode>, StorageError> {
        let store = self
            .nodes
            .read()
            .map_err(|e| StorageError::Io(format!("lock poisoned: {}", e)))?;
        let mut nodes: Vec<WllNode> = store
            .values()
            .filter(|n| n.parent_ids.contains(parent_id))
            .cloned()
            .collect();
        nodes.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(nodes)
    }

    async fn count(&self) -> Result<usize, StorageError> {
        let store = self
            .nodes
            .read()
            .map_err(|e| StorageError::Io(format!("lock poisoned: {}", e)))?;
        Ok(store.len())
    }

    async fn count_for_worldline(&self, worldline_id: &WorldlineId) -> Result<usize, StorageError> {
        let store = self
            .nodes
            .read()
            .map_err(|e| StorageError::Io(format!("lock poisoned: {}", e)))?;
        Ok(store
            .values()
            .filter(|n| n.worldline_id == *worldline_id)
            .count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::NodeContent;
    use crate::nodes::IntentNode;
    use crate::types::GovernanceTier;
    use worldline_types::{EventId, TemporalAnchor};

    fn make_node(wl: &WorldlineId, ts_ms: u64) -> WllNode {
        let intent = IntentNode::new(EventId::new(), "test", GovernanceTier::Tier0);
        WllNode::new(
            wl.clone(),
            NodeContent::Intent(intent),
            vec![],
            TemporalAnchor::new(ts_ms, 0, 0),
            GovernanceTier::Tier0,
        )
    }

    #[tokio::test]
    async fn put_and_get() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();
        let node = make_node(&wl, 100);
        let id = node.id.clone();

        storage.put(node).await.unwrap();
        let retrieved = storage.get(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, id);
    }

    #[tokio::test]
    async fn put_duplicate_fails() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();
        let node = make_node(&wl, 100);
        let node2 = node.clone();

        storage.put(node).await.unwrap();
        assert!(storage.put(node2).await.is_err());
    }

    #[tokio::test]
    async fn get_nonexistent() {
        let storage = InMemoryGraphStorage::new();
        let result = storage.get(&ContentHash::hash(b"nope")).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn contains() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();
        let node = make_node(&wl, 100);
        let id = node.id.clone();

        assert!(!storage.contains(&id).await.unwrap());
        storage.put(node).await.unwrap();
        assert!(storage.contains(&id).await.unwrap());
    }

    #[tokio::test]
    async fn get_by_worldline() {
        let storage = InMemoryGraphStorage::new();
        let wl1 = WorldlineId::ephemeral();
        let wl2 = WorldlineId::ephemeral();

        storage.put(make_node(&wl1, 100)).await.unwrap();
        storage.put(make_node(&wl1, 200)).await.unwrap();
        storage.put(make_node(&wl2, 150)).await.unwrap();

        let nodes = storage.get_by_worldline(&wl1).await.unwrap();
        assert_eq!(nodes.len(), 2);
        // Should be time-ordered.
        assert!(nodes[0].timestamp < nodes[1].timestamp);
    }

    #[tokio::test]
    async fn get_by_type() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();

        storage.put(make_node(&wl, 100)).await.unwrap();
        storage.put(make_node(&wl, 200)).await.unwrap();

        let intents = storage
            .get_by_type(&wl, NodeContentType::Intent)
            .await
            .unwrap();
        assert_eq!(intents.len(), 2);

        let deltas = storage
            .get_by_type(&wl, NodeContentType::Delta)
            .await
            .unwrap();
        assert_eq!(deltas.len(), 0);
    }

    #[tokio::test]
    async fn get_by_time_range() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();

        storage.put(make_node(&wl, 100)).await.unwrap();
        storage.put(make_node(&wl, 200)).await.unwrap();
        storage.put(make_node(&wl, 300)).await.unwrap();

        let range = TemporalRange::new(
            TemporalAnchor::new(150, 0, 0),
            TemporalAnchor::new(250, 0, 0),
        );
        let nodes = storage.get_by_time_range(&wl, &range).await.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].timestamp.physical_ms, 200);
    }

    #[tokio::test]
    async fn get_children() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();
        let parent = make_node(&wl, 100);
        let parent_id = parent.id.clone();
        storage.put(parent).await.unwrap();

        let intent = IntentNode::new(EventId::new(), "child", GovernanceTier::Tier0);
        let child = WllNode::new(
            wl,
            NodeContent::Intent(intent),
            vec![parent_id.clone()],
            TemporalAnchor::new(200, 0, 0),
            GovernanceTier::Tier0,
        );
        storage.put(child).await.unwrap();

        let children = storage.get_children(&parent_id).await.unwrap();
        assert_eq!(children.len(), 1);
    }

    #[tokio::test]
    async fn count() {
        let storage = InMemoryGraphStorage::new();
        assert_eq!(storage.count().await.unwrap(), 0);

        let wl = WorldlineId::ephemeral();
        storage.put(make_node(&wl, 100)).await.unwrap();
        storage.put(make_node(&wl, 200)).await.unwrap();
        assert_eq!(storage.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn count_for_worldline() {
        let storage = InMemoryGraphStorage::new();
        let wl1 = WorldlineId::ephemeral();
        let wl2 = WorldlineId::ephemeral();

        storage.put(make_node(&wl1, 100)).await.unwrap();
        storage.put(make_node(&wl1, 200)).await.unwrap();
        storage.put(make_node(&wl2, 150)).await.unwrap();

        assert_eq!(storage.count_for_worldline(&wl1).await.unwrap(), 2);
        assert_eq!(storage.count_for_worldline(&wl2).await.unwrap(), 1);
    }
}
