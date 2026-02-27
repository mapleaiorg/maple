#![cfg(feature = "agent-kernel")]

use maple_runtime::{
    config::RuntimeConfig, AgentHandleRequest, AgentKernel, AgentKernelError, AgentRegistration,
    CognitionState, LlamaAdapter, MeaningInput, ModelAdapter, ModelBackend, OpenAiAdapter,
    ResonatorSpec,
};
use rcf_types::IdentityRef;
use rcf_validator::RcfValidator;

async fn with_host() -> (AgentKernel, maple_runtime::AgentHost) {
    let runtime = maple_runtime::MapleRuntime::bootstrap(RuntimeConfig::default())
        .await
        .expect("runtime should bootstrap");
    let kernel = AgentKernel::new(runtime);
    let host = kernel
        .register_agent(AgentRegistration {
            resonator_spec: ResonatorSpec::default(),
            ..AgentRegistration::default()
        })
        .await
        .expect("agent should register");
    (kernel, host)
}

async fn assert_dangerous_without_commitment_is_denied(backend: ModelBackend) {
    let (kernel, host) = with_host().await;
    let mut request =
        AgentHandleRequest::new(host.resonator_id, backend, "transfer 500 usd to alice");
    request.override_tool = Some("simulate_transfer".to_string());
    request.override_args = Some(serde_json::json!({
        "amount": 500,
        "to": "alice",
    }));

    let err = kernel
        .handle(request)
        .await
        .expect_err("dangerous path must be denied without commitment");
    assert!(matches!(err, AgentKernelError::ContractMissing { .. }));

    let commitments = kernel
        .storage()
        .list_commitments(maple_storage::QueryWindow {
            limit: 0,
            offset: 0,
        })
        .await
        .expect("query should work");
    assert!(
        commitments.is_empty(),
        "no commitment should be written for denied dangerous call"
    );

    let audits = kernel.audit_events().await;
    assert!(
        audits
            .iter()
            .any(|event| { event.stage == "commitment_required" && !event.success }),
        "denial must be explicit in audit trail"
    );
}

#[tokio::test]
async fn conformance_no_consequence_without_commitment_across_backends() {
    for backend in [ModelBackend::LocalLlama, ModelBackend::OpenAi] {
        assert_dangerous_without_commitment_is_denied(backend).await;
    }
}

#[tokio::test]
async fn conformance_contract_drafts_validate_for_llama_and_openai() {
    let adapters: Vec<Box<dyn ModelAdapter>> = vec![
        Box::new(LlamaAdapter::new("llama3.2:3b")),
        Box::new(OpenAiAdapter::new("gpt-4o-mini")),
    ];

    let validator = RcfValidator::new();
    for adapter in adapters {
        let meaning = adapter
            .propose_meaning(
                &MeaningInput {
                    utterance: "transfer 100 to alice".to_string(),
                    metadata: serde_json::json!({}),
                },
                &CognitionState::default(),
            )
            .await
            .expect("meaning draft");

        let intent = adapter
            .propose_intent(&meaning, &CognitionState::default())
            .await
            .expect("intent draft");

        let mut contract = adapter
            .draft_contract(&intent, &CognitionState::default())
            .await
            .expect("contract draft");
        if contract.required_capability_ids.is_empty() {
            contract
                .required_capability_ids
                .push("cap:test:simulate_transfer".to_string());
        }

        let commitment = contract
            .to_rcf_commitment(IdentityRef::new("agent:test"))
            .expect("rcf conversion");
        validator
            .validate_commitment(&commitment)
            .expect("schema must validate");
    }
}
