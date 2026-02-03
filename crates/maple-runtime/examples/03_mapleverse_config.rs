//! # Mapleverse Configuration Example
//!
//! Demonstrates MAPLE configured for pure AI-to-AI agent coordination.
//!
//! Mapleverse characteristics:
//! - No human profiles allowed (pure AI)
//! - Strong commitment accountability
//! - Explicit coupling and intent
//! - Optimized for massive scale (100M+ concurrent agents)
//!
//! Run with: `cargo run --example 03_mapleverse_config`

use maple_runtime::{
    config::mapleverse_runtime_config, MapleRuntime, ResonatorProfile, ResonatorSpec,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🍁 MAPLE Runtime - Mapleverse Configuration Example\n");
    println!("🤖 Pure AI Agent Coordination Environment\n");

    // Bootstrap with Mapleverse configuration
    println!("📦 Bootstrapping Mapleverse runtime...");
    let config = mapleverse_runtime_config();
    let runtime = MapleRuntime::bootstrap(config).await?;
    println!("✅ Mapleverse runtime ready\n");

    // Register Coordination Resonators (the default for Mapleverse)
    println!("🎯 Registering Coordination Resonators...");

    let mut spec1 = ResonatorSpec::default();
    spec1.profile = ResonatorProfile::Coordination;
    let agent1 = runtime.register_resonator(spec1).await?;
    println!("✅ Agent 1: {} (Coordination profile)", agent1.id);

    let mut spec2 = ResonatorSpec::default();
    spec2.profile = ResonatorProfile::Coordination;
    let agent2 = runtime.register_resonator(spec2).await?;
    println!("✅ Agent 2: {} (Coordination profile)", agent2.id);

    // Attempting to register a Human profile would fail in Mapleverse
    println!("\n❌ Note: Human profiles are not allowed in Mapleverse");
    println!("   This enforces pure AI-to-AI coordination\n");

    // Demonstrate commitment-based interaction
    println!("📝 In Mapleverse, all agent actions require explicit commitments");
    println!("   • No implicit trust");
    println!("   • Full audit trail");
    println!("   • Attributable consequences\n");

    // Show attention economics
    if let Some(budget) = agent1.attention_status().await {
        println!("⚡ Attention Economics:");
        println!("   • Each agent has finite attention capacity");
        println!("   • Prevents runaway coupling");
        println!("   • Enables graceful degradation");
        println!("   • Agent 1 capacity: {}", budget.total_capacity);
    }

    println!("\n📈 Mapleverse is designed to scale to 100M+ concurrent agents");
    println!("   • Lightweight presence signaling");
    println!("   • Attention-bounded coupling");
    println!("   • Distributed temporal coordination");

    runtime.shutdown().await?;
    println!("\n🎉 Mapleverse example completed!");

    Ok(())
}
