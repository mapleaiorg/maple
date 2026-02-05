use maple_model_openai::{OpenAiModelAdapter, AUTH_ENV_VAR};
use maple_runtime::{CognitionState, JournalSliceItem, MeaningDraft, ModelAdapter};

#[tokio::test]
async fn provider_auth_check_can_skip_when_env_missing() {
    if std::env::var(AUTH_ENV_VAR).is_err() {
        eprintln!(
            "skipping provider auth test because {} is missing",
            AUTH_ENV_VAR
        );
        return;
    }

    let adapter = OpenAiModelAdapter::default_stub();
    let token = adapter
        .auth_token_from_env()
        .expect("token should be available");
    assert!(!token.trim().is_empty());
}

#[tokio::test]
async fn provider_summarize_and_intent_stub_are_callable() {
    let adapter = OpenAiModelAdapter::default_stub();

    let _intent = adapter
        .propose_intent(
            &MeaningDraft {
                summary: "pay invoice".to_string(),
                ambiguity_notes: vec![],
                confidence: 0.8,
            },
            &CognitionState::default(),
        )
        .await
        .expect("intent stub should succeed");

    let summary = adapter
        .summarize(&[JournalSliceItem {
            stage: "meaning".to_string(),
            message: "invoice parsed".to_string(),
            payload: serde_json::json!({}),
        }])
        .await
        .expect("summarize should succeed");

    assert!(!summary.summary.is_empty());
}
