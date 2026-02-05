use async_trait::async_trait;

use super::{
    infer_with_parser, synthesize_raw_response, ModelAdapter, ModelAdapterError, ModelBackend,
    ModelProviderConfig, ModelRequest, ModelResponse,
};

/// OpenAI cognition adapter (proposal-only; execution still gated by MAPLE runtime).
#[derive(Debug, Clone)]
pub struct OpenAiAdapter {
    config: ModelProviderConfig,
}

impl OpenAiAdapter {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            config: ModelProviderConfig::open_ai(model),
        }
    }
}

#[async_trait]
impl ModelAdapter for OpenAiAdapter {
    fn backend(&self) -> ModelBackend {
        ModelBackend::OpenAi
    }

    fn config(&self) -> &ModelProviderConfig {
        &self.config
    }

    async fn infer(&self, request: &ModelRequest) -> Result<ModelResponse, ModelAdapterError> {
        infer_with_parser(
            self.backend(),
            self.config(),
            request,
            synthesize_raw_response(request, &self.config.model),
        )
    }
}
