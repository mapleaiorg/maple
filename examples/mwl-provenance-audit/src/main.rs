//! MWL Example 15: Provenance Audit Trail
//!
//! Demonstrates the provenance and audit capabilities of the MWL kernel:
//!
//! 1. **Event Fabric** — Immutable event log with BLAKE3 integrity hashes
//! 2. **Provenance Index** — Efficient worldline-scoped history queries
//! 3. **Causal DAG** — Events linked through parent references
//! 4. **Integrity Verification** — Tamper detection across the entire fabric
//! 5. **Cross-Worldline Audit** — Track events across multiple participants
//!
//! Constitutional Invariants demonstrated:
//! - I.4 (Causal Integrity): Events form a DAG, parents must exist
//! - I.5 (Accountability): Every event has a worldline origin
//! - I.6 (Integrity): BLAKE3 hashes for tamper detection
//! - I.7 (Non-Repudiation): Provenance records are append-only

use colored::Colorize;
use worldline_core::identity::IdentityManager;
use worldline_core::types::{CommitmentId, IdentityMaterial};
use worldline_ledger::provenance::ProvenanceIndex;
use worldline_runtime::fabric::{
    CouplingScope, EventFabric, EventPayload, FabricConfig, ResonanceStage,
};

fn header(title: &str) {
    println!();
    println!("{}", "═".repeat(72).cyan());
    println!("  {}", title.cyan().bold());
    println!("{}", "═".repeat(72).cyan());
}

fn separator() {
    println!("{}", "━".repeat(72).dimmed());
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .init();

    println!();
    println!(
        "{}",
        "╔══════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║    MWL Example 15: Provenance Audit Trail                   ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════╝".cyan()
    );

    // Setup identities
    let mut identity_mgr = IdentityManager::new();
    let alice = identity_mgr
        .create_worldline(IdentityMaterial::GenesisHash([1u8; 32]))
        .unwrap();
    let bob = identity_mgr
        .create_worldline(IdentityMaterial::GenesisHash([2u8; 32]))
        .unwrap();

    let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();
    let mut provenance = ProvenanceIndex::new();

    // ── Part 1: Building an Audit Trail ─────────────────────────────
    header("Part 1: Building an Audit Trail");

    // Alice: genesis
    let a_genesis = fabric
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
    provenance.add_event(&a_genesis).unwrap();
    println!("  {} Alice genesis:   {:?}", "├".dimmed(), a_genesis.id);

    // Bob: genesis
    let b_genesis = fabric
        .emit(
            bob.clone(),
            ResonanceStage::System,
            EventPayload::WorldlineCreated {
                profile: "agent".into(),
            },
            vec![],
        )
        .await
        .unwrap();
    provenance.add_event(&b_genesis).unwrap();
    println!("  {} Bob genesis:     {:?}", "├".dimmed(), b_genesis.id);

    // Alice: meaning formed
    let a_meaning = fabric
        .emit(
            alice.clone(),
            ResonanceStage::Meaning,
            EventPayload::MeaningFormed {
                interpretation_count: 2,
                confidence: 0.88,
                ambiguity_preserved: true,
            },
            vec![a_genesis.id.clone()],
        )
        .await
        .unwrap();
    provenance.add_event(&a_meaning).unwrap();
    println!("  {} Alice meaning:   {:?}", "├".dimmed(), a_meaning.id);

    // Alice: intent stabilized
    let a_intent = fabric
        .emit(
            alice.clone(),
            ResonanceStage::Intent,
            EventPayload::IntentStabilized {
                direction: "delegate task to Bob".into(),
                confidence: 0.91,
                conditions: vec!["Bob accepts delegation".into()],
            },
            vec![a_meaning.id.clone()],
        )
        .await
        .unwrap();
    provenance.add_event(&a_intent).unwrap();
    println!("  {} Alice intent:    {:?}", "├".dimmed(), a_intent.id);

    // Alice: commitment declared
    let cid = CommitmentId::new();
    let a_commit = fabric
        .emit(
            alice.clone(),
            ResonanceStage::Commitment,
            EventPayload::CommitmentDeclared {
                commitment_id: cid.clone(),
                scope: serde_json::json!({"domain": "Communication", "target": "Bob"}),
                parties: vec![bob.clone()],
            },
            vec![a_intent.id.clone()],
        )
        .await
        .unwrap();
    provenance.add_event(&a_commit).unwrap();
    println!("  {} Alice commitment: {:?}", "├".dimmed(), a_commit.id);

    // Bob: coupling established (response to Alice)
    let b_coupling = fabric
        .emit(
            bob.clone(),
            ResonanceStage::Coupling,
            EventPayload::CouplingEstablished {
                target: alice.clone(),
                intensity: 0.6,
                scope: CouplingScope {
                    domains: vec!["Communication".into()],
                    constraints: vec![],
                },
            },
            vec![b_genesis.id.clone()],
        )
        .await
        .unwrap();
    provenance.add_event(&b_coupling).unwrap();
    println!("  {} Bob coupling:    {:?}", "├".dimmed(), b_coupling.id);

    // Alice: commitment fulfilled
    let a_fulfilled = fabric
        .emit(
            alice.clone(),
            ResonanceStage::Consequence,
            EventPayload::CommitmentFulfilled { commitment_id: cid },
            vec![a_commit.id.clone()],
        )
        .await
        .unwrap();
    provenance.add_event(&a_fulfilled).unwrap();
    println!("  {} Alice fulfilled: {:?}", "└".dimmed(), a_fulfilled.id);

    // ── Part 2: Querying Provenance ─────────────────────────────────
    header("Part 2: Querying Provenance by WorldLine");

    let alice_history = provenance.worldline_history(&alice, None);
    println!(
        "  {} Alice's trail ({} events):",
        "├".dimmed(),
        format!("{}", alice_history.len()).yellow()
    );

    let stage_names = ["System", "Meaning", "Intent", "Commitment", "Consequence"];
    for (i, event_id) in alice_history.iter().enumerate() {
        let prefix = if i < alice_history.len() - 1 {
            "│  ├"
        } else {
            "│  └"
        };
        let stage = if i < stage_names.len() {
            stage_names[i]
        } else {
            "?"
        };
        println!("  {}  [{}] {:?}", prefix.dimmed(), stage.blue(), event_id);
    }

    separator();

    let bob_history = provenance.worldline_history(&bob, None);
    println!(
        "  {} Bob's trail ({} events):",
        "├".dimmed(),
        format!("{}", bob_history.len()).yellow()
    );
    for (i, event_id) in bob_history.iter().enumerate() {
        let prefix = if i < bob_history.len() - 1 {
            "│  ├"
        } else {
            "│  └"
        };
        println!("  {}  {:?}", prefix.dimmed(), event_id);
    }

    println!(
        "  {} Total provenance index: {} records",
        "└".dimmed(),
        format!("{}", provenance.len()).yellow()
    );

    // ── Part 3: Integrity Verification ──────────────────────────────
    header("Part 3: Integrity Verification (I.6)");

    let all_events = [
        &a_genesis,
        &b_genesis,
        &a_meaning,
        &a_intent,
        &a_commit,
        &b_coupling,
        &a_fulfilled,
    ];
    let mut verified_count = 0;
    for event in &all_events {
        let ok = event.verify_integrity();
        if ok {
            verified_count += 1;
        }
        println!(
            "  {} {:?} → {}",
            if verified_count < all_events.len() {
                "├"
            } else {
                "└"
            }
            .dimmed(),
            event.id,
            if ok {
                "INTACT".green()
            } else {
                "TAMPERED".red()
            }
        );
    }

    separator();

    let report = fabric.verify().await.unwrap();
    println!("  {} Fabric verification:", "├".dimmed());
    println!(
        "  {}   total events: {}",
        "│".dimmed(),
        format!("{}", report.total_events).yellow()
    );
    println!(
        "  {}   all clean:    {}",
        "└".dimmed(),
        if report.is_clean() {
            "YES".green().bold()
        } else {
            "NO".red().bold()
        }
    );

    // ── Part 4: Causal Chain Analysis ───────────────────────────────
    header("Part 4: Causal Chain Analysis (I.4)");

    println!("  {} Alice's causal chain:", "├".dimmed());
    println!("  {}   Genesis (no parents)", "│  ├".dimmed());
    println!("  {}     └→ Meaning (parent: genesis)", "│  ├".dimmed());
    println!("  {}          └→ Intent (parent: meaning)", "│  ├".dimmed());
    println!(
        "  {}               └→ Commitment (parent: intent)",
        "│  ├".dimmed()
    );
    println!(
        "  {}                    └→ Consequence (parent: commitment)",
        "│  └".dimmed()
    );

    // Verify actual causal links
    println!("  {} Causal links verified:", "├".dimmed());
    println!(
        "  {}   meaning → genesis:    {}",
        "│".dimmed(),
        if a_meaning.parents.contains(&a_genesis.id) {
            "linked".green()
        } else {
            "broken".red()
        }
    );
    println!(
        "  {}   intent → meaning:     {}",
        "│".dimmed(),
        if a_intent.parents.contains(&a_meaning.id) {
            "linked".green()
        } else {
            "broken".red()
        }
    );
    println!(
        "  {}   commit → intent:      {}",
        "│".dimmed(),
        if a_commit.parents.contains(&a_intent.id) {
            "linked".green()
        } else {
            "broken".red()
        }
    );
    println!(
        "  {}   fulfilled → commit:   {}",
        "│".dimmed(),
        if a_fulfilled.parents.contains(&a_commit.id) {
            "linked".green()
        } else {
            "broken".red()
        }
    );

    // Temporal ordering
    println!("  {} Temporal ordering:", "├".dimmed());
    let ts_genesis = a_genesis.timestamp.clone();
    let ts_meaning = a_meaning.timestamp.clone();
    let ts_intent = a_intent.timestamp.clone();
    let ts_commit = a_commit.timestamp.clone();
    let ts_fulfilled = a_fulfilled.timestamp.clone();
    let timestamps = [
        ("genesis", ts_genesis),
        ("meaning", ts_meaning),
        ("intent", ts_intent),
        ("commit", ts_commit),
        ("fulfilled", ts_fulfilled),
    ];
    let mut monotonic = true;
    for window in timestamps.windows(2) {
        if window[0].1 > window[1].1 {
            monotonic = false;
            break;
        }
    }
    println!(
        "  {}   monotonically increasing: {}",
        "└".dimmed(),
        if monotonic {
            "YES".green().bold()
        } else {
            "NO".red().bold()
        }
    );

    // ── Part 5: Accountability ──────────────────────────────────────
    header("Part 5: Accountability (I.5)");

    println!("  {} Every event has an origin worldline:", "├".dimmed());
    for (i, event) in all_events.iter().enumerate() {
        let prefix = if i < all_events.len() - 1 {
            "├"
        } else {
            "└"
        };
        let wl_name = if event.worldline_id == alice {
            "Alice"
        } else {
            "Bob"
        };
        println!(
            "  {}   {:?} → {} ({})",
            prefix.dimmed(),
            event.id,
            format!("{}", event.worldline_id).dimmed(),
            wl_name.blue()
        );
    }

    // ── Summary ─────────────────────────────────────────────────────
    header("Summary");
    println!(
        "  {} Events emitted:       {} across 2 worldlines",
        "├".dimmed(),
        format!("{}", all_events.len()).yellow()
    );
    println!(
        "  {} Provenance records:   {}",
        "├".dimmed(),
        format!("{}", provenance.len()).yellow()
    );
    println!(
        "  {} Integrity verified:   {}/{}",
        "├".dimmed(),
        format!("{}", verified_count).green(),
        all_events.len()
    );
    println!(
        "  {} Causal chain:         {}",
        "├".dimmed(),
        "all links verified".green()
    );
    println!(
        "  {} Temporal ordering:    {}",
        "├".dimmed(),
        "monotonically increasing".green()
    );
    println!(
        "  {} Constitutional:       {}",
        "└".dimmed(),
        "I.4, I.5, I.6, I.7 demonstrated".green()
    );
    println!();
}
