use crate::error::{GraphError, ValidationError};
use crate::graph::WllNode;
use crate::storage::GraphStorage;
use crate::types::ValidationResult;
use std::collections::HashSet;

/// Validates WLL Context Graph integrity.
///
/// Enforces invariants:
/// - I.WAF-1: Content hash matches computed hash
/// - I.2: Causal provenance (signature verification)
/// - Parent references exist in storage
/// - Temporal ordering (child.timestamp >= parent.timestamp)
pub struct GraphValidator;

impl GraphValidator {
    /// Full validation of a single node against storage.
    pub async fn validate_node(
        node: &WllNode,
        storage: &dyn GraphStorage,
    ) -> Result<ValidationResult, GraphError> {
        let mut checks = 0;
        let mut passed = 0;
        let mut errors = Vec::new();

        // Check 1: Content hash integrity (I.WAF-1).
        checks += 1;
        match node.verify_content_hash() {
            Ok(()) => passed += 1,
            Err(e) => errors.push(format!("content hash: {}", e)),
        }

        // Check 2: Signature (if present).
        if node.signature.is_some() {
            checks += 1;
            match node.verify_signature() {
                Ok(()) => passed += 1,
                Err(e) => errors.push(format!("signature: {}", e)),
            }
        }

        // Check 3: Parent references exist.
        for parent_id in &node.parent_ids {
            checks += 1;
            match storage.contains(parent_id).await {
                Ok(true) => passed += 1,
                Ok(false) => errors.push(format!("dangling parent: {}", parent_id)),
                Err(e) => errors.push(format!("storage lookup failed: {}", e)),
            }
        }

        // Check 4: Temporal ordering (child >= parent).
        for parent_id in &node.parent_ids {
            checks += 1;
            match storage.get(parent_id).await {
                Ok(Some(parent)) => {
                    if node.timestamp >= parent.timestamp {
                        passed += 1;
                    } else {
                        errors.push(format!(
                            "temporal violation: node {} < parent {}",
                            node.timestamp.physical_ms, parent.timestamp.physical_ms
                        ));
                    }
                }
                Ok(None) => {
                    // Already caught by parent existence check.
                    errors.push(format!(
                        "parent not found for temporal check: {}",
                        parent_id
                    ));
                }
                Err(e) => errors.push(format!("storage error during temporal check: {}", e)),
            }
        }

        if errors.is_empty() {
            Ok(ValidationResult::ok(checks))
        } else {
            Ok(ValidationResult::failed(checks, passed, errors))
        }
    }

    /// Validate a causal chain: walk from leaf to root verifying links.
    pub async fn validate_chain(
        leaf_id: &crate::types::ContentHash,
        storage: &dyn GraphStorage,
    ) -> Result<ValidationResult, GraphError> {
        let mut visited = HashSet::new();
        let mut queue = vec![leaf_id.clone()];
        let mut total_checks = 0;
        let mut total_passed = 0;
        let mut all_errors = Vec::new();

        while let Some(current_id) = queue.pop() {
            if visited.contains(&current_id) {
                continue;
            }
            visited.insert(current_id.clone());

            let node = match storage
                .get(&current_id)
                .await
                .map_err(GraphError::Storage)?
            {
                Some(n) => n,
                None => {
                    // Missing node in chain — record as validation error, not hard error.
                    total_checks += 1;
                    all_errors.push(format!("node not found in chain: {}", current_id));
                    continue;
                }
            };

            let result = Self::validate_node(&node, storage).await?;
            total_checks += result.checks_performed;
            total_passed += result.checks_passed;
            all_errors.extend(result.errors);

            for parent_id in &node.parent_ids {
                if !visited.contains(parent_id) {
                    queue.push(parent_id.clone());
                }
            }
        }

        if all_errors.is_empty() {
            Ok(ValidationResult::ok(total_checks))
        } else {
            Ok(ValidationResult::failed(
                total_checks,
                total_passed,
                all_errors,
            ))
        }
    }

    /// Quick content-hash-only check (no storage needed).
    pub fn verify_content_hash(node: &WllNode) -> Result<(), ValidationError> {
        node.verify_content_hash()
    }

    /// Quick signature-only check (no storage needed).
    pub fn verify_signature(node: &WllNode) -> Result<(), ValidationError> {
        node.verify_signature()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{NodeContent, WllNode};
    use crate::nodes::IntentNode;
    use crate::storage::InMemoryGraphStorage;
    use crate::types::GovernanceTier;
    use worldline_types::{EventId, TemporalAnchor, WorldlineId};

    fn make_intent_node(
        wl: &WorldlineId,
        ts_ms: u64,
        parents: Vec<crate::types::ContentHash>,
    ) -> WllNode {
        let intent = IntentNode::new(EventId::new(), "test", GovernanceTier::Tier0);
        WllNode::new(
            wl.clone(),
            NodeContent::Intent(intent),
            parents,
            TemporalAnchor::new(ts_ms, 0, 0),
            GovernanceTier::Tier0,
        )
    }

    #[tokio::test]
    async fn validate_root_node() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();
        let node = make_intent_node(&wl, 100, vec![]);
        storage.put(node.clone()).await.unwrap();

        let result = GraphValidator::validate_node(&node, &storage)
            .await
            .unwrap();
        assert!(result.valid);
        assert!(result.checks_performed >= 1);
    }

    #[tokio::test]
    async fn validate_node_with_parents() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();

        let parent = make_intent_node(&wl, 100, vec![]);
        let parent_id = parent.id.clone();
        storage.put(parent).await.unwrap();

        let child = make_intent_node(&wl, 200, vec![parent_id]);
        storage.put(child.clone()).await.unwrap();

        let result = GraphValidator::validate_node(&child, &storage)
            .await
            .unwrap();
        assert!(result.valid);
    }

    #[tokio::test]
    async fn validate_dangling_parent() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();

        let dangling = crate::types::ContentHash::hash(b"nonexistent");
        let node = make_intent_node(&wl, 100, vec![dangling]);
        storage.put(node.clone()).await.unwrap();

        let result = GraphValidator::validate_node(&node, &storage)
            .await
            .unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("dangling")));
    }

    #[tokio::test]
    async fn validate_temporal_violation() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();

        let parent = make_intent_node(&wl, 200, vec![]);
        let parent_id = parent.id.clone();
        storage.put(parent).await.unwrap();

        // Child has earlier timestamp than parent — violation!
        let child = make_intent_node(&wl, 100, vec![parent_id]);
        storage.put(child.clone()).await.unwrap();

        let result = GraphValidator::validate_node(&child, &storage)
            .await
            .unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("temporal")));
    }

    #[tokio::test]
    async fn validate_tampered_hash() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();
        let mut node = make_intent_node(&wl, 100, vec![]);
        // Tamper with the hash (we have to store the original, then modify for the check).
        let original_id = node.id.clone();
        node.id = crate::types::ContentHash::hash(b"tampered");
        // We can't put this in storage (different ID), so validate against storage directly.
        // We store the node under its tampered ID.
        storage.put(node.clone()).await.unwrap();

        let result = GraphValidator::validate_node(&node, &storage)
            .await
            .unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("content hash")));
    }

    #[tokio::test]
    async fn validate_signed_node() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();
        let secret_bytes: [u8; 32] = [99u8; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);
        let mut node = make_intent_node(&wl, 100, vec![]);
        node.sign(&signing_key);
        storage.put(node.clone()).await.unwrap();

        let result = GraphValidator::validate_node(&node, &storage)
            .await
            .unwrap();
        assert!(result.valid);
        // Should have 2 checks: content hash + signature.
        assert!(result.checks_performed >= 2);
    }

    #[tokio::test]
    async fn validate_chain_simple() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();

        let n1 = make_intent_node(&wl, 100, vec![]);
        let n1_id = n1.id.clone();
        storage.put(n1).await.unwrap();

        let n2 = make_intent_node(&wl, 200, vec![n1_id.clone()]);
        let n2_id = n2.id.clone();
        storage.put(n2).await.unwrap();

        let n3 = make_intent_node(&wl, 300, vec![n2_id.clone()]);
        let n3_id = n3.id.clone();
        storage.put(n3).await.unwrap();

        let result = GraphValidator::validate_chain(&n3_id, &storage)
            .await
            .unwrap();
        assert!(result.valid);
        // 3 nodes, each with content hash check + parent checks.
        assert!(result.checks_performed >= 3);
    }

    #[tokio::test]
    async fn validate_chain_with_broken_link() {
        let storage = InMemoryGraphStorage::new();
        let wl = WorldlineId::ephemeral();

        let n1 = make_intent_node(&wl, 100, vec![]);
        let n1_id = n1.id.clone();
        storage.put(n1).await.unwrap();

        // n2 references a dangling parent that doesn't exist.
        let dangling = crate::types::ContentHash::hash(b"ghost");
        let n2 = make_intent_node(&wl, 200, vec![n1_id, dangling]);
        let n2_id = n2.id.clone();
        storage.put(n2).await.unwrap();

        let result = GraphValidator::validate_chain(&n2_id, &storage)
            .await
            .unwrap();
        assert!(!result.valid);
    }

    #[test]
    fn quick_content_hash_check() {
        let wl = WorldlineId::ephemeral();
        let node = make_intent_node(&wl, 100, vec![]);
        assert!(GraphValidator::verify_content_hash(&node).is_ok());
    }

    #[test]
    fn quick_content_hash_tampered() {
        let wl = WorldlineId::ephemeral();
        let mut node = make_intent_node(&wl, 100, vec![]);
        node.id = crate::types::ContentHash::hash(b"wrong");
        assert!(GraphValidator::verify_content_hash(&node).is_err());
    }
}
