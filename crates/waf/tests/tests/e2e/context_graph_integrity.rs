//! End-to-end test: Context graph nodes maintain content-addressing integrity.
//!
//! Verifies invariant I.WAF-1: Context Graph Integrity.
//! Every WLL node is content-addressed via BLAKE3 and causally linked.

use maple_waf_context_graph::{
    CommitmentNode, ConsequenceNode, ContentHash, ContextGraphManager, DeltaNode,
    EvidenceBundleRef, GovernanceTier, GraphValidator, InMemoryContextGraphManager, InferenceNode,
    IntentNode, NodeContent, NodeContentType, WllNode,
};
use worldline_types::{EventId, TemporalAnchor, WorldlineId};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_worldline() -> WorldlineId {
    WorldlineId::ephemeral()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn append_node_and_verify_content_hash() {
    let mgr = InMemoryContextGraphManager::new();
    let wl = test_worldline();

    let intent = IntentNode::new(EventId::new(), "reduce latency", GovernanceTier::Tier1);
    let id = mgr
        .append(
            wl.clone(),
            NodeContent::Intent(intent),
            vec![],
            TemporalAnchor::now(0),
            GovernanceTier::Tier1,
        )
        .await
        .unwrap();

    let node = mgr.get_node(&id).await.unwrap().unwrap();
    assert!(node.verify_content_hash().is_ok());
}

#[tokio::test]
async fn content_addressing_is_deterministic() {
    let wl = test_worldline();
    let intent = IntentNode::new(EventId::new(), "test determinism", GovernanceTier::Tier0);
    let content = NodeContent::Intent(intent);
    let ts = TemporalAnchor::new(1000, 0, 0);

    let id1 = WllNode::compute_id(&content, &[], &wl, &ts);
    let id2 = WllNode::compute_id(&content, &[], &wl, &ts);
    assert_eq!(id1, id2);
}

#[tokio::test]
async fn different_content_produces_different_hashes() {
    let wl = test_worldline();
    let ts = TemporalAnchor::new(1000, 0, 0);

    let intent_a = IntentNode::new(EventId::new(), "intent A", GovernanceTier::Tier0);
    let intent_b = IntentNode::new(EventId::new(), "intent B", GovernanceTier::Tier1);

    let id_a = WllNode::compute_id(&NodeContent::Intent(intent_a), &[], &wl, &ts);
    let id_b = WllNode::compute_id(&NodeContent::Intent(intent_b), &[], &wl, &ts);
    assert_ne!(id_a, id_b);
}

#[tokio::test]
async fn different_parents_produce_different_hashes() {
    let wl = test_worldline();
    let intent = IntentNode::new(EventId::new(), "same content", GovernanceTier::Tier0);
    let content = NodeContent::Intent(intent);
    let ts = TemporalAnchor::new(1000, 0, 0);

    let id_no_parents = WllNode::compute_id(&content, &[], &wl, &ts);
    let id_with_parents = WllNode::compute_id(&content, &[ContentHash::hash(b"parent")], &wl, &ts);
    assert_ne!(id_no_parents, id_with_parents);
}

#[tokio::test]
async fn full_six_node_evolution_chain() {
    let mgr = InMemoryContextGraphManager::new();
    let wl = test_worldline();

    // 1. Intent
    let intent_id = mgr
        .append(
            wl.clone(),
            NodeContent::Intent(IntentNode::new(
                EventId::new(),
                "optimize memory",
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
            NodeContent::Inference(InferenceNode::new("llama3.2", 0.92)),
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
            NodeContent::Delta(DeltaNode::new(
                maple_waf_context_graph::SubstrateType::Rust,
                vec![0xDE, 0xAD],
            )),
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
            NodeContent::Evidence(EvidenceBundleRef::new(ContentHash::hash(
                b"evidence-bundle",
            ))),
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

    // Validate the full chain from the leaf.
    let validation = mgr.validate_chain(&consequence_id).await.unwrap();
    assert!(validation.valid);
    assert!(validation.checks_performed >= 6);

    // Verify total node count.
    assert_eq!(mgr.node_count().await.unwrap(), 6);
}

#[tokio::test]
async fn tampered_node_hash_detected() {
    let wl = test_worldline();
    let mut node = WllNode::new(
        wl,
        NodeContent::Intent(IntentNode::new(
            EventId::new(),
            "test",
            GovernanceTier::Tier0,
        )),
        vec![],
        TemporalAnchor::new(1000, 0, 0),
        GovernanceTier::Tier0,
    );

    // Tamper with the hash.
    node.id = ContentHash::hash(b"tampered");
    assert!(node.verify_content_hash().is_err());
}

#[tokio::test]
async fn signed_node_verified() {
    let wl = test_worldline();
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);

    let mut node = WllNode::new(
        wl,
        NodeContent::Intent(IntentNode::new(
            EventId::new(),
            "signed intent",
            GovernanceTier::Tier0,
        )),
        vec![],
        TemporalAnchor::new(1000, 0, 0),
        GovernanceTier::Tier0,
    );

    node.sign(&signing_key);
    assert!(node.signature.is_some());
    assert!(node.verify_signature().is_ok());
}

#[tokio::test]
async fn bad_signature_rejected() {
    let wl = test_worldline();
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);

    let mut node = WllNode::new(
        wl,
        NodeContent::Intent(IntentNode::new(
            EventId::new(),
            "test sig",
            GovernanceTier::Tier0,
        )),
        vec![],
        TemporalAnchor::new(1000, 0, 0),
        GovernanceTier::Tier0,
    );

    node.sign(&signing_key);
    // Tamper with the signature.
    node.signature = Some("00".repeat(64));
    assert!(node.verify_signature().is_err());
}

#[tokio::test]
async fn multiple_worldlines_isolated_in_same_graph() {
    let mgr = InMemoryContextGraphManager::new();
    let wl1 = test_worldline();
    let wl2 = test_worldline();

    mgr.append(
        wl1.clone(),
        NodeContent::Intent(IntentNode::new(
            EventId::new(),
            "wl1 intent",
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
            EventId::new(),
            "wl2 intent",
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

    // Total node count is 2.
    assert_eq!(mgr.node_count().await.unwrap(), 2);
}

#[tokio::test]
async fn graph_validator_catches_tampered_node() {
    let wl = test_worldline();
    let mut node = WllNode::new(
        wl,
        NodeContent::Intent(IntentNode::new(
            EventId::new(),
            "clean",
            GovernanceTier::Tier0,
        )),
        vec![],
        TemporalAnchor::new(100, 0, 0),
        GovernanceTier::Tier0,
    );

    assert!(GraphValidator::verify_content_hash(&node).is_ok());

    node.id = ContentHash::hash(b"evil");
    assert!(GraphValidator::verify_content_hash(&node).is_err());
}

#[tokio::test]
async fn latest_stable_returns_most_recent_stable_consequence() {
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
