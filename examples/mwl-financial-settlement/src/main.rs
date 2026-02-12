//! MWL Example 13: Financial Settlement
//!
//! Demonstrates the financial extensions of the MWL kernel:
//!
//! 1. **EVOS (Balance-as-Projection)** — Balance computed by replaying the settlement
//!    trajectory, NEVER stored as a mutable value
//! 2. **DvP Atomicity** — Delivery-vs-Payment: all legs settle or none
//! 3. **Regulatory Engine** — AML, sanctions, capital adequacy, circuit breakers
//!
//! Constitutional Invariants demonstrated:
//! - I.ME-FIN-1: Balance computed from trajectory, not stored
//! - I.CEP-FIN-1: DvP/PvP required — partial settlement is a violation

use colored::Colorize;
use maple_kernel_financial::{
    AssetId, AtomicSettlement, BalanceProjection, FinancialCommitment, FinancialGateExtension,
    RegulatoryEngine, SettledLeg, SettlementEvent, SettlementLeg, SettlementType,
};
use maple_mwl_types::{CommitmentId, IdentityMaterial, TemporalAnchor, WorldlineId};

fn wid(seed: u8) -> WorldlineId {
    WorldlineId::derive(&IdentityMaterial::GenesisHash([seed; 32]))
}

fn separator() {
    println!("{}", "━".repeat(72).dimmed());
}

fn header(title: &str) {
    println!();
    println!("{}", "═".repeat(72).cyan());
    println!("  {}", title.cyan().bold());
    println!("{}", "═".repeat(72).cyan());
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .init();

    println!();
    println!("{}", "╔══════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║    MWL Example 13: Financial Settlement                     ║".cyan().bold());
    println!("{}", "╚══════════════════════════════════════════════════════════════╝".cyan());

    let alice = wid(1);
    let bob = wid(2);
    let usd = AssetId::new("USD");
    let btc = AssetId::new("BTC");

    // ── Part 1: EVOS — Balance as Projection ────────────────────────
    header("Part 1: EVOS — Balance as Projection (I.ME-FIN-1)");

    let mut evos = BalanceProjection::new();

    // Before any settlement, no balance exists
    let result = evos.project(&alice, &usd);
    println!("  {} Before any settlement:", "├".dimmed());
    println!("  {}   Alice USD balance: {}", "│".dimmed(),
        match &result {
            Ok(b) => format!("{}", b.balance_minor).yellow().to_string(),
            Err(e) => format!("ERROR: {} (correct — no trajectory yet)", e).red().to_string(),
        });

    // Record settlement events
    evos.record_for_worldline(
        alice.clone(),
        SettlementEvent {
            settlement_id: "s1".into(),
            commitment_id: CommitmentId::new(),
            asset: usd.clone(),
            amount_minor: 500_000, // +$5,000
            counterparty: bob.clone(),
            settled_at: TemporalAnchor::now(0),
            settlement_type: SettlementType::FreeOfPayment,
        },
    );

    evos.record_for_worldline(
        alice.clone(),
        SettlementEvent {
            settlement_id: "s2".into(),
            commitment_id: CommitmentId::new(),
            asset: usd.clone(),
            amount_minor: 300_000, // +$3,000
            counterparty: bob.clone(),
            settled_at: TemporalAnchor::now(0),
            settlement_type: SettlementType::FreeOfPayment,
        },
    );

    evos.record_for_worldline(
        alice.clone(),
        SettlementEvent {
            settlement_id: "s3".into(),
            commitment_id: CommitmentId::new(),
            asset: usd.clone(),
            amount_minor: -150_000, // -$1,500
            counterparty: bob.clone(),
            settled_at: TemporalAnchor::now(0),
            settlement_type: SettlementType::FreeOfPayment,
        },
    );

    println!("  {} After 3 settlements:", "├".dimmed());
    println!("  {}   +$5,000 → +$3,000 → -$1,500", "│".dimmed());

    let balance = evos.project(&alice, &usd).unwrap();
    println!("  {} Alice USD projected balance: {} (${:.2})", "├".dimmed(),
        format!("{}", balance.balance_minor).green().bold(),
        balance.balance_minor as f64 / 100.0);

    // Verify idempotency
    let balance2 = evos.project(&alice, &usd).unwrap();
    println!("  {} Projection idempotent: {}", "└".dimmed(),
        if balance.balance_minor == balance2.balance_minor {
            "YES — replay always gives same result".green()
        } else {
            "FAIL".red()
        });

    // ── Part 2: DvP Atomicity ───────────────────────────────────────
    header("Part 2: DvP Atomicity (I.CEP-FIN-1)");

    // Successful DvP: Alice sends USD, Bob sends BTC
    let dvp_ok = AtomicSettlement {
        settlement_id: "dvp-001".into(),
        legs: vec![
            SettledLeg {
                leg: SettlementLeg {
                    from: alice.clone(),
                    to: bob.clone(),
                    asset: usd.clone(),
                    amount_minor: 100_000,
                },
                settled: true,
                reference: Some("leg-usd-001".into()),
            },
            SettledLeg {
                leg: SettlementLeg {
                    from: bob.clone(),
                    to: alice.clone(),
                    asset: btc.clone(),
                    amount_minor: 5_000,
                },
                settled: true,
                reference: Some("leg-btc-001".into()),
            },
        ],
        settled_at: TemporalAnchor::now(0),
        atomic: true,
    };

    let validation = FinancialGateExtension::validate_atomicity(&dvp_ok);
    println!("  {} DvP (all legs settled):", "├".dimmed());
    println!("  {}   Leg 1: Alice → Bob  $1,000 USD  [{}]", "│".dimmed(), "settled".green());
    println!("  {}   Leg 2: Bob → Alice  50 BTC      [{}]", "│".dimmed(), "settled".green());
    println!("  {}   Atomicity: {}", "│".dimmed(),
        if validation.is_ok() { "VALID".green().bold() } else { "INVALID".red().bold() });

    separator();

    // Failed DvP: one leg didn't settle
    let dvp_partial = AtomicSettlement {
        settlement_id: "dvp-002".into(),
        legs: vec![
            SettledLeg {
                leg: SettlementLeg {
                    from: alice.clone(),
                    to: bob.clone(),
                    asset: usd.clone(),
                    amount_minor: 100_000,
                },
                settled: true,
                reference: None,
            },
            SettledLeg {
                leg: SettlementLeg {
                    from: bob.clone(),
                    to: alice.clone(),
                    asset: btc.clone(),
                    amount_minor: 5_000,
                },
                settled: false, // FAILED
                reference: None,
            },
        ],
        settled_at: TemporalAnchor::now(0),
        atomic: true,
    };

    let validation = FinancialGateExtension::validate_atomicity(&dvp_partial);
    println!("  {} DvP (one leg failed):", "├".dimmed());
    println!("  {}   Leg 1: Alice → Bob  $1,000 USD  [{}]", "│".dimmed(), "settled".green());
    println!("  {}   Leg 2: Bob → Alice  50 BTC      [{}]", "│".dimmed(), "FAILED".red());
    println!("  {}   Atomicity: {}", "└".dimmed(),
        if validation.is_err() {
            "VIOLATION — partial settlement detected".red().bold()
        } else {
            "ERROR: should have failed".red().bold()
        });

    // ── Part 3: Regulatory Engine ───────────────────────────────────
    header("Part 3: Regulatory Engine");

    let mut engine = RegulatoryEngine::new();

    // Normal transaction
    let commitment = FinancialCommitment {
        commitment_id: CommitmentId::new(),
        asset: usd.clone(),
        amount_minor: 500_000, // $5,000
        settlement_type: SettlementType::DvP,
        counterparty: bob.clone(),
        declaring_identity: alice.clone(),
        created_at: TemporalAnchor::now(0),
    };

    println!("  {} Normal transaction ($5,000): {}", "├".dimmed(),
        match engine.check_compliance(&commitment) {
            Ok(()) => "COMPLIANT".green().bold().to_string(),
            Err(e) => format!("BLOCKED: {}", e).red().to_string(),
        });

    // Large transaction (above AML threshold)
    let large_commitment = FinancialCommitment {
        commitment_id: CommitmentId::new(),
        asset: usd.clone(),
        amount_minor: 30_000_000, // $300,000
        settlement_type: SettlementType::DvP,
        counterparty: bob.clone(),
        declaring_identity: alice.clone(),
        created_at: TemporalAnchor::now(0),
    };

    println!("  {} Large transaction ($300K):   {}", "├".dimmed(),
        match engine.check_compliance(&large_commitment) {
            Ok(()) => "COMPLIANT".green().to_string(),
            Err(e) => format!("BLOCKED — {}", e).red().to_string(),
        });

    // Circuit breaker
    engine.activate_circuit_breaker("extreme market volatility");
    println!("  {} Circuit breaker active:", "├".dimmed());
    println!("  {}   Normal transaction:       {}", "│".dimmed(),
        match engine.check_compliance(&commitment) {
            Ok(()) => "COMPLIANT".green().to_string(),
            Err(e) => format!("BLOCKED — {}", e).red().to_string(),
        });

    engine.deactivate_circuit_breaker();
    println!("  {} Circuit breaker deactivated:", "├".dimmed());
    println!("  {}   Normal transaction:       {}", "└".dimmed(),
        match engine.check_compliance(&commitment) {
            Ok(()) => "COMPLIANT".green().to_string(),
            Err(e) => format!("BLOCKED — {}", e).red().to_string(),
        });

    // ── Summary ─────────────────────────────────────────────────────
    header("Summary");
    println!("  {} EVOS:                {}", "├".dimmed(), "Balance computed by replaying trajectory (never stored)".green());
    println!("  {} DvP Atomicity:       {}", "├".dimmed(), "All legs settle or none — partial = violation".green());
    println!("  {} Regulatory Engine:   {}", "├".dimmed(), "AML, sanctions, capital adequacy, circuit breakers".green());
    println!("  {} Invariants:          {}", "└".dimmed(), "I.ME-FIN-1, I.CEP-FIN-1 demonstrated".green());
    println!();
}
