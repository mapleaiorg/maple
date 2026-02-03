//! # Finalverse Configuration Example
//!
//! Demonstrates MAPLE configured for meaningful human-AI coexistence.
//!
//! Finalverse characteristics:
//! - Human agency protection (architectural, not policy-based)
//! - Coercion detection
//! - Emotional exploitation prevention
//! - Reversible consequences preferred
//! - Experiential focus
//!
//! Run with: `cargo run --example 04_finalverse_config`

use maple_runtime::{
    config::finalverse_runtime_config, MapleRuntime, ResonatorProfile, ResonatorSpec,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🍁 MAPLE Runtime - Finalverse Configuration Example\n");
    println!("👤 Meaningful Human-AI Coexistence\n");

    // Bootstrap with Finalverse configuration
    println!("📦 Bootstrapping Finalverse runtime...");
    let config = finalverse_runtime_config();
    let runtime = MapleRuntime::bootstrap(config).await?;
    println!("✅ Finalverse runtime ready\n");

    // Register a Human Resonator
    println!("👤 Registering Human Resonator...");
    let mut human_spec = ResonatorSpec::default();
    human_spec.profile = ResonatorProfile::Human;
    let human = runtime.register_resonator(human_spec).await?;
    println!("✅ Human Resonator: {}", human.id);

    // Register AI World Resonators
    println!("\n🌍 Registering World Resonators (AI)...");
    let mut world_spec = ResonatorSpec::default();
    world_spec.profile = ResonatorProfile::World;
    let world_agent = runtime.register_resonator(world_spec).await?;
    println!("✅ World Agent: {}", world_agent.id);

    // Demonstrate Human Agency Protection
    println!("\n🛡️  Human Agency Protection:");
    println!("   • Architectural enforcement (not policy-based)");
    println!("   • Humans can always disengage from any coupling");
    println!("   • No forced actions on Human Resonators");
    println!("   • Explicit consent required for consequential actions\n");

    // Demonstrate Coercion Detection
    println!("🔍 Coercion Detection:");
    println!("   • Monitors for patterns of manipulation");
    println!("   • Detects attention exhaustion attacks");
    println!("   • Prevents gradual erosion of agency");
    println!("   • Architectural safeguards against emotional exploitation\n");

    // Show Reversibility Preference
    println!("⏪ Reversibility Preference:");
    println!("   • Finalverse prefers reversible consequences");
    println!("   • Allows for exploration without permanent harm");
    println!("   • Supports experiential learning");
    println!("   • Maintains meaningful experiences without irreversible damage\n");

    // Demonstrate presence gradient
    if let Some(presence) = human.get_presence() {
        println!("📊 Human Presence (Gradient, not Binary):");
        println!("   • Discoverability: {:.2}", presence.discoverability);
        println!("   • Responsiveness: {:.2}", presence.responsiveness);
        println!(
            "   • Coupling Readiness: {:.2}",
            presence.coupling_readiness
        );
        println!("   • Silent Mode: {}", presence.silent_mode);
        println!("\n   Note: Humans can be present without being willing to interact");
    }

    println!("\n💡 Key Insight:");
    println!("   Finalverse creates meaningful experiences where humans and AI");
    println!("   coexist, with architectural guarantees that human agency");
    println!("   cannot be bypassed, even accidentally.\n");

    runtime.shutdown().await?;
    println!("🎉 Finalverse example completed!");

    Ok(())
}
