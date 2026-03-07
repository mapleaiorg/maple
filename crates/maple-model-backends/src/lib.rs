//! MAPLE Model Backends — adapter implementations for model inference providers.
//!
//! Provides a unified inference interface with concrete backend implementations
//! for Ollama, OpenAI, Anthropic, and other providers.

pub mod backends;
pub mod inference;

pub use inference::{
    ChatMessage, FunctionCall, InferenceRequest, InferenceResponse, MessageRole,
    TokenUsage, ToolCall, ToolDefinition,
};
