//! End-to-end test: Swap gate + rollback manager correctly handles degradation by reverting.
//!
//! Verifies invariants:
//! - I.WAF-3: Swap Atomicity
//! - I.WAF-4: Rollback Guarantee
//! - I.WAF-5: Evidence Completeness

use maple_waf_context_graph::{ContentHash, GovernanceTier};
use maple_waf_evidence::*;
use maple_waf_swap_gate::{
    RollbackManager, SimulatedShadowRunner, SwapResult, UpgradeProposal, WafSwapGate,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_passing_evidence() -> EvidenceBundle {
    EvidenceBundle::new(
        ContentHash::hash(b"delta-v1"),
        ContentHash::hash(b"artifact-v1"),
        vec![
            TestResult {
                name: "unit_tests".into(),
                passed: true,
                duration_ms: 50,
                error: None,
            },
            TestResult {
                name: "integration_tests".into(),
                passed: true,
                duration_ms: 200,
                error: None,
            },
        ],
        vec![InvariantResult {
            id: "I.WAF-1".into(),
            description: "Context Graph Integrity".into(),
            holds: true,
            details: "all hashes verified".into(),
        }],
        Some(ReproBuildResult::verified(ContentHash::hash(b"build-v1"))),
        EquivalenceTier::E0,
    )
}

fn make_proposal(tier: GovernanceTier) -> UpgradeProposal {
    UpgradeProposal::new(
        ContentHash::hash(b"artifact-v1"),
        ContentHash::hash(b"evidence-v1"),
        ContentHash::hash(b"delta-v1"),
    )
    .with_governance_tier(tier)
    .with_description("performance optimization v1")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn swap_creates_snapshot_for_rollback() {
    let gate = WafSwapGate::new();
    let evidence = make_passing_evidence();
    let proposal = make_proposal(GovernanceTier::Tier0);
    let state = vec![1, 2, 3, 4, 5];

    let result = gate.execute(&proposal, &evidence, state).await.unwrap();
    assert!(result.is_success());

    // A snapshot should have been taken before the swap.
    assert_eq!(gate.rollback_manager().snapshot_count(), 1);
}

#[tokio::test]
async fn rollback_restores_previous_state() {
    let gate = WafSwapGate::new();
    let evidence = make_passing_evidence();
    let proposal = make_proposal(GovernanceTier::Tier0);
    let original_state = vec![10, 20, 30];

    gate.execute(&proposal, &evidence, original_state.clone())
        .await
        .unwrap();

    // Rollback to the latest snapshot.
    let rollback_result = gate.rollback().unwrap();
    assert!(matches!(rollback_result, SwapResult::RolledBack { .. }));

    // Verify the snapshot contains the original state.
    let snapshot = gate.rollback_manager().rollback_to_latest().unwrap();
    assert_eq!(snapshot.state, original_state);
}

#[tokio::test]
async fn multiple_swaps_maintain_snapshot_chain() {
    let gate = WafSwapGate::new();

    for i in 0..5 {
        let evidence = EvidenceBundle::new(
            ContentHash::hash(format!("delta-{}", i).as_bytes()),
            ContentHash::hash(format!("artifact-{}", i).as_bytes()),
            vec![TestResult {
                name: "t".into(),
                passed: true,
                duration_ms: 1,
                error: None,
            }],
            vec![InvariantResult {
                id: "I.1".into(),
                description: "Identity".into(),
                holds: true,
                details: "ok".into(),
            }],
            Some(ReproBuildResult::verified(ContentHash::hash(
                format!("build-{}", i).as_bytes(),
            ))),
            EquivalenceTier::E0,
        );
        let proposal = UpgradeProposal::new(
            ContentHash::hash(format!("artifact-{}", i).as_bytes()),
            ContentHash::hash(format!("evidence-{}", i).as_bytes()),
            ContentHash::hash(format!("delta-{}", i).as_bytes()),
        );

        gate.execute(&proposal, &evidence, vec![i as u8])
            .await
            .unwrap();
    }

    assert_eq!(gate.rollback_manager().snapshot_count(), 5);
}

#[tokio::test]
async fn rollback_manager_respects_max_snapshots() {
    let mgr = RollbackManager::new(3);

    mgr.take_snapshot(vec![1], "snap-1");
    mgr.take_snapshot(vec![2], "snap-2");
    mgr.take_snapshot(vec![3], "snap-3");
    mgr.take_snapshot(vec![4], "snap-4");

    // Only 3 snapshots should be retained.
    assert_eq!(mgr.snapshot_count(), 3);

    // The latest should be snap-4.
    let latest = mgr.latest().unwrap();
    assert_eq!(latest.state, vec![4]);
}

#[tokio::test]
async fn rollback_to_specific_snapshot() {
    let mgr = RollbackManager::new(10);

    let h1 = mgr.take_snapshot(vec![1], "first");
    let _h2 = mgr.take_snapshot(vec![2], "second");
    let _h3 = mgr.take_snapshot(vec![3], "third");

    // Rollback to the first snapshot specifically.
    let snap = mgr.rollback_to(&h1).unwrap();
    assert_eq!(snap.state, vec![1]);
    assert_eq!(snap.description, "first");
}

#[tokio::test]
async fn rollback_without_snapshots_fails_gracefully() {
    let gate = WafSwapGate::new();
    let result = gate.rollback();
    assert!(result.is_err());
}

#[tokio::test]
async fn snapshot_is_content_addressed() {
    let mgr = RollbackManager::new(10);

    let h1 = mgr.take_snapshot(vec![1, 2, 3], "snap-a");
    let expected = ContentHash::hash(&[1, 2, 3]);
    assert_eq!(h1, expected);

    // Same data produces the same hash.
    let h2 = mgr.take_snapshot(vec![1, 2, 3], "snap-b");
    assert_eq!(h1, h2);

    // Different data produces a different hash.
    let h3 = mgr.take_snapshot(vec![4, 5, 6], "snap-c");
    assert_ne!(h1, h3);
}

#[tokio::test]
async fn shadow_failure_prevents_swap_and_preserves_state() {
    let gate = WafSwapGate::new()
        .with_shadow_runner(SimulatedShadowRunner::failing());
    let evidence = make_passing_evidence();
    let proposal = make_proposal(GovernanceTier::Tier0);

    let result = gate.execute(&proposal, &evidence, vec![1, 2, 3]).await;
    assert!(result.is_err());

    // No snapshot should have been taken since the swap never got to that phase.
    assert_eq!(gate.rollback_manager().snapshot_count(), 0);
}
