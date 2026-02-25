//! End-to-end test: Governance tier classification and approval workflows.
//!
//! Verifies that:
//! - GovernanceTierEngine classifies changes correctly
//! - ApprovalManager auto-approves low tiers and holds high tiers
//! - Swap gate respects governance tier requirements

use maple_waf_context_graph::{ContentHash, GovernanceTier};
use maple_waf_evidence::*;
use maple_waf_governance::{
    ApprovalManager, ApprovalRequest, GovernanceTierEngine, SimulatedApprovalManager,
};
use maple_waf_swap_gate::{UpgradeProposal, WafSwapGate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_passing_evidence() -> EvidenceBundle {
    EvidenceBundle::new(
        ContentHash::hash(b"delta"),
        ContentHash::hash(b"artifact"),
        vec![TestResult {
            name: "test_all".into(),
            passed: true,
            duration_ms: 10,
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

fn make_request(id: &str, tier: GovernanceTier) -> ApprovalRequest {
    ApprovalRequest {
        id: id.into(),
        governance_tier: tier,
        description: format!("test change at {}", tier),
        requested_by: "test-system".into(),
        timestamp_ms: 1700000000000,
    }
}

// ---------------------------------------------------------------------------
// Governance Tier Classification Tests
// ---------------------------------------------------------------------------

#[test]
fn classify_small_change_as_tier0() {
    let tier =
        GovernanceTierEngine::classify_change("fix typo in comments", &["src/utils.rs".into()], 5);
    assert_eq!(tier, GovernanceTier::Tier0);
}

#[test]
fn classify_moderate_change_as_tier1() {
    let tier = GovernanceTierEngine::classify_change(
        "refactor helper functions",
        &["src/helpers.rs".into()],
        50,
    );
    assert_eq!(tier, GovernanceTier::Tier1);
}

#[test]
fn classify_large_change_as_tier2() {
    let tier = GovernanceTierEngine::classify_change(
        "new feature module",
        &["src/features/new_module.rs".into()],
        200,
    );
    assert_eq!(tier, GovernanceTier::Tier2);
}

#[test]
fn classify_compiler_path_as_tier3() {
    let tier = GovernanceTierEngine::classify_change(
        "compiler pass optimization",
        &["src/compiler/passes/optimize.rs".into()],
        15,
    );
    assert_eq!(tier, GovernanceTier::Tier3);
}

#[test]
fn classify_kernel_path_as_tier4() {
    let tier = GovernanceTierEngine::classify_change(
        "kernel gate update",
        &["crates/maple-kernel-gate/src/gate.rs".into()],
        20,
    );
    assert_eq!(tier, GovernanceTier::Tier4);
}

#[test]
fn classify_very_large_change_as_tier3_or_higher() {
    let tier =
        GovernanceTierEngine::classify_change("massive refactor", &["src/lib.rs".into()], 750);
    assert!(tier >= GovernanceTier::Tier3);
}

// ---------------------------------------------------------------------------
// Governance Tier Properties
// ---------------------------------------------------------------------------

#[test]
fn lower_tiers_do_not_require_human_approval() {
    assert!(!GovernanceTier::Tier0.requires_human_approval());
    assert!(!GovernanceTier::Tier1.requires_human_approval());
    assert!(!GovernanceTier::Tier2.requires_human_approval());
}

#[test]
fn higher_tiers_require_human_approval() {
    assert!(GovernanceTier::Tier3.requires_human_approval());
    assert!(GovernanceTier::Tier4.requires_human_approval());
    assert!(GovernanceTier::Tier5.requires_human_approval());
}

#[test]
fn tier4_and_tier5_require_formal_verification() {
    assert!(!GovernanceTier::Tier3.requires_formal_verification());
    assert!(GovernanceTier::Tier4.requires_formal_verification());
    assert!(GovernanceTier::Tier5.requires_formal_verification());
}

#[test]
fn only_tier5_requires_multi_human() {
    assert!(!GovernanceTier::Tier4.requires_multi_human());
    assert!(GovernanceTier::Tier5.requires_multi_human());
}

// ---------------------------------------------------------------------------
// Approval Manager Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tier0_auto_approved() {
    let mgr = SimulatedApprovalManager::new();
    let req = make_request("req-0", GovernanceTier::Tier0);
    let status = mgr.request_approval(req).await.unwrap();
    assert!(status.is_approved());
}

#[tokio::test]
async fn tier2_auto_approved() {
    let mgr = SimulatedApprovalManager::new();
    let req = make_request("req-2", GovernanceTier::Tier2);
    let status = mgr.request_approval(req).await.unwrap();
    assert!(status.is_approved());
}

#[tokio::test]
async fn tier3_held_pending() {
    let mgr = SimulatedApprovalManager::new();
    let req = make_request("req-3", GovernanceTier::Tier3);
    let status = mgr.request_approval(req).await.unwrap();
    assert!(!status.is_decided());
    assert!(!status.is_approved());
}

#[tokio::test]
async fn tier4_held_pending() {
    let mgr = SimulatedApprovalManager::new();
    let req = make_request("req-4", GovernanceTier::Tier4);
    let status = mgr.request_approval(req).await.unwrap();
    assert!(!status.is_decided());
}

#[tokio::test]
async fn explicit_approve_promotes_pending_request() {
    let mgr = SimulatedApprovalManager::new();
    let req = make_request("req-3", GovernanceTier::Tier3);
    let status = mgr.request_approval(req).await.unwrap();
    assert!(!status.is_approved());

    mgr.approve("req-3").unwrap();
    let updated = mgr.check_approval("req-3").await.unwrap();
    assert!(updated.is_approved());
}

#[tokio::test]
async fn check_nonexistent_approval_fails() {
    let mgr = SimulatedApprovalManager::new();
    let result = mgr.check_approval("does-not-exist").await;
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Swap Gate Governance Integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn swap_gate_allows_low_tier_proposal() {
    let gate = WafSwapGate::new().with_auto_approve_max(GovernanceTier::Tier2);
    let evidence = make_passing_evidence();
    let proposal = UpgradeProposal::new(
        ContentHash::hash(b"art"),
        ContentHash::hash(b"evi"),
        ContentHash::hash(b"delta"),
    )
    .with_governance_tier(GovernanceTier::Tier1);

    let result = gate.execute(&proposal, &evidence, vec![1]).await.unwrap();
    assert!(result.is_success());
}

#[tokio::test]
async fn swap_gate_denies_high_tier_proposal() {
    let gate = WafSwapGate::new().with_auto_approve_max(GovernanceTier::Tier1);
    let evidence = make_passing_evidence();
    let proposal = UpgradeProposal::new(
        ContentHash::hash(b"art"),
        ContentHash::hash(b"evi"),
        ContentHash::hash(b"delta"),
    )
    .with_governance_tier(GovernanceTier::Tier3);

    let result = gate.execute(&proposal, &evidence, vec![1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn swap_gate_respects_exact_tier_boundary() {
    let gate = WafSwapGate::new().with_auto_approve_max(GovernanceTier::Tier2);
    let evidence = make_passing_evidence();

    // Tier2 should be allowed (at boundary).
    let proposal_ok = UpgradeProposal::new(
        ContentHash::hash(b"art2"),
        ContentHash::hash(b"evi2"),
        ContentHash::hash(b"delta2"),
    )
    .with_governance_tier(GovernanceTier::Tier2);
    let result = gate
        .execute(&proposal_ok, &evidence, vec![1])
        .await
        .unwrap();
    assert!(result.is_success());

    // Tier3 should be denied (one above boundary).
    let evidence2 = make_passing_evidence();
    let proposal_deny = UpgradeProposal::new(
        ContentHash::hash(b"art3"),
        ContentHash::hash(b"evi3"),
        ContentHash::hash(b"delta3"),
    )
    .with_governance_tier(GovernanceTier::Tier3);
    let result = gate.execute(&proposal_deny, &evidence2, vec![1]).await;
    assert!(result.is_err());
}
