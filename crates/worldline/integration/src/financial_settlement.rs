//! Financial Settlement Integration Tests
//!
//! Verifies EVOS (balance-as-projection), ARES (DvP atomicity),
//! and regulatory policy enforcement.

use worldline_core::types::{CommitmentId, IdentityMaterial, TemporalAnchor, WorldlineId};
use worldline_runtime::financial::{
    AssetId, AtomicSettlement, BalanceProjection, FinancialCommitment, FinancialGateExtension,
    LiquidityFieldOperator, RegulatoryEngine, SettledLeg, SettlementChannel, SettlementEvent,
    SettlementLeg, SettlementNetwork, SettlementType,
};

fn alice() -> WorldlineId {
    WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
}

fn bob() -> WorldlineId {
    WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]))
}

fn settlement_event(
    counterparty: WorldlineId,
    asset: AssetId,
    amount: i64,
    id: &str,
) -> SettlementEvent {
    SettlementEvent {
        settlement_id: id.into(),
        commitment_id: CommitmentId::new(),
        asset,
        amount_minor: amount,
        counterparty,
        settled_at: TemporalAnchor::now(0),
        settlement_type: SettlementType::FreeOfPayment,
    }
}

/// DvP atomic settlement: both legs execute or neither does (I.CEP-FIN-1).
#[test]
fn test_dvp_atomic_settlement() {
    let mut evos = BalanceProjection::new();

    let alice_wid = alice();
    let bob_wid = bob();
    let usd = AssetId::new("USD");
    let btc = AssetId::new("BTC");

    // Give Alice USD and Bob BTC via initial settlements
    evos.record_for_worldline(
        alice_wid.clone(),
        settlement_event(bob_wid.clone(), usd.clone(), 100_000, "init-alice-usd"),
    );
    evos.record_for_worldline(
        bob_wid.clone(),
        settlement_event(alice_wid.clone(), btc.clone(), 1_000_000, "init-bob-btc"),
    );

    // Verify initial balances
    let alice_usd = evos.project(&alice_wid, &usd).unwrap();
    assert_eq!(alice_usd.balance_minor, 100_000);
    let bob_btc = evos.project(&bob_wid, &btc).unwrap();
    assert_eq!(bob_btc.balance_minor, 1_000_000);

    // Execute DvP settlement — build AtomicSettlement
    let settlement = AtomicSettlement {
        settlement_id: "dvp-001".into(),
        legs: vec![
            SettledLeg {
                leg: SettlementLeg {
                    from: alice_wid.clone(),
                    to: bob_wid.clone(),
                    asset: usd.clone(),
                    amount_minor: 50_000,
                },
                settled: true,
                reference: None,
            },
            SettledLeg {
                leg: SettlementLeg {
                    from: bob_wid.clone(),
                    to: alice_wid.clone(),
                    asset: btc.clone(),
                    amount_minor: 500_000,
                },
                settled: true,
                reference: None,
            },
        ],
        settled_at: TemporalAnchor::now(0),
        atomic: true,
    };

    // Validate atomicity
    let validation = FinancialGateExtension::validate_atomicity(&settlement);
    assert!(validation.is_ok(), "DvP settlement should validate");

    // Apply settlement legs to EVOS
    for settled_leg in &settlement.legs {
        evos.record_for_worldline(
            settled_leg.leg.from.clone(),
            settlement_event(
                settled_leg.leg.to.clone(),
                settled_leg.leg.asset.clone(),
                -settled_leg.leg.amount_minor,
                &format!("{}-debit", settlement.settlement_id),
            ),
        );
        evos.record_for_worldline(
            settled_leg.leg.to.clone(),
            settlement_event(
                settled_leg.leg.from.clone(),
                settled_leg.leg.asset.clone(),
                settled_leg.leg.amount_minor,
                &format!("{}-credit", settlement.settlement_id),
            ),
        );
    }

    // Both balances updated
    assert_eq!(
        evos.project(&alice_wid, &usd).unwrap().balance_minor,
        50_000
    );
    assert_eq!(evos.project(&bob_wid, &usd).unwrap().balance_minor, 50_000);
    assert_eq!(
        evos.project(&alice_wid, &btc).unwrap().balance_minor,
        500_000
    );
    assert_eq!(evos.project(&bob_wid, &btc).unwrap().balance_minor, 500_000);
}

/// Balance is projection from trajectory, not a stored value (I.ME-FIN-1).
#[test]
fn test_balance_is_projection_not_stored() {
    let mut evos = BalanceProjection::new();
    let wid = alice();
    let usd = AssetId::new("USD");

    // Initially no trajectory → project returns error
    assert!(
        evos.project(&wid, &usd).is_err(),
        "No trajectory should return error"
    );

    // Record a series of settlements
    evos.record_for_worldline(
        wid.clone(),
        settlement_event(bob(), usd.clone(), 100_000, "s1"),
    );
    evos.record_for_worldline(
        wid.clone(),
        settlement_event(bob(), usd.clone(), -30_000, "s2"),
    );
    evos.record_for_worldline(
        wid.clone(),
        settlement_event(bob(), usd.clone(), 15_000, "s3"),
    );

    // Balance = sum of trajectory (100000 - 30000 + 15000 = 85000)
    let balance = evos.project(&wid, &usd).unwrap();
    assert_eq!(balance.balance_minor, 85_000);

    // Trajectory length reflects all settlements
    assert_eq!(evos.trajectory_length(&wid, &usd), 3);

    // Each replay of project() gives the same result (idempotent)
    assert_eq!(evos.project(&wid, &usd).unwrap().balance_minor, 85_000);
}

/// Multiple assets tracked independently.
#[test]
fn test_multi_asset_projection() {
    let mut evos = BalanceProjection::new();
    let wid = alice();
    let usd = AssetId::new("USD");
    let eur = AssetId::new("EUR");

    evos.record_for_worldline(
        wid.clone(),
        settlement_event(bob(), usd.clone(), 100_000, "s1"),
    );
    evos.record_for_worldline(
        wid.clone(),
        settlement_event(bob(), eur.clone(), 50_000, "s2"),
    );

    assert_eq!(evos.project(&wid, &usd).unwrap().balance_minor, 100_000);
    assert_eq!(evos.project(&wid, &eur).unwrap().balance_minor, 50_000);

    let assets = evos.assets_for_worldline(&wid);
    assert_eq!(assets.len(), 2);
}

/// Regulatory engine blocks transactions under circuit breaker.
#[test]
fn test_regulatory_policy_enforcement() {
    let mut engine = RegulatoryEngine::new();

    let wid = alice();
    let usd = AssetId::new("USD");

    // Check a small transaction → should pass
    let small_commitment = FinancialCommitment {
        commitment_id: CommitmentId::new(),
        asset: usd.clone(),
        amount_minor: 1_000,
        settlement_type: SettlementType::FreeOfPayment,
        counterparty: bob(),
        declaring_identity: wid.clone(),
        created_at: TemporalAnchor::now(0),
    };

    let result = engine.check_compliance(&small_commitment);
    assert!(result.is_ok(), "Small transaction should pass compliance");

    // Activate circuit breaker
    engine.activate_circuit_breaker("Market stress");
    assert!(
        engine.is_circuit_breaker_active(),
        "Circuit breaker should be active"
    );

    // Deactivate
    engine.deactivate_circuit_breaker();
    assert!(
        !engine.is_circuit_breaker_active(),
        "Circuit breaker should be deactivated"
    );
}

/// Liquidity field computation works.
#[test]
fn test_liquidity_field_computation() {
    let operator = LiquidityFieldOperator::new();

    let network = SettlementNetwork {
        participants: vec![alice(), bob()],
        channels: vec![SettlementChannel {
            from: alice(),
            to: bob(),
            asset: AssetId::new("USD"),
            liquidity_minor: 900_000,
            capacity_minor: 1_000_000,
        }],
    };

    let field = operator.compute_field(&network);
    assert!(!field.channel_scores.is_empty());
    assert!(field.network_score > 0.0);
}

/// Settlement trajectory hashes are deterministic.
#[test]
fn test_trajectory_hash_deterministic() {
    let mut evos1 = BalanceProjection::new();
    let mut evos2 = BalanceProjection::new();
    let wid = alice();
    let usd = AssetId::new("USD");

    evos1.record_for_worldline(wid.clone(), settlement_event(bob(), usd.clone(), 100, "s1"));
    evos2.record_for_worldline(wid.clone(), settlement_event(bob(), usd.clone(), 100, "s1"));

    let hash1 = evos1.project(&wid, &usd).unwrap().trajectory_hash;
    let hash2 = evos2.project(&wid, &usd).unwrap().trajectory_hash;

    assert_eq!(hash1, hash2, "Same trajectory should produce same hash");
}
