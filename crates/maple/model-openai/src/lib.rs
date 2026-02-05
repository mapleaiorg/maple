//! OpenAI provider adapter for MAPLE cognition.
//!
//! This adapter is cognition-only. It can propose drafts and summaries but cannot
//! execute side effects directly.

use std::sync::Arc;

use async_trait::async_trait;
use maple_runtime::{
    CognitionState, EpisodicSummary, IntentDraft, JournalSliceItem, ModelAdapter,
    ModelAdapterError, ModelBackend, ModelErrorKind, ModelProviderConfig, ModelRequest,
    ModelResponse, ModelUsage, StructuredCognition, ValidationStatus,
};
use serde::{Deserialize, Serialize};

pub const DEFAULT_MODEL: &str = "gpt-4o-mini";
pub const AUTH_ENV_VAR: &str = "OPENAI_API_KEY";

/// Minimal transport request type for OpenAI completions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiRequest {
    pub model: String,
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub response_format: String,
}

/// Minimal transport response type for OpenAI completions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiResponse {
    pub output_text: String,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// Provider transport abstraction.
#[async_trait]
pub trait OpenAiTransport: Send + Sync {
    async fn complete(
        &self,
        request: &OpenAiRequest,
        api_key: &str,
    ) -> Result<OpenAiResponse, ModelAdapterError>;
}

/// Default no-op transport. Real HTTP transport can be plugged in later.
#[derive(Debug, Default)]
pub struct NoopTransport;

#[async_trait]
impl OpenAiTransport for NoopTransport {
    async fn complete(
        &self,
        _request: &OpenAiRequest,
        _api_key: &str,
    ) -> Result<OpenAiResponse, ModelAdapterError> {
        Err(ModelAdapterError::new(
            ModelBackend::OpenAi,
            ModelErrorKind::Transport,
            "OpenAI transport not implemented",
        ))
    }
}

/// Provider adapter implementing the MAPLE `ModelAdapter` trait.
#[derive(Clone)]
pub struct OpenAiModelAdapter {
    config: ModelProviderConfig,
    transport: Arc<dyn OpenAiTransport>,
}

impl std::fmt::Debug for OpenAiModelAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiModelAdapter")
            .field("config", &self.config)
            .finish()
    }
}

impl OpenAiModelAdapter {
    pub fn new(model: impl Into<String>) -> Self {
        Self::with_transport(model, Arc::new(NoopTransport))
    }

    pub fn with_transport(model: impl Into<String>, transport: Arc<dyn OpenAiTransport>) -> Self {
        Self {
            config: ModelProviderConfig::open_ai(model),
            transport,
        }
    }

    pub fn auth_token_from_env(&self) -> Result<String, ModelAdapterError> {
        std::env::var(AUTH_ENV_VAR).map_err(|_| {
            ModelAdapterError::new(
                self.backend(),
                ModelErrorKind::InvalidConfig,
                format!("missing {}", AUTH_ENV_VAR),
            )
        })
    }

    pub fn default_stub() -> Self {
        Self::new(DEFAULT_MODEL)
    }

    fn fallback_response(&self, prompt: &str) -> ModelResponse {
        ModelResponse {
            backend: self.backend(),
            provider: self.config.clone(),
            usage: ModelUsage::default(),
            raw_text: format!("openai:fallback:{}", prompt),
            cognition: StructuredCognition {
                meaning_summary: "OpenAI adapter requires explicit structured transport output"
                    .to_string(),
                intent: "request_clarification".to_string(),
                confidence: 0.0,
                suggested_tool: None,
                validation: ValidationStatus::Fallback,
            },
        }
    }
}

#[async_trait]
impl ModelAdapter for OpenAiModelAdapter {
    fn backend(&self) -> ModelBackend {
        ModelBackend::OpenAi
    }

    fn config(&self) -> &ModelProviderConfig {
        &self.config
    }

    async fn infer(&self, request: &ModelRequest) -> Result<ModelResponse, ModelAdapterError> {
        // Cognition-only: providers can propose, but execution is always gated by AgentKernel.
        if let Some(raw) = &request.raw_response_override {
            return Ok(ModelResponse {
                backend: self.backend(),
                provider: self.config.clone(),
                usage: ModelUsage::default(),
                raw_text: raw.clone(),
                cognition: StructuredCognition {
                    meaning_summary: "OpenAI override response requires higher-level parser"
                        .to_string(),
                    intent: "request_clarification".to_string(),
                    confidence: 0.0,
                    suggested_tool: None,
                    validation: ValidationStatus::Fallback,
                },
            });
        }

        let _ = &self.transport;
        Ok(self.fallback_response(&request.user_prompt))
    }

    async fn propose_meaning(
        &self,
        _input: &maple_runtime::MeaningInput,
        _state: &CognitionState,
    ) -> Result<maple_runtime::MeaningDraft, ModelAdapterError> {
        Err(ModelAdapterError::new(
            self.backend(),
            ModelErrorKind::Transport,
            "propose_meaning is not implemented for this provider adapter yet",
        ))
    }

    async fn propose_intent(
        &self,
        meaning: &maple_runtime::MeaningDraft,
        _state: &CognitionState,
    ) -> Result<IntentDraft, ModelAdapterError> {
        Ok(IntentDraft {
            objective: format!("openai_intent_stub:{}", meaning.summary.trim()),
            steps: vec![
                "collect_clarification".to_string(),
                "prepare_contract_draft".to_string(),
            ],
            confidence: 0.55,
            blocking_ambiguity: false,
        })
    }

    async fn summarize(
        &self,
        journal_slice: &[JournalSliceItem],
    ) -> Result<EpisodicSummary, ModelAdapterError> {
        let summary = if journal_slice.is_empty() {
            "OpenAI summary: no events".to_string()
        } else {
            format!("OpenAI summary: {} events processed", journal_slice.len())
        };

        let key_points = journal_slice
            .iter()
            .take(3)
            .map(|event| format!("[{}] {}", event.stage, event.message))
            .collect::<Vec<_>>();

        Ok(EpisodicSummary {
            summary,
            key_points,
            open_questions: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stable_intent_stub() {
        let adapter = OpenAiModelAdapter::default_stub();
        let intent = adapter
            .propose_intent(
                &maple_runtime::MeaningDraft {
                    summary: "transfer 100".to_string(),
                    ambiguity_notes: vec![],
                    confidence: 0.9,
                },
                &CognitionState::default(),
            )
            .await
            .expect("intent stub should succeed");

        assert!(intent.objective.starts_with("openai_intent_stub:"));
        assert_eq!(intent.steps.len(), 2);
    }

    #[tokio::test]
    async fn summarize_is_available() {
        let adapter = OpenAiModelAdapter::default_stub();
        let summary = adapter
            .summarize(&[JournalSliceItem {
                stage: "intent".to_string(),
                message: "intent stabilized".to_string(),
                payload: serde_json::json!({}),
            }])
            .await
            .expect("summary should succeed");

        assert!(summary.summary.contains("summary"));
        assert_eq!(summary.open_questions.len(), 0);
    }
}
