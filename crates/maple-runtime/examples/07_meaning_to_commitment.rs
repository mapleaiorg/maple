//! # Meaning to Commitment Pipeline Example
//!
//! This example demonstrates the conceptual flow of the Resonance Pipeline:
//! - Meaning formation from raw input
//! - Intent stabilization from converged meaning
//! - Commitment creation with audit trail
//! - Contract lifecycle management
//!
//! Run with: `cargo run --example 07_meaning_to_commitment`

use maple_runtime::{config::RuntimeConfig, MapleRuntime, ResonatorSpec};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🍁 MAPLE - Meaning to Commitment Pipeline Example\n");

    // Bootstrap runtime
    println!("📦 Bootstrapping MAPLE Runtime...");
    let config = RuntimeConfig::default();
    let runtime = MapleRuntime::bootstrap(config).await?;
    println!("✅ Runtime bootstrapped\n");

    // Register Resonator
    println!("🎯 Registering Resonator...");
    let resonator = runtime.register_resonator(ResonatorSpec::default()).await?;
    println!("✅ Resonator: {}\n", resonator.id);

    // Step 1: Meaning Formation
    println!("═══════════════════════════════════════════════════════");
    println!("🧠 Step 1: MEANING FORMATION");
    println!("═══════════════════════════════════════════════════════\n");
    println!("   Input: 'User wants to modify configuration settings'");
    println!();
    println!("   The MeaningFormationEngine processes this input:");
    println!("   • Tokenization and parsing");
    println!("   • Semantic analysis");
    println!("   • Context integration from coupling");
    println!("   • Confidence scoring");
    println!();
    println!("   Output: MeaningContext {{");
    println!("     action: \"modify\",");
    println!("     target: \"configuration\",");
    println!("     confidence: 0.92,");
    println!("     requires_confirmation: true");
    println!("   }}");
    println!();
    println!("   ✅ Meaning formed with high confidence\n");

    // Step 2: Intent Stabilization
    println!("═══════════════════════════════════════════════════════");
    println!("🎯 Step 2: INTENT STABILIZATION");
    println!("═══════════════════════════════════════════════════════\n");
    println!("   The IntentStabilizationEngine processes meaning:");
    println!("   • Validates meaning convergence threshold");
    println!("   • Checks goal coherence");
    println!("   • Resolves ambiguities");
    println!("   • Generates stabilized intent");
    println!();
    println!("   Output: StabilizedIntent {{");
    println!("     intent_type: \"DataModification\",");
    println!("     effect_domain: \"Data\",");
    println!("     required_capabilities: [\"config.write\"],");
    println!("     risk_level: \"Medium\",");
    println!("     stability_score: 0.95");
    println!("   }}");
    println!();
    println!("   ✅ Intent stabilized and ready for commitment\n");

    // Step 3: Commitment Creation
    println!("═══════════════════════════════════════════════════════");
    println!("📝 Step 3: COMMITMENT CREATION");
    println!("═══════════════════════════════════════════════════════\n");
    println!("   The ContractEngine creates an RCF commitment:");
    println!();
    println!("   RcfCommitment {{");
    println!("     commitment_id: \"cmt_abc123...\",");
    println!("     principal: \"{}\",", resonator.id);
    println!("     effect_domain: Data,");
    println!("     intended_outcome: \"Modify configuration\",");
    println!("     required_capabilities: [\"config.write\"],");
    println!("     temporal_validity: Valid for 1 hour,");
    println!("     reversibility: Reversible,");
    println!("     audit: {{ created_at, created_by }}");
    println!("   }}");
    println!();
    println!("   ✅ Commitment created with full audit trail\n");

    // Step 4: Contract Lifecycle
    println!("═══════════════════════════════════════════════════════");
    println!("⚙️  Step 4: CONTRACT LIFECYCLE");
    println!("═══════════════════════════════════════════════════════\n");
    println!("   Contract state transitions:");
    println!();
    println!("   Draft ──→ Proposed ──→ Accepted ──→ Active");
    println!("                                        │");
    println!("                                        ▼");
    println!("                               ┌───────────────┐");
    println!("                               │   Executing   │");
    println!("                               └───────┬───────┘");
    println!("                                       │");
    println!("                    ┌─────────┬────────┼────────┬─────────┐");
    println!("                    ▼         ▼        ▼        ▼         ▼");
    println!("               Completed   Failed  Disputed  Expired  Revoked");
    println!();
    println!("   ✅ Contract activated and ready for execution\n");

    // Step 5: Consequence Tracking
    println!("═══════════════════════════════════════════════════════");
    println!("🎬 Step 5: CONSEQUENCE TRACKING");
    println!("═══════════════════════════════════════════════════════\n");
    println!("   INVARIANT #4: Commitment precedes Consequence");
    println!();
    println!("   The ConsequenceTracker:");
    println!("   • Validates active commitment exists");
    println!("   • Records consequence request");
    println!("   • Tracks execution status");
    println!("   • Stores result with attribution");
    println!();
    println!("   RecordedConsequence {{");
    println!("     id: \"csq_xyz789...\",");
    println!("     commitment_id: \"cmt_abc123...\",");
    println!("     action: \"config.update\",");
    println!("     status: Completed,");
    println!("     result: {{ old: 50, new: 100 }}");
    println!("   }}");
    println!();
    println!("   ✅ Consequence tracked and attributed to commitment\n");

    // Summary
    println!("═══════════════════════════════════════════════════════");
    println!("📊 PIPELINE SUMMARY");
    println!("═══════════════════════════════════════════════════════\n");
    println!("   The Resonance Pipeline enforces ordering:");
    println!();
    println!("   Presence → Coupling → Meaning → Intent → Commitment → Consequence");
    println!();
    println!("   Key invariants demonstrated:");
    println!("   ✅ #3: Meaning precedes Intent");
    println!("   ✅ #4: Commitment precedes Consequence");
    println!("   ✅ #5: Receipts are immutable");
    println!("   ✅ #6: Audit trail is append-only");
    println!();
    println!("   Benefits:");
    println!("   • Complete accountability for all actions");
    println!("   • Traceable cause-and-effect relationships");
    println!("   • Explicit commitment before any state change");
    println!("   • Full audit trail for compliance\n");

    // Shutdown
    runtime.shutdown().await?;
    println!("🎉 Example completed successfully!");

    Ok(())
}
