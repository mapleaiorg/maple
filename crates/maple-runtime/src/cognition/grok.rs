use async_trait::async_trait;

use super::{
    infer_with_parser, synthesize_raw_response, ModelAdapter, ModelAdapterError, ModelBackend,
    ModelProviderConfig, ModelRequest, ModelResponse,
};

/// Grok cognition adapter (proposal-only; execution still gated by MAPLE runtime).
#[derive(Debug, Clone)]
pub struct GrokAdapter {
    config: ModelProviderConfig,
}

impl GrokAdapter {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            config: ModelProviderConfig::grok(model),
        }
    }
}

#[async_trait]
impl ModelAdapter for GrokAdapter {
    fn backend(&self) -> ModelBackend {
        ModelBackend::Grok
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
