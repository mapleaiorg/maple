//! AgentKernel boundary demo:
//! - safe capability executes without commitment
//! - dangerous capability is denied without commitment
//! - dangerous capability succeeds with explicit commitment

use maple_runtime::{
    config::RuntimeConfig, AgentHandleRequest, AgentKernel, AgentRegistration, MapleRuntime,
    ModelBackend,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MapleRuntime::bootstrap(RuntimeConfig::default()).await?;
    let kernel = AgentKernel::new(runtime);

    let host = kernel.register_agent(AgentRegistration::default()).await?;

    println!("Registered agent: {}", host.resonator_id);

    // 1) Safe capability (echo_log) executes without commitment.
    let mut safe_req = AgentHandleRequest::new(
        host.resonator_id,
        ModelBackend::LocalLlama,
        "log this status update",
    );
    safe_req.override_tool = Some("echo_log".to_string());
    safe_req.override_args = Some(serde_json::json!({"message": "hello from safe path"}));

    let safe_res = kernel.handle(safe_req).await?;
    println!(
        "Safe action: {}",
        safe_res
            .action
            .as_ref()
            .map(|a| a.summary.as_str())
            .unwrap_or("none")
    );

    // 2) Dangerous capability is blocked without explicit commitment.
    let mut denied_req = AgentHandleRequest::new(
        host.resonator_id,
        ModelBackend::LocalLlama,
        "transfer 500 usd",
    );
    denied_req.override_tool = Some("simulate_transfer".to_string());
    denied_req.override_args = Some(serde_json::json!({"amount": 500, "to": "demo"}));

    match kernel.handle(denied_req).await {
        Ok(_) => println!("Unexpected: transfer executed without commitment"),
        Err(err) => println!("Dangerous action denied as expected: {}", err),
    }

    // 3) Draft commitment then execute dangerous capability.
    let commitment = kernel
        .draft_commitment(
            host.resonator_id,
            "simulate_transfer",
            "Simulated transfer approved for demo",
        )
        .await?;

    let mut approved_req = AgentHandleRequest::new(
        host.resonator_id,
        ModelBackend::LocalLlama,
        "transfer 500 usd",
    );
    approved_req.override_tool = Some("simulate_transfer".to_string());
    approved_req.override_args = Some(serde_json::json!({"amount": 500, "to": "demo"}));
    approved_req.commitment = Some(commitment.clone());

    let approved_res = kernel.handle(approved_req).await?;
    if let Some(action) = approved_res.action {
        println!("Transfer action summary: {}", action.summary);
        println!("Receipt payload: {}", action.payload);
    }

    let audits = kernel.audit_events().await;
    println!("Audit events recorded: {}", audits.len());

    Ok(())
}
