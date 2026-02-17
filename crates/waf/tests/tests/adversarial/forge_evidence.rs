//! Adversarial test: Forged/tampered evidence bundles are detected and rejected.
//!
//! Verifies invariant I.WAF-5: Evidence Completeness.
//! The swap gate must reject any evidence bundle whose content hash does not
//! match its actual contents.

use maple_waf_context_graph::{ContentHash, GovernanceTier};
use maple_waf_evidence::*;
use maple_waf_swap_gate::{UpgradeProposal, WafSwapGate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn legitimate_evidence() -> EvidenceBundle {
    EvidenceBundle::new(
        ContentHash::hash(b"delta"),
        ContentHash::hash(b"artifact"),
        vec![TestResult {
            name: "unit_tests".into(),
            passed: true,
            duration_ms: 10,
            error: None,
        }],
        vec![InvariantResult {
            id: "I.1".into(),
            description: "Identity".into(),
            holds: true,
            details: "verified".into(),
        }],
        Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
        EquivalenceTier::E0,
    )
}

fn make_proposal() -> UpgradeProposal {
    UpgradeProposal::new(
        ContentHash::hash(b"artifact"),
        ContentHash::hash(b"evidence"),
        ContentHash::hash(b"delta"),
    )
    .with_governance_tier(GovernanceTier::Tier0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tampered_delta_hash_rejected() {
    let gate = WafSwapGate::new();
    let mut evidence = legitimate_evidence();
    evidence.delta_hash = ContentHash::hash(b"forged-delta");

    let proposal = make_proposal();
    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn tampered_artifact_hash_rejected() {
    let gate = WafSwapGate::new();
    let mut evidence = legitimate_evidence();
    evidence.artifact_hash = ContentHash::hash(b"forged-artifact");

    let proposal = make_proposal();
    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn tampered_test_results_rejected() {
    let gate = WafSwapGate::new();
    let mut evidence = legitimate_evidence();

    // Inject a passing test that was not in the original bundle.
    evidence.test_results.push(TestResult {
        name: "injected_passing_test".into(),
        passed: true,
        duration_ms: 1,
        error: None,
    });

    let proposal = make_proposal();
    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn flipped_test_result_rejected() {
    let gate = WafSwapGate::new();
    let mut evidence = legitimate_evidence();

    // Flip a test from failing to passing after hash computation.
    // First create evidence with a failing test.
    let mut bad_evidence = EvidenceBundle::new(
        ContentHash::hash(b"delta"),
        ContentHash::hash(b"artifact"),
        vec![TestResult {
            name: "unit_tests".into(),
            passed: false,
            duration_ms: 10,
            error: Some("failed".into()),
        }],
        vec![InvariantResult {
            id: "I.1".into(),
            description: "Identity".into(),
            holds: true,
            details: "verified".into(),
        }],
        Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
        EquivalenceTier::E0,
    );

    // Now flip the test to passing (forgery).
    bad_evidence.test_results[0].passed = true;
    bad_evidence.test_results[0].error = None;

    let proposal = make_proposal();
    let result = gate.execute(&proposal, &bad_evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn tampered_invariant_results_rejected() {
    let gate = WafSwapGate::new();
    let mut evidence = legitimate_evidence();

    // Flip an invariant from violated to holding.
    evidence.invariant_results[0].holds = false;

    let proposal = make_proposal();
    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn forged_bundle_hash_rejected() {
    let gate = WafSwapGate::new();
    let mut evidence = legitimate_evidence();

    // Replace the hash with a completely fabricated one.
    evidence.hash = ContentHash::hash(b"fabricated-hash");

    let proposal = make_proposal();
    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn empty_test_results_rejected() {
    let gate = WafSwapGate::new();
    let evidence = EvidenceBundle::new(
        ContentHash::hash(b"delta"),
        ContentHash::hash(b"artifact"),
        vec![], // No tests!
        vec![InvariantResult {
            id: "I.1".into(),
            description: "Identity".into(),
            holds: true,
            details: "ok".into(),
        }],
        Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
        EquivalenceTier::E0,
    );

    let proposal = make_proposal();
    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn empty_invariant_results_rejected() {
    let gate = WafSwapGate::new();
    let evidence = EvidenceBundle::new(
        ContentHash::hash(b"delta"),
        ContentHash::hash(b"artifact"),
        vec![TestResult {
            name: "t".into(),
            passed: true,
            duration_ms: 1,
            error: None,
        }],
        vec![], // No invariants!
        Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
        EquivalenceTier::E0,
    );

    let proposal = make_proposal();
    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn tampered_repro_build_rejected() {
    let gate = WafSwapGate::new();
    let mut evidence = legitimate_evidence();

    // Change the repro build result.
    evidence.repro_build = Some(ReproBuildResult::failed(
        ContentHash::hash(b"build1"),
        ContentHash::hash(b"build2"),
    ));

    let proposal = make_proposal();
    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    // The hash won't match since repro_build data was changed after construction.
    assert!(result.is_err());
}

#[tokio::test]
async fn legitimate_evidence_accepted() {
    let gate = WafSwapGate::new();
    let evidence = legitimate_evidence();
    let proposal = make_proposal();

    let result = gate.execute(&proposal, &evidence, vec![1]).await.unwrap();
    assert!(result.is_success());
}

#[tokio::test]
async fn verify_hash_detects_all_tampering_vectors() {
    let original = legitimate_evidence();
    assert!(original.verify_hash());

    // Vector 1: tamper delta_hash.
    let mut v1 = original.clone();
    v1.delta_hash = ContentHash::hash(b"evil");
    assert!(!v1.verify_hash());

    // Vector 2: tamper artifact_hash.
    let mut v2 = original.clone();
    v2.artifact_hash = ContentHash::hash(b"evil");
    assert!(!v2.verify_hash());

    // Vector 3: tamper test_results.
    let mut v3 = original.clone();
    v3.test_results.push(TestResult {
        name: "injected".into(),
        passed: true,
        duration_ms: 0,
        error: None,
    });
    assert!(!v3.verify_hash());

    // Vector 4: tamper invariant_results.
    let mut v4 = original.clone();
    v4.invariant_results[0].holds = !v4.invariant_results[0].holds;
    assert!(!v4.verify_hash());

    // Vector 5: tamper repro_build.
    let mut v5 = original.clone();
    v5.repro_build = None;
    assert!(!v5.verify_hash());
}
