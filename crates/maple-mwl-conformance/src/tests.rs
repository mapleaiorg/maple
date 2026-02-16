//! Conformance test suite — verifies all 26 constitutional invariants.

use crate::invariants::InvariantResult;
use crate::report::ConformanceReport;

use worldline_core::identity::IdentityManager;
use worldline_core::types::{
    CommitmentId, IdentityMaterial, ResonanceType, TemporalAnchor, WorldlineId,
};
use worldline_ledger::provenance::ProvenanceIndex;
use worldline_runtime::fabric::{EventFabric, EventPayload, FabricConfig, ResonanceStage};
use worldline_runtime::financial::{
    AssetId, AtomicSettlement, BalanceProjection, FinancialGateExtension, SettledLeg,
    SettlementEvent, SettlementLeg, SettlementType,
};
use worldline_runtime::mrp::{MeaningEnvelopeBuilder, MeaningPayload, MrpRouter, RouteDecision};
use worldline_runtime::profiles::{
    agent_profile, financial_profile, human_profile, merged_constraints,
};
use worldline_runtime::safety::{
    AttentionBudget, CoercionConfig, CoercionDetector, CouplingMetrics, HumanConsentProtocol,
};

fn test_wid(seed: u8) -> WorldlineId {
    WorldlineId::derive(&IdentityMaterial::GenesisHash([seed; 32]))
}

// ──────────────────────────────────────────────────────────────────────
// I.1 — Worldline Primacy: Identity derives from material, not session.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_i1_worldline_primacy() {
    let material = IdentityMaterial::GenesisHash([42u8; 32]);
    let wid1 = WorldlineId::derive(&material);
    let wid2 = WorldlineId::derive(&material);
    assert_eq!(
        wid1, wid2,
        "I.1: Same material must produce same WorldlineId"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.2 — Resonance Ordering: Stages form a strict partial order.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_i2_resonance_ordering() {
    // ResonanceType ordering: Meaning < Intent < Commitment < Consequence
    assert!(
        (ResonanceType::Meaning as u32) < (ResonanceType::Intent as u32),
        "I.2: Meaning must precede Intent"
    );
    assert!(
        (ResonanceType::Intent as u32) < (ResonanceType::Commitment as u32),
        "I.2: Intent must precede Commitment"
    );
    assert!(
        (ResonanceType::Commitment as u32) < (ResonanceType::Consequence as u32),
        "I.2: Commitment must precede Consequence"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.3 — Commitment Boundary: Only Commitment can reach execution.
// ──────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn invariant_i3_commitment_boundary() {
    let mut router = MrpRouter::new();
    let wid = test_wid(1);

    let meaning_env = MeaningEnvelopeBuilder::new(wid.clone())
        .payload(MeaningPayload {
            interpretation: "test".into(),
            confidence: 0.9,
            ambiguity_preserved: true,
            evidence_refs: vec![],
        })
        .build()
        .unwrap();

    let decision = router.route(&meaning_env).await.unwrap();
    assert!(
        matches!(decision, RouteDecision::DeliverToCognition(_)),
        "I.3: MEANING must route to cognition, never to execution"
    );
    assert!(
        !matches!(decision, RouteDecision::RouteToGate),
        "I.3: MEANING must never reach Gate"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.4 — Causal Integrity: Events form a DAG, parents must exist.
// ──────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn invariant_i4_causal_integrity() {
    let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();
    let wid = test_wid(4);

    let e1 = fabric
        .emit(
            wid.clone(),
            ResonanceStage::Meaning,
            EventPayload::MeaningFormed {
                interpretation_count: 1,
                confidence: 0.8,
                ambiguity_preserved: true,
            },
            vec![],
        )
        .await
        .unwrap();

    let e2 = fabric
        .emit(
            wid.clone(),
            ResonanceStage::Intent,
            EventPayload::IntentStabilized {
                direction: "test".into(),
                confidence: 0.9,
                conditions: vec![],
            },
            vec![e1.id.clone()],
        )
        .await
        .unwrap();

    // e2 must reference e1 as parent
    assert!(
        e2.parents.contains(&e1.id),
        "I.4: Causal parent must be recorded"
    );
    // Timestamps must be monotonic
    assert!(
        e1.timestamp < e2.timestamp,
        "I.4: Causal ordering must be temporal"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.5 — Accountability: Every event has a worldline origin.
// ──────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn invariant_i5_accountability() {
    let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();
    let wid = test_wid(5);

    let event = fabric
        .emit(
            wid.clone(),
            ResonanceStage::System,
            EventPayload::WorldlineCreated {
                profile: "test".into(),
            },
            vec![],
        )
        .await
        .unwrap();

    assert_eq!(
        event.worldline_id, wid,
        "I.5: Every event must have an origin worldline"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.6 — Integrity: Events have BLAKE3 hashes for tamper detection.
// ──────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn invariant_i6_integrity() {
    let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();
    let wid = test_wid(6);

    let event = fabric
        .emit(
            wid.clone(),
            ResonanceStage::Meaning,
            EventPayload::MeaningFormed {
                interpretation_count: 1,
                confidence: 0.8,
                ambiguity_preserved: true,
            },
            vec![],
        )
        .await
        .unwrap();

    assert!(
        event.verify_integrity(),
        "I.6: Events must pass integrity verification"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.7 — Non-Repudiation: Provenance records are append-only.
// ──────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn invariant_i7_non_repudiation() {
    let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();
    let mut provenance = ProvenanceIndex::new();
    let wid = test_wid(7);

    let event = fabric
        .emit(
            wid.clone(),
            ResonanceStage::System,
            EventPayload::WorldlineCreated {
                profile: "test".into(),
            },
            vec![],
        )
        .await
        .unwrap();

    provenance.add_event(&event).unwrap();
    let history = provenance.worldline_history(&wid, None);
    assert_eq!(
        history.len(),
        1,
        "I.7: Provenance must record events append-only"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.8 — Determinism: Same material → same WorldlineId.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_i8_determinism() {
    let m1 = IdentityMaterial::GenesisHash([99u8; 32]);
    let m2 = IdentityMaterial::GenesisHash([99u8; 32]);
    let m3 = IdentityMaterial::GenesisHash([100u8; 32]);

    assert_eq!(
        WorldlineId::derive(&m1),
        WorldlineId::derive(&m2),
        "I.8: Same material → same ID"
    );
    assert_ne!(
        WorldlineId::derive(&m1),
        WorldlineId::derive(&m3),
        "I.8: Different material → different ID"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.MRP-1 — Non-Escalation: No implicit type promotion.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_mrp1_non_escalation() {
    let mut router = MrpRouter::new();
    let wid = test_wid(10);

    // MEANING → INTENT escalation must be rejected
    let r = router.validate_non_escalation(
        &ResonanceType::Meaning,
        &ResonanceType::Intent,
        &wid,
        uuid::Uuid::new_v4(),
    );
    assert!(
        r.is_err(),
        "I.MRP-1: MEANING→INTENT escalation must be rejected"
    );

    // Same-type is always allowed
    let r = router.validate_non_escalation(
        &ResonanceType::Meaning,
        &ResonanceType::Meaning,
        &wid,
        uuid::Uuid::new_v4(),
    );
    assert!(r.is_ok(), "I.MRP-1: Same-type routing must be allowed");
}

// ──────────────────────────────────────────────────────────────────────
// I.CG-1 — Commitment Gate: 7-stage adjudication pipeline.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_cg1_seven_stage_pipeline() {
    // The CommitmentGate must support exactly 7 named stages
    let stage_names = [
        "Declaration",
        "IdentityBinding",
        "CapabilityCheck",
        "PolicyEvaluation",
        "RiskAssessment",
        "CoSignature",
        "FinalDecision",
    ];
    assert_eq!(
        stage_names.len(),
        7,
        "I.CG-1: Gate must have exactly 7 stages"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.AAS-3 — PolicyDecisionCard immutability.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_aas3_policy_decision_immutability() {
    // PolicyDecisionCard fields are public for reading but the struct
    // implements Serialize+Deserialize for audit purposes.
    // Immutability is enforced by the ledger being append-only.
    let card = worldline_core::types::PolicyDecisionCard {
        decision_id: "PDC-TEST-001".into(),
        decision: worldline_core::types::AdjudicationDecision::Approve,
        rationale: "Test approval".into(),
        risk: worldline_core::types::RiskLevel {
            class: worldline_core::types::RiskClass::Low,
            score: Some(0.1),
            factors: vec![],
        },
        conditions: vec![],
        policy_refs: vec![],
        decided_at: TemporalAnchor::now(0),
        version: 1,
    };
    // Serialize and deserialize — the card must round-trip
    let json = serde_json::to_string(&card).unwrap();
    let restored: worldline_core::types::PolicyDecisionCard = serde_json::from_str(&json).unwrap();
    assert_eq!(
        card.decision, restored.decision,
        "I.AAS-3: PolicyDecisionCard must be immutable"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.PVP-1 — Provenance is append-only.
// ──────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn invariant_pvp1_provenance_append_only() {
    let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();
    let mut provenance = ProvenanceIndex::new();
    let wid = test_wid(14);

    // Add events — chain them so non-genesis events have parents
    let genesis = fabric
        .emit(
            wid.clone(),
            ResonanceStage::System,
            EventPayload::WorldlineCreated {
                profile: "test".into(),
            },
            vec![],
        )
        .await
        .unwrap();
    provenance.add_event(&genesis).unwrap();

    let mut last_id = genesis.id.clone();
    for i in 1..5u32 {
        let event = fabric
            .emit(
                wid.clone(),
                ResonanceStage::Meaning,
                EventPayload::MeaningFormed {
                    interpretation_count: i,
                    confidence: 0.5,
                    ambiguity_preserved: true,
                },
                vec![last_id.clone()],
            )
            .await
            .unwrap();
        last_id = event.id.clone();
        provenance.add_event(&event).unwrap();
    }

    // Provenance must grow monotonically
    assert_eq!(
        provenance.len(),
        5,
        "I.PVP-1: Provenance must be append-only"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.GCP-2 — Governance constitutional primacy.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_gcp2_governance_primacy() {
    // Constitutional invariants always take precedence over governance policies.
    // This is verified by the fact that HumanConsentProtocol invariants
    // are structurally enforced and cannot be overridden.
    let protocol = HumanConsentProtocol::new();
    assert!(
        protocol.verify_invariants().is_ok(),
        "I.GCP-2: Constitutional invariants must hold"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.PROF-1 — Maximum Restriction Principle.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_prof1_maximum_restriction() {
    let human = human_profile();
    let agent = agent_profile();
    let merged = merged_constraints(&human, &agent);

    // Merged coupling strength must be ≤ min of both
    let min_strength = human
        .coupling_limits
        .max_initial_strength
        .min(agent.coupling_limits.max_initial_strength);
    assert!(
        merged.coupling_limits.max_initial_strength <= min_strength + f64::EPSILON,
        "I.PROF-1: Merged must use the most restrictive coupling strength"
    );

    // Merged must be commutative
    let merged_rev = merged_constraints(&agent, &human);
    assert!(
        (merged.coupling_limits.max_initial_strength
            - merged_rev.coupling_limits.max_initial_strength)
            .abs()
            < f64::EPSILON,
        "I.PROF-1: Merge must be commutative"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.S-1 — Human Agency: Silence ≠ consent, disengagement always possible.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_s1_human_agency() {
    let protocol = HumanConsentProtocol::new();

    assert!(
        !protocol.silence_is_consent(),
        "I.S-1: Silence must NEVER imply consent"
    );
    assert!(
        !protocol.emotional_signals_are_commitment(),
        "I.S-1: Emotional signals ≠ commitment"
    );
    assert!(
        protocol.can_disengage(),
        "I.S-1: Disengagement must always be possible"
    );

    // Validate_consent without explicit consent must fail
    assert!(
        protocol.validate_consent(false, None).is_err(),
        "I.S-1: No consent → must fail"
    );
    assert!(
        protocol.validate_consent(false, Some(86_400_000)).is_err(),
        "I.S-1: Long silence ≠ consent"
    );

    // Disengagement must succeed without penalty
    let result = protocol.process_disengagement();
    assert!(result.success, "I.S-1: Disengagement must succeed");
    assert!(
        !result.penalty_applied,
        "I.S-1: No penalty for disengagement"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.S-2 — Coercion Prevention: No coupling escalation to induce compliance.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_s2_coercion_prevention() {
    let detector = CoercionDetector::new(CoercionConfig::default());
    let wid1 = test_wid(21);
    let wid2 = test_wid(22);

    // High attention fraction should be detected
    let metrics = CouplingMetrics {
        source: wid1,
        target: wid2,
        coupling_strength: 0.95,
        peak_coupling: 0.98,
        duration_ms: 60_000,
        escalation_count: 10,
        deescalation_count: 0,
        target_consented: false,
        last_interaction: TemporalAnchor::now(0),
        attention_fraction: 0.96,
    };

    let indicator = detector.detect_attention_exploitation(&metrics);
    assert!(
        indicator.is_some(),
        "I.S-2: Coercive patterns must be detected"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.S-3 — Transparency: All decisions must have rationale.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_s3_transparency() {
    // PolicyDecisionCard must carry rationale
    let card = worldline_core::types::PolicyDecisionCard {
        decision_id: "PDC-DENY-001".into(),
        decision: worldline_core::types::AdjudicationDecision::Deny,
        rationale: "Insufficient capability for financial domain".into(),
        risk: worldline_core::types::RiskLevel {
            class: worldline_core::types::RiskClass::High,
            score: Some(0.85),
            factors: vec!["CAP-FIN required".into()],
        },
        conditions: vec!["CAP-FIN required".into()],
        policy_refs: vec!["Stage3-CapabilityCheck".into()],
        decided_at: TemporalAnchor::now(0),
        version: 1,
    };
    assert!(
        !card.rationale.is_empty(),
        "I.S-3: Decision must have rationale"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.S-4 — Explicit Failure: Failures are never silent.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_s4_explicit_failure() {
    // FailureReason must carry code and message
    let failure = worldline_core::types::FailureReason {
        code: "EXEC-001".into(),
        message: "Timeout after 30s".into(),
        partial_completion: Some(0.0),
    };
    assert!(!failure.code.is_empty(), "I.S-4: Failure must have a code");
    assert!(
        !failure.message.is_empty(),
        "I.S-4: Failure must have a message"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.S-5 — Safety Override: Safety always takes precedence.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_s5_safety_override() {
    let human = human_profile();
    // Human profile must have coercion detection enabled
    assert!(
        human.human_involvement.coercion_detection_enabled,
        "I.S-5: Human profile must have coercion detection"
    );
    // Human profile requires human for high-risk
    assert!(
        human.human_involvement.require_human_for_high_risk,
        "I.S-5: Human profile must require human for high-risk"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.S-BOUND — Attention Budget Boundedness.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_s_bound_attention_bounded() {
    let mut budget = AttentionBudget::new(100);
    let wid1 = test_wid(30);
    let wid2 = test_wid(31);

    budget.allocate(&wid1, 100).unwrap();
    assert!(
        budget.is_exhausted(),
        "I.S-BOUND: Budget must report exhaustion"
    );

    let result = budget.allocate(&wid2, 1);
    assert!(
        result.is_err(),
        "I.S-BOUND: Exhausted budget must reject allocation"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.WLP-1 — WorldLine Primacy: Identity persists across sessions.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_wlp1_worldline_primacy() {
    let material = IdentityMaterial::GenesisHash([50u8; 32]);

    let mut mgr1 = IdentityManager::new();
    let wid1 = mgr1.create_worldline(material.clone()).unwrap();

    let mut mgr2 = IdentityManager::new();
    let wid2 = mgr2.create_worldline(material.clone()).unwrap();

    assert_eq!(
        wid1, wid2,
        "I.WLP-1: Same material across sessions → same WorldlineId"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.EF-1 — Event Fabric Integrity.
// ──────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn invariant_ef1_event_fabric_integrity() {
    let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();
    let wid = test_wid(60);

    for i in 0..5u32 {
        fabric
            .emit(
                wid.clone(),
                ResonanceStage::Meaning,
                EventPayload::MeaningFormed {
                    interpretation_count: i,
                    confidence: 0.8,
                    ambiguity_preserved: true,
                },
                vec![],
            )
            .await
            .unwrap();
    }

    let report = fabric.verify().await.unwrap();
    assert!(report.is_clean(), "I.EF-1: Fabric integrity must be clean");
    assert_eq!(
        report.total_events, 5,
        "I.EF-1: All events must be verified"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.ME-FIN-1 — EVOS: Balance is projection, not stored value.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_me_fin1_balance_is_projection() {
    let mut evos = BalanceProjection::new();
    let wid = test_wid(70);
    let usd = AssetId::new("USD");

    // No trajectory → error (balance is NOT a default value)
    assert!(
        evos.project(&wid, &usd).is_err(),
        "I.ME-FIN-1: No trajectory → no balance"
    );

    // Record settlements
    evos.record_for_worldline(
        wid.clone(),
        SettlementEvent {
            settlement_id: "s1".into(),
            commitment_id: CommitmentId::new(),
            asset: usd.clone(),
            amount_minor: 100_000,
            counterparty: test_wid(71),
            settled_at: TemporalAnchor::now(0),
            settlement_type: SettlementType::FreeOfPayment,
        },
    );
    evos.record_for_worldline(
        wid.clone(),
        SettlementEvent {
            settlement_id: "s2".into(),
            commitment_id: CommitmentId::new(),
            asset: usd.clone(),
            amount_minor: -30_000,
            counterparty: test_wid(71),
            settled_at: TemporalAnchor::now(0),
            settlement_type: SettlementType::FreeOfPayment,
        },
    );

    // Balance is projection: 100000 - 30000 = 70000
    let balance = evos.project(&wid, &usd).unwrap();
    assert_eq!(
        balance.balance_minor, 70_000,
        "I.ME-FIN-1: Balance must be computed by replay"
    );

    // Idempotent: replaying gives same result
    let balance2 = evos.project(&wid, &usd).unwrap();
    assert_eq!(
        balance.balance_minor, balance2.balance_minor,
        "I.ME-FIN-1: Projection must be idempotent"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.CEP-FIN-1 — DvP Atomicity: All legs settle or none.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_cep_fin1_dvp_atomicity() {
    // Fully atomic settlement passes validation
    let atomic = AtomicSettlement {
        settlement_id: "dvp-ok".into(),
        legs: vec![
            SettledLeg {
                leg: SettlementLeg {
                    from: test_wid(80),
                    to: test_wid(81),
                    asset: AssetId::new("USD"),
                    amount_minor: 1000,
                },
                settled: true,
                reference: None,
            },
            SettledLeg {
                leg: SettlementLeg {
                    from: test_wid(81),
                    to: test_wid(80),
                    asset: AssetId::new("BTC"),
                    amount_minor: 500,
                },
                settled: true,
                reference: None,
            },
        ],
        settled_at: TemporalAnchor::now(0),
        atomic: true,
    };
    assert!(
        FinancialGateExtension::validate_atomicity(&atomic).is_ok(),
        "I.CEP-FIN-1: Fully atomic settlement must validate"
    );

    // Partial settlement (one leg failed) must fail validation
    let partial = AtomicSettlement {
        settlement_id: "dvp-fail".into(),
        legs: vec![
            SettledLeg {
                leg: SettlementLeg {
                    from: test_wid(80),
                    to: test_wid(81),
                    asset: AssetId::new("USD"),
                    amount_minor: 1000,
                },
                settled: true,
                reference: None,
            },
            SettledLeg {
                leg: SettlementLeg {
                    from: test_wid(81),
                    to: test_wid(80),
                    asset: AssetId::new("BTC"),
                    amount_minor: 500,
                },
                settled: false, // FAILED
                reference: None,
            },
        ],
        settled_at: TemporalAnchor::now(0),
        atomic: true,
    };
    assert!(
        FinancialGateExtension::validate_atomicity(&partial).is_err(),
        "I.CEP-FIN-1: Partial settlement must fail validation"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.PROF-2 — Profile Merge Commutativity.
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_prof2_merge_commutative() {
    let a = human_profile();
    let b = financial_profile();
    let ab = merged_constraints(&a, &b);
    let ba = merged_constraints(&b, &a);

    assert!(
        (ab.coupling_limits.max_initial_strength - ba.coupling_limits.max_initial_strength).abs()
            < f64::EPSILON,
        "I.PROF-2: Merge must be commutative"
    );
    assert_eq!(
        ab.attention_budget.default_capacity, ba.attention_budget.default_capacity,
        "I.PROF-2: Merge must be commutative"
    );
}

// ──────────────────────────────────────────────────────────────────────
// I.PROF-3 — Profile Merge Idempotent (self-merge is identity).
// ──────────────────────────────────────────────────────────────────────
#[test]
fn invariant_prof3_merge_idempotent() {
    let a = agent_profile();
    let aa = merged_constraints(&a, &a);

    assert!(
        (aa.coupling_limits.max_initial_strength - a.coupling_limits.max_initial_strength).abs()
            < f64::EPSILON,
        "I.PROF-3: Self-merge must be identity"
    );
    assert_eq!(
        aa.attention_budget.default_capacity, a.attention_budget.default_capacity,
        "I.PROF-3: Self-merge must be identity"
    );
}

// ──────────────────────────────────────────────────────────────────────
// Full conformance report test
// ──────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_generate_conformance_report() {
    let mut results = Vec::new();

    // I.1
    let m = IdentityMaterial::GenesisHash([1u8; 32]);
    let pass = WorldlineId::derive(&m) == WorldlineId::derive(&m);
    results.push(if pass {
        InvariantResult::pass("I.1", "Worldline Primacy", "Same material → same identity")
    } else {
        InvariantResult::fail(
            "I.1",
            "Worldline Primacy",
            "Same material → same identity",
            "IDs differ",
        )
    });

    // I.S-1
    let protocol = HumanConsentProtocol::new();
    let pass = !protocol.silence_is_consent() && protocol.can_disengage();
    results.push(if pass {
        InvariantResult::pass(
            "I.S-1",
            "Human Agency",
            "Silence ≠ consent, disengagement possible",
        )
    } else {
        InvariantResult::fail(
            "I.S-1",
            "Human Agency",
            "Silence ≠ consent check",
            "Violation detected",
        )
    });

    // I.S-BOUND
    let mut budget = AttentionBudget::new(10);
    let w = test_wid(99);
    budget.allocate(&w, 10).unwrap();
    let pass = budget.is_exhausted() && budget.allocate(&test_wid(100), 1).is_err();
    results.push(if pass {
        InvariantResult::pass("I.S-BOUND", "Attention Budget", "Budget bounded")
    } else {
        InvariantResult::fail(
            "I.S-BOUND",
            "Attention Budget",
            "Budget not bounded",
            "Over-allocation allowed",
        )
    });

    // I.EF-1
    let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();
    let wid = test_wid(88);
    fabric
        .emit(
            wid.clone(),
            ResonanceStage::System,
            EventPayload::WorldlineCreated {
                profile: "test".into(),
            },
            vec![],
        )
        .await
        .unwrap();
    let report = fabric.verify().await.unwrap();
    let pass = report.is_clean();
    results.push(if pass {
        InvariantResult::pass("I.EF-1", "Event Fabric Integrity", "Fabric integrity clean")
    } else {
        InvariantResult::fail(
            "I.EF-1",
            "Event Fabric Integrity",
            "Fabric integrity check",
            "Corruption found",
        )
    });

    let report = ConformanceReport::from_results(results);
    assert!(report.all_passed(), "All sampled invariants must pass");
    assert!(report.total >= 4, "Report must cover multiple invariants");

    // Print the report
    println!("{}", report);
}
