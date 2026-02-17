//! Full Resonance Lifecycle Integration Test
//!
//! THE canonical integration test: exercises every kernel component
//! in the order mandated by the Resonance Architecture.
//!
//! Flow: Presence → Coupling → Meaning → Intent → Commitment → Consequence

use worldline_core::types::{CommitmentScope, ConfidenceProfile, EffectDomain};
use worldline_runtime::fabric::{CouplingScope, EventPayload, ResonanceStage};
use worldline_runtime::gate::{AdjudicationResult, CommitmentOutcome};
use worldline_runtime::mrp::{
    CommitmentEnvelopeBuilder, CommitmentPayload, IntentEnvelopeBuilder, IntentPayload,
    MeaningEnvelopeBuilder, MeaningPayload, RouteDecision,
};

use crate::helpers::{KernelOptions, TestKernel};

/// Test the complete resonance flow from presence through consequence.
#[tokio::test]
async fn test_full_resonance_lifecycle() {
    // ── 1. SETUP ──────────────────────────────────────────────────────
    let mut kernel = TestKernel::new(KernelOptions::default()).await;

    // ── 2. PRESENCE: Create two WorldLines ────────────────────────────
    let agent_a = kernel.create_worldline(1);
    let agent_b = kernel.create_worldline(2);

    // Grant communication capability to agent_a
    kernel.grant_capability(&agent_a, "CAP-COMM", EffectDomain::Communication);

    // Emit genesis events (establishes presence in the fabric)
    let genesis_a = kernel.emit_genesis(&agent_a).await;
    let _genesis_b = kernel.emit_genesis(&agent_b).await;

    // Both worldlines are present and discoverable
    let history_a = kernel.provenance.worldline_history(&agent_a, None);
    let history_b = kernel.provenance.worldline_history(&agent_b, None);
    assert_eq!(history_a.len(), 1, "Agent A should have genesis event");
    assert_eq!(history_b.len(), 1, "Agent B should have genesis event");

    // ── 3. COUPLING: Establish coupling between A and B ───────────────
    let coupling_event = kernel
        .fabric
        .emit(
            agent_a.clone(),
            ResonanceStage::Coupling,
            EventPayload::CouplingEstablished {
                target: agent_b.clone(),
                intensity: 0.5,
                scope: CouplingScope {
                    domains: vec!["communication".into()],
                    constraints: vec![],
                },
            },
            vec![genesis_a.id.clone()],
        )
        .await
        .unwrap();
    kernel.provenance.add_event(&coupling_event).unwrap();

    // Verify coupling event is in provenance
    let history_a = kernel.provenance.worldline_history(&agent_a, None);
    assert_eq!(history_a.len(), 2, "Agent A should have genesis + coupling");

    // ── 4. MEANING: Agent-A forms meaning ─────────────────────────────
    let meaning_event = kernel
        .emit_meaning(&agent_a, vec![coupling_event.id.clone()])
        .await;

    // Create MRP MEANING envelope and route it
    let meaning_envelope = MeaningEnvelopeBuilder::new(agent_a.clone())
        .payload(MeaningPayload {
            interpretation: "Agent B is available for communication".into(),
            confidence: 0.85,
            ambiguity_preserved: true,
            evidence_refs: vec![coupling_event.id.clone()],
        })
        .build()
        .unwrap();

    let decision = kernel.mrp_router.route(&meaning_envelope).await.unwrap();

    // MEANING stays in cognition — does NOT reach execution
    assert!(
        matches!(decision, RouteDecision::DeliverToCognition(_)),
        "MEANING must route to cognition, not execution"
    );

    // ── 5. INTENT: Stabilize intent from meaning ──────────────────────
    let intent_event = kernel
        .emit_intent(&agent_a, vec![meaning_event.id.clone()])
        .await;

    // Create MRP INTENT envelope and route it
    let intent_envelope = IntentEnvelopeBuilder::new(agent_a.clone())
        .payload(IntentPayload {
            direction: "Send greeting to Agent B".into(),
            confidence: ConfidenceProfile::new(0.9, 0.85, 0.9, 0.8),
            conditions: vec![],
            derived_from: Some(meaning_event.id.clone()),
        })
        .build()
        .unwrap();

    let decision = kernel.mrp_router.route(&intent_envelope).await.unwrap();

    // INTENT stays in cognition — non-executable
    assert!(
        matches!(decision, RouteDecision::DeliverToCognition(_)),
        "INTENT must route to cognition, not execution"
    );

    // ── 6. COMMITMENT: Declare commitment from intent ─────────────────
    let declaration = kernel.build_declaration(
        agent_a.clone(),
        intent_event.id.clone(),
        EffectDomain::Communication,
        "CAP-COMM",
        vec![agent_b.clone()],
    );
    let commitment_id = declaration.id.clone();

    // Route commitment envelope through MRP → Gate
    let commitment_envelope = CommitmentEnvelopeBuilder::new(agent_a.clone())
        .payload(CommitmentPayload {
            commitment_id: commitment_id.clone(),
            scope: CommitmentScope {
                effect_domain: EffectDomain::Communication,
                targets: vec![agent_b.clone()],
                constraints: vec![],
            },
            affected_parties: vec![],
            evidence: vec![],
        })
        .build()
        .unwrap();

    let mrp_decision = kernel.mrp_router.route(&commitment_envelope).await.unwrap();
    assert!(
        matches!(mrp_decision, RouteDecision::RouteToGate),
        "COMMITMENT must route to Gate"
    );

    // Submit through the 7-stage Commitment Gate
    let result = kernel.gate.submit(declaration).await.unwrap();

    // All 7 stages pass → APPROVED
    assert!(
        matches!(result, AdjudicationResult::Approved { .. }),
        "Commitment should be approved: {:?}",
        result
    );

    // Verify ledger entry exists with PolicyDecisionCard
    let ledger_entry = kernel.gate.ledger().history(&commitment_id).unwrap();
    assert_eq!(
        ledger_entry.decision.decision,
        worldline_core::types::AdjudicationDecision::Approve
    );

    // ── 7. CONSEQUENCE: Record outcome ────────────────────────────────
    kernel
        .gate
        .record_outcome(&commitment_id, CommitmentOutcome::Fulfilled)
        .await
        .unwrap();

    // Verify outcome recorded in ledger
    let entry = kernel.gate.ledger().history(&commitment_id).unwrap();
    assert!(
        entry.lifecycle.len() >= 3,
        "Lifecycle should have declared, approved, fulfilled"
    );

    // ── 8. PROVENANCE: Verify full audit trail ────────────────────────
    // Worldline history for agent_a shows all events
    let full_history = kernel.provenance.worldline_history(&agent_a, None);
    assert!(
        full_history.len() >= 3,
        "Agent A should have genesis, coupling, meaning, intent events"
    );

    // Causal path from genesis to meaning exists
    let path = kernel
        .provenance
        .causal_path(&genesis_a.id, &meaning_event.id);
    assert!(
        path.is_some(),
        "Causal path from genesis to meaning must exist"
    );

    // ── 9. VERIFY: Fabric integrity ───────────────────────────────────
    let integrity = kernel.fabric.verify().await.unwrap();
    assert!(integrity.is_clean(), "Fabric integrity must be clean");
}

/// Verify that events are causally ordered throughout the lifecycle.
#[tokio::test]
async fn test_causal_ordering_preserved() {
    let mut kernel = TestKernel::new(KernelOptions::default()).await;
    let wid = kernel.create_worldline(10);
    kernel.grant_capability(&wid, "CAP-COMM", EffectDomain::Communication);

    let genesis = kernel.emit_genesis(&wid).await;
    let meaning = kernel.emit_meaning(&wid, vec![genesis.id.clone()]).await;
    let intent = kernel.emit_intent(&wid, vec![meaning.id.clone()]).await;

    // Timestamps must be monotonically increasing
    assert!(genesis.timestamp < meaning.timestamp);
    assert!(meaning.timestamp < intent.timestamp);

    // Provenance ancestors of intent include meaning and genesis
    let ancestors = kernel.provenance.ancestors(&intent.id, None);
    assert_eq!(ancestors.len(), 2);
}

/// Verify that batch events maintain ordering.
#[tokio::test]
async fn test_batch_events_ordered() {
    let mut kernel = TestKernel::new(KernelOptions::default()).await;
    let wid = kernel.create_worldline(20);

    let genesis = kernel.emit_genesis(&wid).await;

    let items = vec![
        (
            wid.clone(),
            ResonanceStage::Meaning,
            EventPayload::MeaningFormed {
                interpretation_count: 1,
                confidence: 0.7,
                ambiguity_preserved: true,
            },
            vec![genesis.id.clone()],
        ),
        (
            wid.clone(),
            ResonanceStage::Intent,
            EventPayload::IntentStabilized {
                direction: "forward".into(),
                confidence: 0.9,
                conditions: vec![],
            },
            vec![genesis.id.clone()],
        ),
    ];

    let events = kernel.fabric.emit_batch(items).await.unwrap();
    assert_eq!(events.len(), 2);
    assert!(events[0].timestamp < events[1].timestamp);
}
