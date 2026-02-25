//! Adversarial test: The swap gate cannot be bypassed.
//!
//! Proposals without valid evidence, with insufficient governance tier,
//! or with failing shadow execution must all be denied.
//!
//! Verifies invariants:
//! - I.WAF-3: Swap Atomicity
//! - I.WAF-5: Evidence Completeness

use maple_waf_context_graph::{ContentHash, GovernanceTier};
use maple_waf_evidence::*;
use maple_waf_swap_gate::{SimulatedShadowRunner, UpgradeProposal, WafSwapGate};

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

fn make_proposal(tier: GovernanceTier) -> UpgradeProposal {
    UpgradeProposal::new(
        ContentHash::hash(b"artifact"),
        ContentHash::hash(b"evidence"),
        ContentHash::hash(b"delta"),
    )
    .with_governance_tier(tier)
}

// ---------------------------------------------------------------------------
// Tests: Evidence Bypass Attempts
// ---------------------------------------------------------------------------

#[tokio::test]
async fn no_evidence_cannot_bypass_gate() {
    // Create evidence with no tests and no invariants (but valid hash).
    let gate = WafSwapGate::new();
    let evidence = EvidenceBundle::new(
        ContentHash::hash(b"delta"),
        ContentHash::hash(b"artifact"),
        vec![], // No tests.
        vec![], // No invariants.
        None,   // No repro build.
        EquivalenceTier::E0,
    );
    let proposal = make_proposal(GovernanceTier::Tier0);

    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn failing_tests_cannot_bypass_gate() {
    let gate = WafSwapGate::new();
    let evidence = EvidenceBundle::new(
        ContentHash::hash(b"delta"),
        ContentHash::hash(b"artifact"),
        vec![TestResult {
            name: "test".into(),
            passed: false,
            duration_ms: 1,
            error: Some("assertion error".into()),
        }],
        vec![InvariantResult {
            id: "I.1".into(),
            description: "Identity".into(),
            holds: true,
            details: "ok".into(),
        }],
        Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
        EquivalenceTier::E0,
    );
    let proposal = make_proposal(GovernanceTier::Tier0);

    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn failing_invariants_cannot_bypass_gate() {
    let gate = WafSwapGate::new();
    let evidence = EvidenceBundle::new(
        ContentHash::hash(b"delta"),
        ContentHash::hash(b"artifact"),
        vec![TestResult {
            name: "test".into(),
            passed: true,
            duration_ms: 1,
            error: None,
        }],
        vec![InvariantResult {
            id: "I.WAF-1".into(),
            description: "Context Graph Integrity".into(),
            holds: false, // Invariant violated.
            details: "hash mismatch detected".into(),
        }],
        Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
        EquivalenceTier::E0,
    );
    let proposal = make_proposal(GovernanceTier::Tier0);

    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn mixed_test_results_cannot_bypass_gate() {
    let gate = WafSwapGate::new();
    let evidence = EvidenceBundle::new(
        ContentHash::hash(b"delta"),
        ContentHash::hash(b"artifact"),
        vec![
            TestResult {
                name: "pass".into(),
                passed: true,
                duration_ms: 1,
                error: None,
            },
            TestResult {
                name: "fail".into(),
                passed: false,
                duration_ms: 1,
                error: Some("error".into()),
            },
        ],
        vec![InvariantResult {
            id: "I.1".into(),
            description: "d".into(),
            holds: true,
            details: "ok".into(),
        }],
        Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
        EquivalenceTier::E0,
    );
    let proposal = make_proposal(GovernanceTier::Tier0);

    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Tests: Governance Bypass Attempts
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tier3_proposal_denied_with_tier2_auto_approve() {
    let gate = WafSwapGate::new().with_auto_approve_max(GovernanceTier::Tier2);
    let evidence = valid_evidence();
    let proposal = make_proposal(GovernanceTier::Tier3);

    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn tier4_proposal_denied_with_tier1_auto_approve() {
    let gate = WafSwapGate::new().with_auto_approve_max(GovernanceTier::Tier1);
    let evidence = valid_evidence();
    let proposal = make_proposal(GovernanceTier::Tier4);

    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn tier5_proposal_denied_even_with_tier4_auto_approve() {
    let gate = WafSwapGate::new().with_auto_approve_max(GovernanceTier::Tier4);
    let evidence = valid_evidence();
    let proposal = make_proposal(GovernanceTier::Tier5);

    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Tests: Shadow Execution Bypass Attempts
// ---------------------------------------------------------------------------

#[tokio::test]
async fn failing_shadow_execution_blocks_swap() {
    let gate = WafSwapGate::new().with_shadow_runner(SimulatedShadowRunner::failing());
    let evidence = valid_evidence();
    let proposal = make_proposal(GovernanceTier::Tier0);

    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Tests: Multiple bypass attempts in sequence
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sequential_bypass_attempts_all_fail() {
    let gate = WafSwapGate::new().with_auto_approve_max(GovernanceTier::Tier1);

    // Attempt 1: No evidence.
    let empty_evidence = EvidenceBundle::new(
        ContentHash::hash(b"d"),
        ContentHash::hash(b"a"),
        vec![],
        vec![],
        None,
        EquivalenceTier::E0,
    );
    let p1 = make_proposal(GovernanceTier::Tier0);
    assert!(gate.execute(&p1, &empty_evidence, vec![1]).await.is_err());

    // Attempt 2: Failing tests.
    let fail_evidence = EvidenceBundle::new(
        ContentHash::hash(b"d"),
        ContentHash::hash(b"a"),
        vec![TestResult {
            name: "t".into(),
            passed: false,
            duration_ms: 1,
            error: Some("err".into()),
        }],
        vec![InvariantResult {
            id: "I.1".into(),
            description: "d".into(),
            holds: true,
            details: "ok".into(),
        }],
        Some(ReproBuildResult::verified(ContentHash::hash(b"b"))),
        EquivalenceTier::E0,
    );
    let p2 = make_proposal(GovernanceTier::Tier0);
    assert!(gate.execute(&p2, &fail_evidence, vec![1]).await.is_err());

    // Attempt 3: Too high governance tier.
    let good_evidence = valid_evidence();
    let p3 = make_proposal(GovernanceTier::Tier3);
    assert!(gate.execute(&p3, &good_evidence, vec![1]).await.is_err());

    // No snapshots should have been created from any of these attempts.
    assert_eq!(gate.rollback_manager().snapshot_count(), 0);
}

#[tokio::test]
async fn valid_proposal_succeeds_after_failed_attempts() {
    let gate = WafSwapGate::new();

    // First: a failed attempt.
    let bad_evidence = EvidenceBundle::new(
        ContentHash::hash(b"d"),
        ContentHash::hash(b"a"),
        vec![],
        vec![],
        None,
        EquivalenceTier::E0,
    );
    let bad_proposal = make_proposal(GovernanceTier::Tier0);
    assert!(gate
        .execute(&bad_proposal, &bad_evidence, vec![1])
        .await
        .is_err());

    // Then: a valid attempt should succeed.
    let good_evidence = valid_evidence();
    let good_proposal = make_proposal(GovernanceTier::Tier0);
    let result = gate
        .execute(&good_proposal, &good_evidence, vec![1])
        .await
        .unwrap();
    assert!(result.is_success());
}
