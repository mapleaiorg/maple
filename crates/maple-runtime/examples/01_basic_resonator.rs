//! # Basic Resonator Example
//!
//! This example demonstrates the fundamental concepts of MAPLE:
//! - Bootstrapping the runtime
//! - Creating a Resonator with persistent identity
//! - Signaling presence
//! - Graceful shutdown
//!
//! Run with: `cargo run --example 01_basic_resonator`

use maple_runtime::{
    MapleRuntime, ResonatorSpec,
    config::RuntimeConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for observability
    tracing_subscriber::fmt::init();

    println!("🍁 MAPLE Runtime - Basic Resonator Example\n");

    // Step 1: Bootstrap the MAPLE Resonance Runtime
    println!("📦 Bootstrapping MAPLE Runtime...");
    let config = RuntimeConfig::default();
    let runtime = MapleRuntime::bootstrap(config).await?;
    println!("✅ Runtime bootstrapped successfully\n");

    // Step 2: Register a Resonator
    println!("🎯 Registering Resonator...");
    let spec = ResonatorSpec::default();
    let resonator = runtime.register_resonator(spec).await?;
    println!("✅ Resonator registered: {}\n", resonator.id);

    // Step 3: Signal presence
    println!("📡 Signaling presence...");
    tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;

    let presence = maple_runtime::PresenceState::new();
    resonator.signal_presence(presence).await?;
    println!("✅ Presence signaled\n");

    // Step 4: Check presence state
    if let Some(state) = resonator.get_presence() {
        println!("📊 Current Presence State:");
        println!("   • Discoverability: {:.2}", state.discoverability);
        println!("   • Responsiveness: {:.2}", state.responsiveness);
        println!("   • Stability: {:.2}", state.stability);
        println!("   • Coupling Readiness: {:.2}", state.coupling_readiness);
        println!("   • Silent Mode: {}\n", state.silent_mode);
    }

    // Step 5: Check attention budget
    if let Some(budget) = resonator.attention_status().await {
        println!("⚡ Attention Budget:");
        println!("   • Total Capacity: {}", budget.total_capacity);
        println!("   • Currently Used: {}", budget.used());
        println!("   • Available: {}", budget.available());
        println!("   • Utilization: {:.1}%\n", budget.utilization() * 100.0);
    }

    // Step 6: Graceful shutdown
    println!("🛑 Shutting down runtime...");
    runtime.shutdown().await?;
    println!("✅ Shutdown complete\n");

    println!("🎉 Example completed successfully!");

    Ok(())
}
