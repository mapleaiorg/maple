//! MWL Example 11: WorldLine Lifecycle
//!
//! Demonstrates the full lifecycle of a WorldLine in the Maple WorldLine Framework:
//!
//! 1. **Identity Derivation** — WorldlineId derived deterministically from IdentityMaterial
//! 2. **Event Fabric** — Resonance events emitted through the 8-stage pipeline
//! 3. **Causal Ordering** — Events form a DAG with parent references
//! 4. **Integrity Verification** — BLAKE3 hashes protect against tampering
//! 5. **Provenance Tracking** — Every event is append-only and auditable
//!
//! Constitutional Invariants demonstrated:
//! - I.1 (Worldline Primacy): Identity derives from material, not session
//! - I.4 (Causal Integrity): Events form a DAG with verified parent references
//! - I.6 (Integrity): Every event carries a BLAKE3 hash
//! - I.7 (Non-Repudiation): Provenance records are append-only

use colored::Colorize;
use maple_kernel_fabric::{EventFabric, EventPayload, FabricConfig, ResonanceStage};
use maple_kernel_provenance::ProvenanceIndex;
use maple_mwl_identity::IdentityManager;
use maple_mwl_types::IdentityMaterial;

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
    println!("{}", "║    MWL Example 11: WorldLine Lifecycle                      ║".cyan().bold());
    println!("{}", "╚══════════════════════════════════════════════════════════════╝".cyan());

    // ── Part 1: Identity Derivation ─────────────────────────────────
    header("Part 1: Deterministic Identity Derivation (I.1)");

    let mut identity_mgr = IdentityManager::new();

    // Same material always produces the same WorldlineId
    let material_alice = IdentityMaterial::GenesisHash([1u8; 32]);
    let material_bob = IdentityMaterial::GenesisHash([2u8; 32]);

    let alice = identity_mgr.create_worldline(material_alice.clone()).unwrap();
    let bob = identity_mgr.create_worldline(material_bob.clone()).unwrap();

    println!("  {} Alice WorldlineId: {}", "├".dimmed(), format!("{}", alice).green());
    println!("  {} Bob   WorldlineId: {}", "├".dimmed(), format!("{}", bob).green());

    // Verify determinism: re-derive from same material
    let alice_again = maple_mwl_types::WorldlineId::derive(&material_alice);
    println!("  {} Re-derived Alice:  {}", "├".dimmed(), format!("{}", alice_again).green());
    println!(
        "  {} Deterministic: {}",
        "└".dimmed(),
        if alice == alice_again {
            "YES — same material always yields same identity".green()
        } else {
            "FAIL".red()
        }
    );

    separator();

    // Verify identity manager knows these worldlines
    println!("  {} Alice verified: {}", "├".dimmed(),
        format!("{}", identity_mgr.verify(&alice, &material_alice)).yellow());
    println!("  {} Bob verified:   {}", "└".dimmed(),
        format!("{}", identity_mgr.verify(&bob, &material_bob)).yellow());

    // ── Part 2: Event Fabric — Resonance Stages ─────────────────────
    header("Part 2: Event Fabric — Resonance Stages (I.4, I.6)");

    let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();

    // Genesis event (no parents)
    let e_genesis = fabric
        .emit(
            alice.clone(),
            ResonanceStage::System,
            EventPayload::WorldlineCreated {
                profile: "human".into(),
            },
            vec![],
        )
        .await
        .unwrap();

    println!("  {} Genesis event:  {}", "├".dimmed(),
        format!("{:?}", e_genesis.id).yellow());
    println!("  {}   stage:        {}", "│".dimmed(), "System".blue());
    println!("  {}   parents:      {}", "│".dimmed(), "[] (genesis)".dimmed());

    // Meaning event (child of genesis)
    let e_meaning = fabric
        .emit(
            alice.clone(),
            ResonanceStage::Meaning,
            EventPayload::MeaningFormed {
                interpretation_count: 3,
                confidence: 0.85,
                ambiguity_preserved: true,
            },
            vec![e_genesis.id.clone()],
        )
        .await
        .unwrap();

    println!("  {} Meaning event:  {}", "├".dimmed(),
        format!("{:?}", e_meaning.id).yellow());
    println!("  {}   stage:        {}", "│".dimmed(), "Meaning".blue());
    println!("  {}   confidence:   {}", "│".dimmed(), "0.85".green());
    println!("  {}   parents:      {}", "│".dimmed(),
        format!("[{:?}]", e_genesis.id).dimmed());

    // Intent event (child of meaning)
    let e_intent = fabric
        .emit(
            alice.clone(),
            ResonanceStage::Intent,
            EventPayload::IntentStabilized {
                direction: "send greeting to Bob".into(),
                confidence: 0.92,
                conditions: vec!["Bob is available".into()],
            },
            vec![e_meaning.id.clone()],
        )
        .await
        .unwrap();

    println!("  {} Intent event:   {}", "├".dimmed(),
        format!("{:?}", e_intent.id).yellow());
    println!("  {}   stage:        {}", "│".dimmed(), "Intent".blue());
    println!("  {}   direction:    {}", "│".dimmed(), "send greeting to Bob".green());
    println!("  {}   confidence:   {}", "│".dimmed(), "0.92".green());

    // Commitment event
    let e_commit = fabric
        .emit(
            alice.clone(),
            ResonanceStage::Commitment,
            EventPayload::CommitmentDeclared {
                commitment_id: maple_mwl_types::CommitmentId::new(),
                scope: serde_json::Value::String("Communication".into()),
                parties: vec![bob.clone()],
            },
            vec![e_intent.id.clone()],
        )
        .await
        .unwrap();

    println!("  {} Commitment:     {}", "├".dimmed(),
        format!("{:?}", e_commit.id).yellow());
    println!("  {}   stage:        {}", "│".dimmed(), "Commitment".blue());

    // Consequence event
    let e_consequence = fabric
        .emit(
            alice.clone(),
            ResonanceStage::Consequence,
            EventPayload::CommitmentFulfilled {
                commitment_id: maple_mwl_types::CommitmentId::new(),
            },
            vec![e_commit.id.clone()],
        )
        .await
        .unwrap();

    println!("  {} Consequence:    {}", "└".dimmed(),
        format!("{:?}", e_consequence.id).yellow());

    // ── Part 3: Integrity Verification ──────────────────────────────
    header("Part 3: Integrity Verification (I.6)");

    let events = [&e_genesis, &e_meaning, &e_intent, &e_commit, &e_consequence];
    for (i, event) in events.iter().enumerate() {
        let ok = event.verify_integrity();
        let prefix = if i < events.len() - 1 { "├" } else { "└" };
        println!(
            "  {} Event {:?} integrity: {}",
            prefix.dimmed(),
            event.id,
            if ok { "VERIFIED".green() } else { "FAILED".red() }
        );
    }

    // Fabric-level verification
    let report = fabric.verify().await.unwrap();
    separator();
    println!("  {} Fabric integrity report:", "┌".dimmed());
    println!("  {}   total events:    {}", "├".dimmed(), format!("{}", report.total_events).yellow());
    println!("  {}   clean:           {}", "└".dimmed(),
        if report.is_clean() { "YES".green() } else { "NO".red() });

    // ── Part 4: Provenance Tracking ─────────────────────────────────
    header("Part 4: Provenance Tracking (I.7)");

    let mut provenance = ProvenanceIndex::new();
    provenance.add_event(&e_genesis).unwrap();
    provenance.add_event(&e_meaning).unwrap();
    provenance.add_event(&e_intent).unwrap();
    provenance.add_event(&e_commit).unwrap();
    provenance.add_event(&e_consequence).unwrap();

    let history = provenance.worldline_history(&alice, None);
    println!("  {} Alice's provenance trail: {} events", "├".dimmed(),
        format!("{}", history.len()).yellow());

    for (i, event_id) in history.iter().enumerate() {
        let prefix = if i < history.len() - 1 { "│  ├" } else { "│  └" };
        println!("  {}  {:?}", prefix.dimmed(), event_id);
    }

    println!("  {} Total provenance records: {}", "└".dimmed(),
        format!("{}", provenance.len()).yellow());

    // ── Part 5: Continuity Context ──────────────────────────────────
    header("Part 5: Continuity Context");

    if let Some(ctx) = identity_mgr.continuity_context(&alice) {
        println!("  {} Alice continuity:", "├".dimmed());
        println!("  {}   worldline_id:   {}", "│".dimmed(), format!("{}", ctx.worldline_id).green());
        println!("  {}   segment_index:  {}", "│".dimmed(), format!("{}", ctx.segment_index).yellow());
        println!("  {}   chain_hash:     {}", "└".dimmed(), format!("{:?}", ctx.chain_hash).dimmed());
    } else {
        println!("  {} No continuity context (identity manager may not track it)", "└".dimmed());
    }

    // ── Summary ─────────────────────────────────────────────────────
    header("Summary");
    println!("  {} WorldLines created:       {}", "├".dimmed(), "2 (Alice, Bob)".green());
    println!("  {} Resonance events emitted:  {}", "├".dimmed(), "5 (System→Meaning→Intent→Commitment→Consequence)".green());
    println!("  {} Integrity verified:        {}", "├".dimmed(), "all events pass BLAKE3 hash check".green());
    println!("  {} Provenance records:        {}", "├".dimmed(), "5 append-only entries".green());
    println!("  {} Constitutional invariants: {}", "└".dimmed(), "I.1, I.4, I.6, I.7 demonstrated".green());
    println!();
}
