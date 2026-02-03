//! # iBank Configuration Example
//!
//! Demonstrates MAPLE configured for autonomous AI-only financial operations.
//!
//! iBank characteristics:
//! - AI-only (no human Resonators)
//! - Mandatory audit trails for all commitments
//! - Risk assessments required for financial operations
//! - Strict accountability and attributability
//! - Reversibility preferred where possible
//! - Risk-bounded autonomous decisions
//!
//! Run with: `cargo run --example 05_ibank_config`

use maple_runtime::{config::ibank_runtime_config, MapleRuntime, ResonatorProfile, ResonatorSpec};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🍁 MAPLE Runtime - iBank Configuration Example\n");
    println!("🏦 Autonomous AI-Only Financial System\n");

    // Bootstrap with iBank configuration
    println!("📦 Bootstrapping iBank runtime...");
    let config = ibank_runtime_config();
    let runtime = MapleRuntime::bootstrap(config).await?;
    println!("✅ iBank runtime ready\n");

    // Register IBank Resonators (AI financial agents)
    println!("🤖 Registering iBank Resonators...");

    let mut spec1 = ResonatorSpec::default();
    spec1.profile = ResonatorProfile::IBank;
    let agent1 = runtime.register_resonator(spec1).await?;
    println!("✅ Financial Agent 1: {} (IBank profile)", agent1.id);

    let mut spec2 = ResonatorSpec::default();
    spec2.profile = ResonatorProfile::IBank;
    let agent2 = runtime.register_resonator(spec2).await?;
    println!("✅ Financial Agent 2: {} (IBank profile)", agent2.id);

    // Demonstrate strict accountability
    println!("\n📜 Strict Accountability:");
    println!("   • Every financial commitment has full audit trail");
    println!("   • Digital signatures for non-repudiation");
    println!("   • Immutable ledger of all transactions");
    println!("   • Complete attributability chain\n");

    // Demonstrate risk assessment
    println!("⚠️  Risk Assessment:");
    println!("   • Mandatory for all financial operations");
    println!("   • Risk level scoring (0.0 - 1.0)");
    println!("   • Maximum impact calculation");
    println!("   • Mitigation strategy requirements");
    println!("   • Approval thresholds\n");

    // Show risk-bounded decisions
    println!("💰 Risk-Bounded Autonomous Decisions:");
    println!("   • Maximum consequence value: $1,000,000");
    println!("   • Amounts above require escalation");
    println!("   • Prevents unbounded autonomous spending");
    println!("   • Architectural limit, not policy\n");

    // Demonstrate reversibility preference
    println!("⏪ Reversibility Preference:");
    println!("   • Transactions marked as reversible when possible");
    println!("   • Reversal records maintained");
    println!("   • Settlement delays allow for review");
    println!("   • Balances safety with efficiency\n");

    // Show attention in financial context
    if let Some(budget) = agent1.attention_status().await {
        println!("⚡ Attention in Financial Context:");
        println!("   • Prevents simultaneous high-risk operations");
        println!("   • Bounds complexity of financial decisions");
        println!("   • Natural rate limiting for safety");
        println!("   • Agent 1 capacity: {}", budget.total_capacity);
    }

    // Demonstrate coupling restrictions
    println!("\n🔗 Coupling Restrictions:");
    println!("   • IBank agents can ONLY couple with other IBank agents");
    println!("   • No human profiles allowed in iBank");
    println!("   • This ensures AI-only financial operations");
    println!("   • Prevents accidental human involvement\n");

    // Show commitment requirements
    println!("📋 Commitment Requirements:");
    println!("   • All financial actions require explicit commitments");
    println!("   • Commitments cannot be created without stabilized intent");
    println!("   • Intent cannot be formed without sufficient meaning");
    println!("   • Full chain of reasoning preserved\n");

    println!("💡 Key Insight:");
    println!("   iBank demonstrates that autonomous AI financial systems");
    println!("   can be BOTH powerful AND accountable. Every decision has");
    println!("   a complete audit trail, every risk is assessed, and");
    println!("   architectural limits prevent catastrophic mistakes.\n");

    runtime.shutdown().await?;
    println!("🎉 iBank example completed!");

    Ok(())
}
