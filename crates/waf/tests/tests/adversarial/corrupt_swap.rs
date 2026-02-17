//! Adversarial test: Corrupted snapshots and tampered artifacts are detected.
//!
//! Verifies invariants:
//! - I.WAF-3: Swap Atomicity — no partial upgrades
//! - I.WAF-4: Rollback Guarantee — always revert to last stable state
//! - I.WAF-1: Context Graph Integrity — content-addressed nodes detect tampering

use maple_waf_context_graph::{
    ContentHash, ContextGraphManager, GovernanceTier, InMemoryContextGraphManager, IntentNode,
    NodeContent, WllNode,
};
use maple_waf_evidence::*;
use maple_waf_swap_gate::{RollbackManager, Snapshot, UpgradeProposal, WafSwapGate};
use worldline_types::{EventId, TemporalAnchor, WorldlineId};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn valid_evidence() -> EvidenceBundle {
    EvidenceBundle::new(
        ContentHash::hash(b"delta"),
        ContentHash::hash(b"artifact"),
        vec![TestResult {
            name: "test".into(),
            passed: true,
            duration_ms: 5,
            error: None,
        }],
        vec![InvariantResult {
            id: "I.1".into(),
            description: "Identity".into(),
            holds: true,
            details: "ok".into(),
        }],
        Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
        EquivalenceTier::E0,
    )
}

// ---------------------------------------------------------------------------
// Tests: Snapshot Corruption Detection
// ---------------------------------------------------------------------------

#[test]
fn snapshot_hash_is_content_addressed() {
    let state = vec![1, 2, 3, 4, 5];
    let snapshot = Snapshot::new(state.clone(), "test snapshot");

    // The hash should match BLAKE3 of the state.
    let expected_hash = ContentHash::hash(&state);
    assert_eq!(snapshot.hash, expected_hash);
}

#[test]
fn different_state_produces_different_snapshot_hash() {
    let s1 = Snapshot::new(vec![1, 2, 3], "snap-1");
    let s2 = Snapshot::new(vec![4, 5, 6], "snap-2");
    assert_ne!(s1.hash, s2.hash);
}

#[test]
fn snapshot_retrieval_by_hash_detects_wrong_hash() {
    let mgr = RollbackManager::new(10);

    mgr.take_snapshot(vec![1, 2, 3], "legitimate");

    // Trying to get a snapshot by a non-existent hash fails.
    let fake_hash = ContentHash::hash(b"fake-state");
    let result = mgr.get(&fake_hash);
    assert!(result.is_err());
}

#[test]
fn rollback_to_nonexistent_snapshot_fails() {
    let mgr = RollbackManager::new(10);

    // Create one snapshot.
    mgr.take_snapshot(vec![1], "real");

    // Try to rollback to a completely different hash.
    let fake_hash = ContentHash::hash(b"nonexistent-state");
    let result = mgr.rollback_to(&fake_hash);
    assert!(result.is_err());
}

#[test]
fn snapshot_state_integrity_preserved() {
    let mgr = RollbackManager::new(10);
    let original_state = vec![10, 20, 30, 40, 50];
    let hash = mgr.take_snapshot(original_state.clone(), "checkpoint");

    // Retrieve and verify state is intact.
    let snapshot = mgr.get(&hash).unwrap();
    assert_eq!(snapshot.state, original_state);
    assert_eq!(snapshot.hash, ContentHash::hash(&original_state));
}

#[tokio::test]
async fn swap_snapshot_state_matches_pre_swap_state() {
    let gate = WafSwapGate::new();
    let evidence = valid_evidence();
    let proposal = UpgradeProposal::new(
        ContentHash::hash(b"art"),
        ContentHash::hash(b"evi"),
        ContentHash::hash(b"delta"),
    );

    let pre_swap_state = vec![99, 88, 77];
    gate.execute(&proposal, &evidence, pre_swap_state.clone())
        .await
        .unwrap();

    // The snapshot should contain the pre-swap state.
    let snapshot = gate.rollback_manager().rollback_to_latest().unwrap();
    assert_eq!(snapshot.state, pre_swap_state);
}

// ---------------------------------------------------------------------------
// Tests: WLL Node Tampering
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tampered_wll_node_content_detected() {
    let wl = WorldlineId::ephemeral();
    let mut node = WllNode::new(
        wl,
        NodeContent::Intent(IntentNode::new(
            EventId::new(),
            "original intent",
            GovernanceTier::Tier0,
        )),
        vec![],
        TemporalAnchor::new(1000, 0, 0),
        GovernanceTier::Tier0,
    );

    // Node should verify correctly as created.
    assert!(node.verify_content_hash().is_ok());

    // Tamper: replace the ID with a different hash.
    node.id = ContentHash::hash(b"evil-hash");
    assert!(node.verify_content_hash().is_err());
}

#[tokio::test]
async fn tampered_wll_node_signature_detected() {
    let wl = WorldlineId::ephemeral();
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

    // Legitimate verification passes.
    assert!(node.verify_signature().is_ok());

    // Tamper with the signature bytes.
    node.signature = Some("ff".repeat(64));
    assert!(node.verify_signature().is_err());
}

#[tokio::test]
async fn wrong_signing_key_detected() {
    let wl = WorldlineId::ephemeral();
    let key_a = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
    let key_b = ed25519_dalek::SigningKey::from_bytes(&[99u8; 32]);

    let mut node = WllNode::new(
        wl,
        NodeContent::Intent(IntentNode::new(
            EventId::new(),
            "signed with key A",
            GovernanceTier::Tier0,
        )),
        vec![],
        TemporalAnchor::new(1000, 0, 0),
        GovernanceTier::Tier0,
    );

    // Sign with key A.
    node.sign(&key_a);
    assert!(node.verify_signature().is_ok());

    // Replace signer public key with key B's public key — signature won't match.
    let vk_b = key_b.verifying_key();
    let pk_b_bytes = vk_b.as_bytes();
    let pk_b_hex: String = pk_b_bytes.iter().map(|b| format!("{:02x}", b)).collect();
    node.signer_public_key = Some(pk_b_hex);
    assert!(node.verify_signature().is_err());
}

#[tokio::test]
async fn unsigned_node_fails_signature_verification() {
    let wl = WorldlineId::ephemeral();
    let node = WllNode::new(
        wl,
        NodeContent::Intent(IntentNode::new(
            EventId::new(),
            "unsigned",
            GovernanceTier::Tier0,
        )),
        vec![],
        TemporalAnchor::new(1000, 0, 0),
        GovernanceTier::Tier0,
    );

    // No signature present.
    assert!(node.signature.is_none());
    assert!(node.verify_signature().is_err());
}

// ---------------------------------------------------------------------------
// Tests: Context Graph Integrity After Corruption
// ---------------------------------------------------------------------------

#[tokio::test]
async fn context_graph_rejects_dangling_parent_reference() {
    let mgr = InMemoryContextGraphManager::new();
    let wl = WorldlineId::ephemeral();

    // Add a node with a dangling parent reference.
    let dangling_parent = ContentHash::hash(b"nonexistent-node");
    let intent = IntentNode::new(EventId::new(), "orphan node", GovernanceTier::Tier0);
    let id = mgr
        .append(
            wl,
            NodeContent::Intent(intent),
            vec![dangling_parent],
            TemporalAnchor::new(100, 0, 0),
            GovernanceTier::Tier0,
        )
        .await
        .unwrap();

    // Chain validation should report the dangling reference.
    let validation = mgr.validate_chain(&id).await.unwrap();
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .iter()
            .any(|e| e.contains("dangling") || e.contains("not found"))
    );
}

#[tokio::test]
async fn evidence_bundle_serde_preserves_integrity() {
    let bundle = valid_evidence();
    assert!(bundle.verify_hash());

    let json = serde_json::to_string(&bundle).unwrap();
    let restored: EvidenceBundle = serde_json::from_str(&json).unwrap();

    // Deserialized bundle should still verify.
    assert!(restored.verify_hash());
    assert_eq!(restored.hash, bundle.hash);
}

#[tokio::test]
async fn snapshot_serde_preserves_content_address() {
    let snapshot = Snapshot::new(vec![1, 2, 3], "test");
    let json = serde_json::to_string(&snapshot).unwrap();
    let restored: Snapshot = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.hash, snapshot.hash);
    assert_eq!(restored.state, snapshot.state);
}
