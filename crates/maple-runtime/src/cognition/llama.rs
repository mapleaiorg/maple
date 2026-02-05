use async_trait::async_trait;

use super::{
    infer_with_parser, synthesize_raw_response, ModelAdapter, ModelAdapterError, ModelBackend,
    ModelProviderConfig, ModelRequest, ModelResponse,
};

/// Llama-first adapter with strict parse->repair->fallback handling.
#[derive(Debug, Clone)]
pub struct LlamaAdapter {
    config: ModelProviderConfig,
}

impl LlamaAdapter {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            config: ModelProviderConfig::llama(model),
        }
    }
}

#[async_trait]
impl ModelAdapter for LlamaAdapter {
    fn backend(&self) -> ModelBackend {
        ModelBackend::LocalLlama
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
