//! # Resonator Coupling Example
//!
//! This example demonstrates MAPLE's core innovation: Resonance-based coupling.
//!
//! Key concepts:
//! - Establishing coupling relationships between Resonators
//! - Gradual coupling strengthening (architectural invariant)
//! - Attention-bounded coupling
//! - Safe decoupling
//!
//! Run with: `cargo run --example 02_resonator_coupling`

use maple_runtime::{
    config::RuntimeConfig, CouplingParams, CouplingPersistence, CouplingScope, MapleRuntime,
    ResonatorSpec, SymmetryType,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🍁 MAPLE Runtime - Resonator Coupling Example\n");

    // Bootstrap runtime
    println!("📦 Bootstrapping runtime...");
    let config = RuntimeConfig::default();
    let runtime = MapleRuntime::bootstrap(config).await?;

    // Register two Resonators
    println!("\n🎯 Registering Resonators...");
    let spec_a = ResonatorSpec::default();
    let resonator_a = runtime.register_resonator(spec_a).await?;
    println!("✅ Resonator A: {}", resonator_a.id);

    let spec_b = ResonatorSpec::default();
    let resonator_b = runtime.register_resonator(spec_b).await?;
    println!("✅ Resonator B: {}", resonator_b.id);

    // Establish coupling between A and B
    println!("\n🔗 Establishing coupling A -> B...");
    let coupling_params = CouplingParams {
        source: resonator_a.id,
        target: resonator_b.id,
        initial_strength: 0.2, // MUST be <= 0.3 (gradual strengthening rule)
        initial_attention_cost: 100,
        persistence: CouplingPersistence::Session,
        scope: CouplingScope::Full,
        symmetry: SymmetryType::Symmetric,
    };

    let coupling = resonator_a
        .couple_with(resonator_b.id, coupling_params)
        .await?;
    println!("✅ Coupling established: {}", coupling.id);

    // Check coupling state
    if let Some(state) = coupling.get_coupling() {
        println!("\n📊 Coupling State:");
        println!("   • Strength: {:.2}", state.strength);
        println!("   • Attention Allocated: {}", state.attention_allocated);
        println!("   • Meaning Convergence: {:.2}", state.meaning_convergence);
        println!("   • Interaction Count: {}", state.interaction_count);
        println!("   • Persistence: {:?}", state.persistence);
        println!("   • Scope: {:?}", state.scope);
    }

    // Demonstrate gradual strengthening
    println!("\n⬆️  Strengthening coupling gradually...");
    coupling.strengthen(0.1).await?;
    println!("✅ Coupling strengthened by 0.1");

    coupling.strengthen(0.1).await?;
    println!("✅ Coupling strengthened by 0.1");

    if let Some(state) = coupling.get_coupling() {
        println!("   • New Strength: {:.2}", state.strength);
    }

    // Demonstrate weakening
    println!("\n⬇️  Weakening coupling...");
    coupling.weaken(0.2).await?;
    println!("✅ Coupling weakened by 20%");

    if let Some(state) = coupling.get_coupling() {
        println!("   • New Strength: {:.2}", state.strength);
    }

    // Check attention impact
    if let Some(budget) = resonator_a.attention_status().await {
        println!("\n⚡ Resonator A Attention After Coupling:");
        println!("   • Total Capacity: {}", budget.total_capacity);
        println!("   • Currently Used: {}", budget.used());
        println!("   • Available: {}", budget.available());
        println!("   • Utilization: {:.1}%", budget.utilization() * 100.0);
    }

    // Safe decoupling
    println!("\n🔓 Decoupling safely...");
    let result = coupling.decouple().await?;
    println!("✅ Decoupling result: {:?}", result);

    // Verify attention released
    if let Some(budget) = resonator_a.attention_status().await {
        println!("\n⚡ Resonator A Attention After Decoupling:");
        println!("   • Available: {}", budget.available());
        println!("   • Utilization: {:.1}%", budget.utilization() * 100.0);
    }

    // Shutdown
    runtime.shutdown().await?;
    println!("\n🎉 Example completed successfully!");

    Ok(())
}
