#![cfg(feature = "agent-kernel")]

use async_trait::async_trait;
use maple_runtime::agent_kernel::{CapabilityExecutionError, CapabilityInvocation};
use maple_runtime::{
    config::RuntimeConfig, AgentExecutionProfile, AgentHandleRequest, AgentKernel,
    AgentKernelError, AgentRegistration, CapabilityDescriptor, CapabilityExecution,
    CapabilityExecutionMode, CapabilityExecutor, MapleRuntime, ModelBackend, ResonatorSpec,
};
use rcf_types::EffectDomain;
use serde_json::Value;

#[derive(Debug, Default)]
struct EchoCapabilityExecutor;

#[async_trait]
impl CapabilityExecutor for EchoCapabilityExecutor {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor::safe("echo")
    }

    async fn execute(
        &self,
        invocation: &CapabilityInvocation,
    ) -> Result<CapabilityExecution, CapabilityExecutionError> {
        let message = invocation
            .args
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("(no message)");
        Ok(CapabilityExecution {
            capability_name: "echo".to_string(),
            summary: format!("echo: {}", message),
            payload: serde_json::json!({
                "message": message,
                "commitment_id": invocation.commitment_id.as_ref().map(|id| id.0.clone()),
            }),
        })
    }
}

#[derive(Debug, Default)]
struct TransferFundsExecutor;

#[async_trait]
impl CapabilityExecutor for TransferFundsExecutor {
    fn descriptor(&self) -> CapabilityDescriptor {
        transfer_funds_capability()
    }

    async fn execute(
        &self,
        invocation: &CapabilityInvocation,
    ) -> Result<CapabilityExecution, CapabilityExecutionError> {
        let amount = invocation
            .args
            .get("amount")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let recipient = invocation
            .args
            .get("recipient")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        Ok(CapabilityExecution {
            capability_name: "transfer_funds".to_string(),
            summary: format!("simulated transfer of ${} to {}", amount, recipient),
            payload: serde_json::json!({
                "amount": amount,
                "recipient": recipient,
                "simulated": true,
                "commitment_id": invocation.commitment_id.as_ref().map(|id| id.0.clone()),
            }),
        })
    }
}

fn transfer_funds_capability() -> CapabilityDescriptor {
    CapabilityDescriptor {
        name: "transfer_funds".to_string(),
        domain: EffectDomain::Computation,
        scope: rcf_types::ScopeConstraint::new(
            vec!["wallet:ibank-sim".to_string()],
            vec!["transfer".to_string()],
        ),
        consequential: true,
        execution_mode: CapabilityExecutionMode::Simulation,
    }
}

async fn setup_kernel() -> (AgentKernel, maple_runtime::AgentHost) {
    let runtime = MapleRuntime::bootstrap(RuntimeConfig::default())
        .await
        .expect("runtime must bootstrap");
    let kernel = AgentKernel::new(runtime);
    kernel
        .register_capability_executor(std::sync::Arc::new(EchoCapabilityExecutor))
        .await;
    kernel
        .register_capability_executor(std::sync::Arc::new(TransferFundsExecutor))
        .await;

    let host = kernel
        .register_agent(AgentRegistration {
            resonator_spec: ResonatorSpec::default(),
            profile: AgentExecutionProfile {
                name: "ibank".to_string(),
                min_intent_confidence: 0.65,
                require_commitment_for_consequence: true,
            },
            capabilities: vec![
                CapabilityDescriptor::safe("echo"),
                transfer_funds_capability(),
            ],
        })
        .await
        .expect("agent registration should succeed");

    (kernel, host)
}

#[tokio::test]
async fn transfer_without_contract_is_denied() {
    let (kernel, host) = setup_kernel().await;

    let mut req = AgentHandleRequest::new(
        host.resonator_id,
        ModelBackend::LocalLlama,
        "transfer $100 to Alice",
    );
    req.override_tool = Some("transfer_funds".to_string());
    req.override_args = Some(serde_json::json!({"amount": 100, "recipient": "Alice"}));

    let err = kernel
        .handle(req)
        .await
        .expect_err("transfer should require explicit contract");
    assert!(
        matches!(err, AgentKernelError::ContractMissing { .. }),
        "expected ContractMissing, got {}",
        err
    );
}

#[tokio::test]
async fn transfer_with_contract_records_receipt() {
    let (kernel, host) = setup_kernel().await;

    let commitment = kernel
        .draft_commitment(
            host.resonator_id,
            "transfer_funds",
            "Transfer $100 to Alice under iBank policy",
        )
        .await
        .expect("commitment draft should succeed");

    let mut req = AgentHandleRequest::new(
        host.resonator_id,
        ModelBackend::LocalLlama,
        "transfer $100 to Alice",
    );
    req.override_tool = Some("transfer_funds".to_string());
    req.override_args = Some(serde_json::json!({"amount": 100, "recipient": "Alice"}));
    req.commitment = Some(commitment.clone());

    let response = kernel.handle(req).await.expect("execution should succeed");
    assert!(response.action.is_some(), "expected capability execution");

    let receipts = kernel
        .receipts_for_commitment(&commitment.commitment_id)
        .await
        .expect("receipts should be queryable");
    assert_eq!(receipts.len(), 1, "one receipt should be persisted");
    assert_eq!(receipts[0].capability_id, "transfer_funds");
    assert_eq!(receipts[0].contract_id, commitment.commitment_id);
}
