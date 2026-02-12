//! MRP Non-Escalation Enforcement Tests
//!
//! Verifies I.MRP-1 (Non-Escalation): No envelope may be transformed
//! into a higher-resonance type than it declares. Also verifies that
//! CONSEQUENCE envelopes can only come from execution layer origins.

use std::sync::Arc;

use maple_kernel_mrp::{
    CommitmentEnvelopeBuilder, CommitmentPayload, ConsequenceEnvelopeBuilder,
    ConsequencePayload, MeaningEnvelopeBuilder,
    MeaningPayload, MockExecutionLayer, MrpRouter, RouteDecision, RejectionReason,
};
use maple_mwl_types::{
    CommitmentId, CommitmentScope, EffectDomain,
    IdentityMaterial, ResonanceType, WorldlineId,
};

fn test_wid() -> WorldlineId {
    WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
}

fn other_wid() -> WorldlineId {
    WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]))
}

/// No implicit type escalation: MEANING → INTENT rejected.
#[test]
fn test_meaning_to_intent_escalation_rejected() {
    let mut router = MrpRouter::new();
    let result = router.validate_non_escalation(
        &ResonanceType::Meaning,
        &ResonanceType::Intent,
        &test_wid(),
        uuid::Uuid::new_v4(),
    );
    assert!(result.is_err(), "MEANING → INTENT escalation must be rejected");
}

/// No implicit type escalation: MEANING → COMMITMENT rejected.
#[test]
fn test_meaning_to_commitment_escalation_rejected() {
    let mut router = MrpRouter::new();
    let result = router.validate_non_escalation(
        &ResonanceType::Meaning,
        &ResonanceType::Commitment,
        &test_wid(),
        uuid::Uuid::new_v4(),
    );
    assert!(result.is_err(), "MEANING → COMMITMENT escalation must be rejected");
}

/// No implicit type escalation: INTENT → COMMITMENT rejected.
#[test]
fn test_intent_to_commitment_escalation_rejected() {
    let mut router = MrpRouter::new();
    let result = router.validate_non_escalation(
        &ResonanceType::Intent,
        &ResonanceType::Commitment,
        &test_wid(),
        uuid::Uuid::new_v4(),
    );
    assert!(result.is_err(), "INTENT → COMMITMENT escalation must be rejected");
}

/// Same-type routing is always allowed.
#[test]
fn test_same_type_always_allowed() {
    let mut router = MrpRouter::new();
    for rt in &[
        ResonanceType::Meaning,
        ResonanceType::Intent,
        ResonanceType::Commitment,
        ResonanceType::Consequence,
    ] {
        let result = router.validate_non_escalation(
            rt,
            rt,
            &test_wid(),
            uuid::Uuid::new_v4(),
        );
        assert!(result.is_ok(), "{:?} → {:?} should be allowed", rt, rt);
    }
}

/// Each escalation violation is logged for accountability.
#[test]
fn test_escalation_violations_logged() {
    let mut router = MrpRouter::new();
    assert!(router.escalation_log().is_empty());

    // Three violations
    let _ = router.validate_non_escalation(
        &ResonanceType::Meaning,
        &ResonanceType::Intent,
        &test_wid(),
        uuid::Uuid::new_v4(),
    );
    let _ = router.validate_non_escalation(
        &ResonanceType::Meaning,
        &ResonanceType::Commitment,
        &test_wid(),
        uuid::Uuid::new_v4(),
    );
    let _ = router.validate_non_escalation(
        &ResonanceType::Intent,
        &ResonanceType::Commitment,
        &test_wid(),
        uuid::Uuid::new_v4(),
    );

    assert_eq!(
        router.escalation_log().len(),
        3,
        "All escalation violations must be logged"
    );
}

/// CONSEQUENCE envelope rejected from non-execution-layer origin.
#[tokio::test]
async fn test_consequence_only_from_execution_layer() {
    let wid = test_wid();
    let exec_layer = MockExecutionLayer::new(); // no registered origins
    let mut router = MrpRouter::with_execution_layer(Arc::new(exec_layer));

    let consequence = ConsequenceEnvelopeBuilder::new(wid.clone())
        .payload(ConsequencePayload {
            commitment_id: CommitmentId::new(),
            outcome_description: "done".into(),
            state_changes: serde_json::json!({}),
            observed_by: wid.clone(),
        })
        .build()
        .unwrap();

    let decision = router.route(&consequence).await.unwrap();
    assert!(
        matches!(decision, RouteDecision::Reject(RejectionReason::InvalidConsequenceOrigin)),
        "CONSEQUENCE from non-execution layer must be rejected"
    );
}

/// CONSEQUENCE envelope accepted from registered execution layer.
#[tokio::test]
async fn test_consequence_accepted_from_execution_layer() {
    let wid = test_wid();
    let exec_layer = MockExecutionLayer::new().register(wid.clone());
    let mut router = MrpRouter::with_execution_layer(Arc::new(exec_layer));

    let consequence = ConsequenceEnvelopeBuilder::new(wid.clone())
        .payload(ConsequencePayload {
            commitment_id: CommitmentId::new(),
            outcome_description: "done".into(),
            state_changes: serde_json::json!({}),
            observed_by: wid.clone(),
        })
        .build()
        .unwrap();

    let decision = router.route(&consequence).await.unwrap();
    assert!(
        matches!(decision, RouteDecision::DeliverAsConsequence(_)),
        "CONSEQUENCE from execution layer should be accepted"
    );
}

/// Tampered envelope is quarantined.
#[tokio::test]
async fn test_tampered_envelope_quarantined() {
    let mut router = MrpRouter::new();
    let mut envelope = MeaningEnvelopeBuilder::new(test_wid())
        .payload(MeaningPayload {
            interpretation: "original".into(),
            confidence: 0.9,
            ambiguity_preserved: true,
            evidence_refs: vec![],
        })
        .build()
        .unwrap();

    // Tamper with body without updating hash
    envelope.body = maple_kernel_mrp::TypedPayload::Meaning(MeaningPayload {
        interpretation: "TAMPERED".into(),
        confidence: 0.1,
        ambiguity_preserved: false,
        evidence_refs: vec![],
    });

    let decision = router.route(&envelope).await.unwrap();
    assert!(
        matches!(decision, RouteDecision::Quarantine(_)),
        "Tampered envelope must be quarantined"
    );
}

/// Type mismatch (header vs payload) is rejected.
#[tokio::test]
async fn test_type_mismatch_rejected() {
    let mut router = MrpRouter::new();
    let mut envelope = MeaningEnvelopeBuilder::new(test_wid())
        .payload(MeaningPayload {
            interpretation: "test".into(),
            confidence: 0.9,
            ambiguity_preserved: true,
            evidence_refs: vec![],
        })
        .build()
        .unwrap();

    // Change header type without changing payload
    envelope.header.resonance_type = ResonanceType::Commitment;
    envelope.integrity.hash = envelope.compute_hash();

    let decision = router.route(&envelope).await.unwrap();
    assert!(
        matches!(decision, RouteDecision::Reject(RejectionReason::TypeMismatch { .. })),
        "Type mismatch must be rejected"
    );
}

/// MEANING routes to cognition, never to Gate.
#[tokio::test]
async fn test_meaning_routes_cognition_not_gate() {
    let mut router = MrpRouter::new();
    let envelope = MeaningEnvelopeBuilder::new(test_wid())
        .payload(MeaningPayload {
            interpretation: "test".into(),
            confidence: 0.9,
            ambiguity_preserved: true,
            evidence_refs: vec![],
        })
        .build()
        .unwrap();

    let decision = router.route(&envelope).await.unwrap();
    assert!(matches!(decision, RouteDecision::DeliverToCognition(_)));
}

/// COMMITMENT routes to Gate.
#[tokio::test]
async fn test_commitment_routes_to_gate() {
    let mut router = MrpRouter::new();
    let envelope = CommitmentEnvelopeBuilder::new(test_wid())
        .payload(CommitmentPayload {
            commitment_id: CommitmentId::new(),
            scope: CommitmentScope {
                effect_domain: EffectDomain::Communication,
                targets: vec![other_wid()],
                constraints: vec![],
            },
            affected_parties: vec![],
            evidence: vec![],
        })
        .build()
        .unwrap();

    let decision = router.route(&envelope).await.unwrap();
    assert!(matches!(decision, RouteDecision::RouteToGate));
}
