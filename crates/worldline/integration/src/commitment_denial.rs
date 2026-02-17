//! Commitment Denial Integration Tests
//!
//! Verifies that the Commitment Gate correctly denies commitments
//! for various reasons, and that denials are recorded as first-class
//! entries in the ledger with full accountability.

use worldline_core::types::{
    AdjudicationDecision, CapabilityId, CommitmentScope, EffectDomain, EventId, IdentityMaterial,
    WorldlineId,
};
use worldline_runtime::gate::{AdjudicationResult, CommitmentDeclaration};
use worldline_runtime::mrp::{MeaningEnvelopeBuilder, MeaningPayload, MrpRouter, RouteDecision};

use crate::helpers::{KernelOptions, TestKernel};

/// Commitment exceeding capability scope is denied at Stage 3.
#[tokio::test]
async fn test_commitment_denied_insufficient_capability() {
    let mut kernel = TestKernel::new(KernelOptions::default()).await;

    let wid = kernel.create_worldline(1);
    let target = kernel.create_worldline(2);

    // Grant only communication capability
    kernel.grant_capability(&wid, "CAP-COMM", EffectDomain::Communication);

    let genesis = kernel.emit_genesis(&wid).await;
    let meaning = kernel.emit_meaning(&wid, vec![genesis.id.clone()]).await;
    let intent = kernel.emit_intent(&wid, vec![meaning.id.clone()]).await;

    // Attempt commitment in Financial domain — we don't have that capability
    let declaration = kernel.build_declaration(
        wid.clone(),
        intent.id.clone(),
        EffectDomain::Financial,
        "CAP-FIN",
        vec![target.clone()],
    );
    let cid = declaration.id.clone();

    let result = kernel.gate.submit(declaration).await.unwrap();

    // Should be denied
    assert!(
        matches!(result, AdjudicationResult::Denied { .. }),
        "Should be denied for insufficient capability"
    );

    // Denial is a first-class record in the ledger
    let entry = kernel.gate.ledger().history(&cid).unwrap();
    assert_eq!(entry.decision.decision, AdjudicationDecision::Deny);
    assert!(!entry.decision.rationale.is_empty());
}

/// Commitment without stabilized intent reference is rejected.
#[tokio::test]
async fn test_commitment_without_intent_rejected() {
    let mut kernel = TestKernel::new(KernelOptions::default()).await;

    let wid = kernel.create_worldline(3);
    let target = kernel.create_worldline(4);
    kernel.grant_capability(&wid, "CAP-COMM", EffectDomain::Communication);
    let _genesis = kernel.emit_genesis(&wid).await;

    // Build declaration WITHOUT intent reference
    let declaration = CommitmentDeclaration::builder(
        wid.clone(),
        CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![target],
            constraints: vec![],
        },
    )
    .capability(CapabilityId("CAP-COMM".into()))
    .build(); // NO derived_from_intent

    let result = kernel.gate.submit(declaration).await.unwrap();

    // Stage 1 (Declaration) denies: no intent reference
    assert!(matches!(result, AdjudicationResult::Denied { .. }));

    if let AdjudicationResult::Denied { decision } = result {
        assert!(
            decision.rationale.contains("I.3"),
            "Denial should reference I.3 (Commitment Boundary)"
        );
    }
}

/// Unknown identity is rejected at Stage 2.
#[tokio::test]
async fn test_commitment_unknown_identity_rejected() {
    let mut kernel = TestKernel::new(KernelOptions::default()).await;

    // Create a worldline NOT registered with identity manager
    let unknown_wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([99u8; 32]));

    let declaration = CommitmentDeclaration::builder(
        unknown_wid,
        CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![],
            constraints: vec![],
        },
    )
    .derived_from_intent(EventId::new())
    .capability(CapabilityId("CAP-COMM".into()))
    .build();

    let result = kernel.gate.submit(declaration).await.unwrap();
    assert!(
        matches!(result, AdjudicationResult::Denied { .. }),
        "Unknown identity should be denied at Stage 2"
    );
}

/// Policy denial is properly recorded.
#[tokio::test]
async fn test_commitment_policy_denial() {
    // Create kernel with deny-all policies
    let mut kernel = TestKernel::new(KernelOptions {
        approve_policies: false,
        require_intent_reference: true,
    })
    .await;

    let wid = kernel.create_worldline(5);
    let target = kernel.create_worldline(6);
    kernel.grant_capability(&wid, "CAP-COMM", EffectDomain::Communication);

    let genesis = kernel.emit_genesis(&wid).await;
    let meaning = kernel.emit_meaning(&wid, vec![genesis.id.clone()]).await;
    let intent = kernel.emit_intent(&wid, vec![meaning.id.clone()]).await;

    let declaration = kernel.build_declaration(
        wid.clone(),
        intent.id.clone(),
        EffectDomain::Communication,
        "CAP-COMM",
        vec![target],
    );
    let cid = declaration.id.clone();

    let result = kernel.gate.submit(declaration).await.unwrap();

    assert!(
        matches!(result, AdjudicationResult::Denied { .. }),
        "Should be denied by policy"
    );

    // Denial recorded
    let entry = kernel.gate.ledger().history(&cid).unwrap();
    assert_eq!(entry.decision.decision, AdjudicationDecision::Deny);
}

/// MEANING envelope cannot reach execution layer (I.3).
#[tokio::test]
async fn test_meaning_cannot_reach_execution() {
    let wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([7u8; 32]));

    let meaning_envelope = MeaningEnvelopeBuilder::new(wid.clone())
        .payload(MeaningPayload {
            interpretation: "test meaning".into(),
            confidence: 0.9,
            ambiguity_preserved: true,
            evidence_refs: vec![],
        })
        .build()
        .unwrap();

    let mut router = MrpRouter::new();
    let decision = router.route(&meaning_envelope).await.unwrap();

    // MEANING routes to cognition, NEVER to execution
    assert!(
        matches!(decision, RouteDecision::DeliverToCognition(_)),
        "MEANING must never reach execution"
    );
    // It is NOT RouteToGate
    assert!(
        !matches!(decision, RouteDecision::RouteToGate),
        "MEANING must not route to Gate"
    );
}

/// Multiple denials from different stages are all recorded.
#[tokio::test]
async fn test_multiple_denials_all_recorded() {
    let mut kernel = TestKernel::new(KernelOptions::default()).await;

    let wid = kernel.create_worldline(8);
    let target = kernel.create_worldline(9);
    kernel.grant_capability(&wid, "CAP-COMM", EffectDomain::Communication);
    let genesis = kernel.emit_genesis(&wid).await;
    let meaning = kernel.emit_meaning(&wid, vec![genesis.id.clone()]).await;
    let intent = kernel.emit_intent(&wid, vec![meaning.id.clone()]).await;

    // Submit a valid declaration → approved
    let decl1 = kernel.build_declaration(
        wid.clone(),
        intent.id.clone(),
        EffectDomain::Communication,
        "CAP-COMM",
        vec![target.clone()],
    );
    let result1 = kernel.gate.submit(decl1).await.unwrap();
    assert!(
        matches!(result1, AdjudicationResult::Approved { .. }),
        "Valid declaration should be approved: {:?}",
        result1
    );

    // Submit one without intent reference → denied
    let decl2 = CommitmentDeclaration::builder(
        wid.clone(),
        CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![],
            constraints: vec![],
        },
    )
    .build();
    let result2 = kernel.gate.submit(decl2).await.unwrap();
    assert!(matches!(result2, AdjudicationResult::Denied { .. }));

    // Ledger has both entries
    assert_eq!(kernel.gate.ledger().len(), 2);
}
