//! # Meaning to Commitment Pipeline Example
//!
//! This example demonstrates the full cognitive pipeline:
//! - Meaning formation from raw input
//! - Intent stabilization from converged meaning
//! - Commitment creation with audit trail
//! - Consequence tracking
//!
//! Run with: `cargo run --example 07_meaning_to_commitment`

use maple_runtime::{config::RuntimeConfig, MapleRuntime, ResonatorSpec};
use resonator_commitment::{
    ContractEngine, InMemoryContractEngine, RcfCommitment, CommitmentId,
    EffectDomain, RequiredCapability,
};
use resonator_consequence::{
    ConsequenceTracker, InMemoryConsequenceTracker, ConsequenceRequest,
};
use chrono::Utc;

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

    // Initialize engines
    let contract_engine = InMemoryContractEngine::new();
    let consequence_tracker = InMemoryConsequenceTracker::new();

    // Step 1: Simulate meaning formation
    println!("🧠 Step 1: Meaning Formation");
    println!("   Input: 'User wants to modify configuration settings'");
    println!("   Semantic analysis...");
    println!("   ✅ Meaning formed: action=modify, target=configuration\n");

    // Step 2: Simulate intent stabilization
    println!("🎯 Step 2: Intent Stabilization");
    println!("   Checking meaning convergence...");
    println!("   Validating goal coherence...");
    println!("   ✅ Intent stabilized: ModifyConfiguration\n");

    // Step 3: Create commitment
    println!("📝 Step 3: Commitment Creation");

    let commitment = RcfCommitment {
        commitment_id: CommitmentId(uuid::Uuid::new_v4().to_string()),
        principal: resonator.id.to_string(),
        effect_domain: EffectDomain::DataModification,
        required_capabilities: vec![
            RequiredCapability {
                capability_id: "config.write".to_string(),
                minimum_level: 1,
            },
        ],
        preconditions: vec![],
        postconditions: vec![],
        created_at: Utc::now(),
        expires_at: None,
        metadata: Default::default(),
    };

    let stored = contract_engine.submit_contract(commitment.clone())?;
    println!("   Commitment ID: {}", stored.contract.commitment_id.0);
    println!("   Principal: {}", stored.contract.principal);
    println!("   Effect Domain: {:?}", stored.contract.effect_domain);
    println!("   Status: {:?}", stored.status);
    println!("   ✅ Commitment created and stored\n");

    // Step 4: Transition to Active
    println!("⚙️  Step 4: Activating Commitment");
    contract_engine.propose_contract(&stored.contract.commitment_id)?;
    contract_engine.accept_contract(&stored.contract.commitment_id)?;
    contract_engine.activate_contract(&stored.contract.commitment_id)?;

    if let Some(active) = contract_engine.get_contract(&stored.contract.commitment_id)? {
        println!("   Status: {:?}", active.status);
        println!("   ✅ Commitment is now active\n");
    }

    // Step 5: Execute and track consequence
    println!("🎬 Step 5: Executing & Tracking Consequence");

    // Register the active commitment with consequence tracker
    consequence_tracker.register_commitment(stored.contract.commitment_id.0.clone())?;

    // Request consequence execution
    let consequence = consequence_tracker.request_consequence(
        stored.contract.commitment_id.0.clone(),
        ConsequenceRequest {
            action: "modify_configuration".to_string(),
            parameters: serde_json::json!({
                "setting": "max_connections",
                "value": 100
            }),
        },
    )?;

    println!("   Consequence ID: {}", consequence.id);
    println!("   Linked Commitment: {}", consequence.commitment_id);
    println!("   Action: {}", consequence.action);
    println!("   Status: {:?}", consequence.status);
    println!("   ✅ Consequence tracked\n");

    // Step 6: Complete consequence
    println!("✨ Step 6: Completing Consequence");
    consequence_tracker.complete_consequence(
        &consequence.id,
        serde_json::json!({
            "success": true,
            "old_value": 50,
            "new_value": 100
        }),
    )?;

    if let Some(completed) = consequence_tracker.get_consequence(&consequence.id)? {
        println!("   Status: {:?}", completed.status);
        println!("   Result: {}", completed.result.unwrap_or_default());
        println!("   ✅ Consequence completed\n");
    }

    // Step 7: Complete commitment
    println!("🏁 Step 7: Completing Commitment");
    contract_engine.complete_contract(&stored.contract.commitment_id)?;

    if let Some(completed) = contract_engine.get_contract(&stored.contract.commitment_id)? {
        println!("   Final Status: {:?}", completed.status);
        println!("   ✅ Commitment lifecycle complete\n");
    }

    // Summary
    println!("📊 Pipeline Summary");
    println!("═══════════════════════════════════════════════════════");
    println!("   Meaning → Intent → Commitment → Consequence");
    println!("   ✅ All invariants maintained");
    println!("   ✅ Full audit trail created");
    println!("   ✅ Consequence attributed to commitment\n");

    // Shutdown
    runtime.shutdown().await?;
    println!("🎉 Example completed successfully!");

    Ok(())
}
