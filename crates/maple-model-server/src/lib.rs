//! MAPLE Model Server -- OpenAI-compatible serving API types and handler.
//!
//! Provides request/response types compatible with the OpenAI chat completions API,
//! model endpoint configuration, server configuration, and request handling.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("model not found: {0}")]
    ModelNotFound(String),
    #[error("request error: {0}")]
    RequestError(String),
    #[error("authentication error: {0}")]
    AuthError(String),
    #[error("rate limit exceeded")]
    RateLimitExceeded,
    #[error("server error: {0}")]
    Internal(String),
}

pub type ServerResult<T> = Result<T, ServerError>;

// ---------------------------------------------------------------------------
// OpenAI-compatible types
// ---------------------------------------------------------------------------

/// A single message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".into(), content: content.into(), name: None }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: content.into(), name: None }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: "assistant".into(), content: content.into(), name: None }
    }
}

/// Chat completion request (OpenAI-compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// A single choice in a chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

/// Chat completion response (OpenAI-compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Usage,
}

impl ChatCompletionResponse {
    /// Create a simple response with a single assistant message.
    pub fn simple(model: &str, content: &str, usage: Usage) -> Self {
        Self {
            id: format!("chatcmpl-{}", Utc::now().timestamp_millis()),
            object: "chat.completion".to_string(),
            created: Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage::assistant(content),
                finish_reason: "stop".to_string(),
            }],
            usage,
        }
    }
}

// ---------------------------------------------------------------------------
// Server configuration
// ---------------------------------------------------------------------------

/// Backend type for a model endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelBackend {
    OpenAI,
    Anthropic,
    Local,
    Custom(String),
}

/// Configuration for a model endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEndpoint {
    pub model_name: String,
    pub backend: ModelBackend,
    pub max_tokens: u32,
    pub context_window: u32,
    pub rate_limit_rpm: Option<u32>,
}

/// Authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub api_keys: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_keys: Vec::new(),
        }
    }
}

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub models: Vec<ModelEndpoint>,
    pub auth: AuthConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            models: Vec::new(),
            auth: AuthConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request Handler
// ---------------------------------------------------------------------------

/// Handles incoming inference requests.
pub struct RequestHandler {
    config: ServerConfig,
    request_count: u64,
}

impl RequestHandler {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            request_count: 0,
        }
    }

    /// Validate an incoming request.
    pub fn validate(&self, request: &ChatCompletionRequest) -> ServerResult<()> {
        if request.messages.is_empty() {
            return Err(ServerError::RequestError("messages cannot be empty".into()));
        }
        // Check model exists
        if !self.config.models.iter().any(|m| m.model_name == request.model) {
            return Err(ServerError::ModelNotFound(request.model.clone()));
        }
        Ok(())
    }

    /// Authenticate a request by API key.
    pub fn authenticate(&self, api_key: Option<&str>) -> ServerResult<()> {
        if !self.config.auth.enabled {
            return Ok(());
        }
        match api_key {
            Some(key) if self.config.auth.api_keys.contains(&key.to_string()) => Ok(()),
            Some(_) => Err(ServerError::AuthError("invalid API key".into())),
            None => Err(ServerError::AuthError("API key required".into())),
        }
    }

    /// Handle a chat completion request (stub that returns a mock response).
    pub fn handle(&mut self, request: &ChatCompletionRequest) -> ServerResult<ChatCompletionResponse> {
        self.validate(request)?;
        self.request_count += 1;

        let prompt_tokens = request
            .messages
            .iter()
            .map(|m| m.content.split_whitespace().count() as u32)
            .sum();

        let response_content = format!("Mock response for model {}", request.model);
        let completion_tokens = response_content.split_whitespace().count() as u32;

        Ok(ChatCompletionResponse::simple(
            &request.model,
            &response_content,
            Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
        ))
    }

    /// Get the number of requests handled.
    pub fn request_count(&self) -> u64 {
        self.request_count
    }

    /// List available models.
    pub fn list_models(&self) -> &[ModelEndpoint] {
        &self.config.models
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> ServerConfig {
        ServerConfig {
            host: "localhost".into(),
            port: 8080,
            models: vec![
                ModelEndpoint {
                    model_name: "gpt-4".into(),
                    backend: ModelBackend::OpenAI,
                    max_tokens: 4096,
                    context_window: 8192,
                    rate_limit_rpm: Some(60),
                },
                ModelEndpoint {
                    model_name: "local-llama".into(),
                    backend: ModelBackend::Local,
                    max_tokens: 2048,
                    context_window: 4096,
                    rate_limit_rpm: None,
                },
            ],
            auth: AuthConfig::default(),
        }
    }

    fn make_request(model: &str) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![ChatMessage::user("Hello")],
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: None,
            stream: None,
            stop: None,
        }
    }

    #[test]
    fn test_handle_request() {
        let mut handler = RequestHandler::new(make_config());
        let req = make_request("gpt-4");
        let resp = handler.handle(&req).unwrap();
        assert_eq!(resp.model, "gpt-4");
        assert!(!resp.choices.is_empty());
    }

    #[test]
    fn test_model_not_found() {
        let mut handler = RequestHandler::new(make_config());
        let req = make_request("nonexistent");
        assert!(handler.handle(&req).is_err());
    }

    #[test]
    fn test_empty_messages() {
        let handler = RequestHandler::new(make_config());
        let req = ChatCompletionRequest {
            model: "gpt-4".into(),
            messages: vec![],
            max_tokens: None,
            temperature: None,
            top_p: None,
            stream: None,
            stop: None,
        };
        assert!(handler.validate(&req).is_err());
    }

    #[test]
    fn test_auth_disabled() {
        let handler = RequestHandler::new(make_config());
        assert!(handler.authenticate(None).is_ok());
    }

    #[test]
    fn test_auth_enabled() {
        let mut config = make_config();
        config.auth.enabled = true;
        config.auth.api_keys = vec!["sk-test123".into()];
        let handler = RequestHandler::new(config);
        assert!(handler.authenticate(Some("sk-test123")).is_ok());
        assert!(handler.authenticate(Some("wrong")).is_err());
        assert!(handler.authenticate(None).is_err());
    }

    #[test]
    fn test_chat_message_constructors() {
        let sys = ChatMessage::system("You are helpful");
        assert_eq!(sys.role, "system");
        let usr = ChatMessage::user("Hello");
        assert_eq!(usr.role, "user");
        let asst = ChatMessage::assistant("Hi");
        assert_eq!(asst.role, "assistant");
    }

    #[test]
    fn test_request_count() {
        let mut handler = RequestHandler::new(make_config());
        assert_eq!(handler.request_count(), 0);
        handler.handle(&make_request("gpt-4")).unwrap();
        assert_eq!(handler.request_count(), 1);
    }

    #[test]
    fn test_list_models() {
        let handler = RequestHandler::new(make_config());
        assert_eq!(handler.list_models().len(), 2);
    }

    #[test]
    fn test_usage_tokens() {
        let mut handler = RequestHandler::new(make_config());
        let req = make_request("gpt-4");
        let resp = handler.handle(&req).unwrap();
        assert!(resp.usage.total_tokens > 0);
        assert_eq!(resp.usage.total_tokens, resp.usage.prompt_tokens + resp.usage.completion_tokens);
    }

    #[test]
    fn test_response_object_type() {
        let resp = ChatCompletionResponse::simple("test", "hello", Usage::default());
        assert_eq!(resp.object, "chat.completion");
    }
}
