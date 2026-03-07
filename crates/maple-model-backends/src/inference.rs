//! Unified inference trait and types for MAPLE model backends.
//!
//! All LLM backends implement the [`ModelBackend`] trait, providing a
//! consistent interface for chat inference, streaming, embeddings, and
//! health checking regardless of the underlying runtime.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message sender.
    pub role: MessageRole,
    /// The text content of the message.
    pub content: String,
    /// Tool calls (for tool-calling capable models).
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool call result ID (for tool responses).
    pub tool_call_id: Option<String>,
}

/// The role of a chat message sender.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System instruction message.
    System,
    /// User (human) message.
    User,
    /// Assistant (model) response.
    Assistant,
    /// Tool/function response.
    Tool,
}

/// A tool call issued by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call.
    pub id: String,
    /// The function to be called.
    pub function: FunctionCall,
}

/// A function call within a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Name of the function to call.
    pub name: String,
    /// Arguments as a JSON string.
    pub arguments: String,
}

/// Tool definition for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Name of the tool.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the tool parameters.
    pub parameters: serde_json::Value,
}

/// Request for inference.
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    /// The conversation messages.
    pub messages: Vec<ChatMessage>,
    /// Model identifier to use.
    pub model: String,
    /// Sampling temperature (0.0 to 2.0).
    pub temperature: Option<f32>,
    /// Top-p (nucleus) sampling threshold.
    pub top_p: Option<f32>,
    /// Maximum number of tokens to generate.
    pub max_tokens: Option<u32>,
    /// Stop sequences.
    pub stop: Vec<String>,
    /// Tool definitions for function calling.
    pub tools: Vec<ToolDefinition>,
    /// Whether to request JSON output mode.
    pub json_mode: bool,
    /// Whether to use streaming.
    pub stream: bool,
}

impl Default for InferenceRequest {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            model: String::new(),
            temperature: None,
            top_p: None,
            max_tokens: None,
            stop: Vec::new(),
            tools: Vec::new(),
            json_mode: false,
            stream: false,
        }
    }
}

/// Response from inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    /// The assistant message response.
    pub message: ChatMessage,
    /// Token usage statistics.
    pub usage: TokenUsage,
    /// Model used for this response.
    pub model: String,
    /// Reason the generation finished.
    pub finish_reason: FinishReason,
    /// Backend-specific metadata.
    pub backend_metadata: serde_json::Value,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenUsage {
    /// Number of tokens in the prompt.
    pub prompt_tokens: u32,
    /// Number of tokens generated.
    pub completion_tokens: u32,
    /// Total tokens (prompt + completion).
    pub total_tokens: u32,
}

/// Reason why inference finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Normal stop (end of generation).
    Stop,
    /// Hit the maximum token limit.
    Length,
    /// Model wants to call tools.
    ToolCalls,
    /// Content was filtered.
    ContentFilter,
    /// An error occurred.
    Error,
}

/// A streaming token event.
#[derive(Debug, Clone)]
pub struct StreamEvent {
    /// The new token(s) produced.
    pub delta: String,
    /// Set when the stream ends.
    pub finish_reason: Option<FinishReason>,
    /// Final usage stats (only on last event).
    pub usage: Option<TokenUsage>,
}

/// The unified backend trait. All LLM backends implement this.
#[async_trait]
pub trait ModelBackend: Send + Sync {
    /// Backend identifier (e.g., "ollama", "openai", "anthropic").
    fn backend_id(&self) -> &str;

    /// Check if backend is available and healthy.
    async fn health_check(&self) -> Result<BackendHealth, BackendError>;

    /// List models available in this backend.
    async fn list_models(&self) -> Result<Vec<String>, BackendError>;

    /// Run chat inference (non-streaming).
    async fn chat(
        &self,
        request: &InferenceRequest,
    ) -> Result<InferenceResponse, BackendError>;

    /// Run chat inference (streaming).
    async fn chat_stream(
        &self,
        request: &InferenceRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<StreamEvent, BackendError>>, BackendError>;

    /// Get embeddings for text.
    async fn embed(
        &self,
        text: &[String],
        model: &str,
    ) -> Result<Vec<Vec<f32>>, BackendError>;
}

/// Backend health information.
#[derive(Debug, Clone)]
pub struct BackendHealth {
    /// Whether the backend is available.
    pub available: bool,
    /// Round-trip latency in milliseconds.
    pub latency_ms: Option<u64>,
    /// Models currently loaded in memory.
    pub loaded_models: Vec<String>,
    /// GPU utilization percentage (0.0 to 1.0).
    pub gpu_utilization: Option<f32>,
    /// Memory used in bytes.
    pub memory_used_bytes: Option<u64>,
}

/// Errors that can occur during backend operations.
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    /// The backend is not available or not responding.
    #[error("Backend unavailable: {0}")]
    Unavailable(String),

    /// The requested model was not found.
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Inference failed for the given reason.
    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    /// Rate limited by the backend.
    #[error("Rate limited: retry after {retry_after_ms}ms")]
    RateLimited {
        /// Milliseconds to wait before retrying.
        retry_after_ms: u64,
    },

    /// The request exceeded the model's context length.
    #[error("Context length exceeded: {0} tokens")]
    ContextExceeded(u32),

    /// HTTP-level error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Error during streaming.
    #[error("Streaming error: {0}")]
    StreamError(String),

    /// The response could not be parsed.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}
