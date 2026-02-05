use async_trait::async_trait;

use super::{
    infer_with_parser, synthesize_raw_response, ModelAdapter, ModelAdapterError, ModelBackend,
    ModelProviderConfig, ModelRequest, ModelResponse,
};

/// Anthropic cognition adapter (proposal-only; execution still gated by MAPLE runtime).
#[derive(Debug, Clone)]
pub struct AnthropicAdapter {
    config: ModelProviderConfig,
}

impl AnthropicAdapter {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            config: ModelProviderConfig::anthropic(model),
        }
    }
}

#[async_trait]
impl ModelAdapter for AnthropicAdapter {
    fn backend(&self) -> ModelBackend {
        ModelBackend::Anthropic
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
