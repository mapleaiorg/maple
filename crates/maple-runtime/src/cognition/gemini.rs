use async_trait::async_trait;

use super::{
    infer_with_parser, synthesize_raw_response, ModelAdapter, ModelAdapterError, ModelBackend,
    ModelProviderConfig, ModelRequest, ModelResponse,
};

/// Gemini cognition adapter (proposal-only; execution still gated by MAPLE runtime).
#[derive(Debug, Clone)]
pub struct GeminiAdapter {
    config: ModelProviderConfig,
}

impl GeminiAdapter {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            config: ModelProviderConfig::gemini(model),
        }
    }
}

#[async_trait]
impl ModelAdapter for GeminiAdapter {
    fn backend(&self) -> ModelBackend {
        ModelBackend::Gemini
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
