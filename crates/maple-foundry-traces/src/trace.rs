//! Core trace types for capturing agent interaction data.
//!
//! These types represent a complete interaction trace -- one full agent "turn"
//! capturing the entire cycle from input -> reasoning -> action -> outcome.

use serde::{Deserialize, Serialize};

/// A complete interaction trace -- one full agent "turn"
/// capturing the entire cycle from input -> reasoning -> action -> outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionTrace {
    /// Unique trace ID
    pub trace_id: String,
    /// WorldLine ID of the agent
    pub worldline_id: String,
    /// Agent package reference
    pub agent_package: Option<String>,
    /// Model used for this interaction
    pub model: String,
    /// Backend that served the model
    pub backend: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// The input messages (system prompt + user messages)
    pub input: TraceInput,
    /// The model's response
    pub output: TraceOutput,
    /// Tool calls made during this interaction
    pub tool_calls: Vec<TraceToolCall>,
    /// Final outcome (success, failure, partial)
    pub outcome: TraceOutcome,
    /// Performance metrics
    pub metrics: TraceMetrics,
    /// Provenance: WorldLine event IDs for full audit chain
    pub provenance: Vec<String>,
}

/// Input data for a traced interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceInput {
    /// System prompt used (if any)
    pub system_prompt: Option<String>,
    /// Ordered list of messages in the conversation
    pub messages: Vec<TraceMessage>,
    /// Temperature used
    pub temperature: Option<f32>,
    /// Max tokens configured
    pub max_tokens: Option<u32>,
    /// Tools available to the agent
    pub available_tools: Vec<String>,
}

/// A single message in a traced conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceMessage {
    /// Role: "user", "assistant", "system", etc.
    pub role: String,
    /// Content of the message
    pub content: String,
    /// When this message was sent
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// The model's output for a traced interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceOutput {
    /// Textual content of the response
    pub content: String,
    /// Tool calls included in the response
    pub tool_calls: Vec<TraceToolCallResult>,
    /// Reason the model stopped generating
    pub finish_reason: String,
    /// Token usage statistics
    pub token_usage: TokenUsageTrace,
}

/// A tool call made during an interaction, with its result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceToolCall {
    /// Name of the tool invoked
    pub tool_name: String,
    /// Arguments passed to the tool
    pub arguments: serde_json::Value,
    /// Result returned by the tool
    pub result: serde_json::Value,
    /// Whether the tool call succeeded
    pub success: bool,
    /// Latency of the tool call in milliseconds
    pub latency_ms: u64,
    /// Guard decision for this tool call
    pub guard_decision: String,
}

/// A tool call result as part of the model output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceToolCallResult {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the function called
    pub function_name: String,
    /// Serialized arguments
    pub arguments: String,
}

/// Token usage statistics for a traced interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageTrace {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

/// Outcome of the interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceOutcome {
    /// Did the agent accomplish its task?
    pub success: bool,
    /// Human rating (if available): 1-5
    pub human_rating: Option<f32>,
    /// Automated quality score (if computed)
    pub auto_score: Option<f64>,
    /// Labels for the outcome
    pub labels: Vec<String>,
    /// Error details (if failed)
    pub error: Option<String>,
    /// User feedback (if provided)
    pub feedback: Option<String>,
}

/// Performance metrics for a traced interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceMetrics {
    /// Total end-to-end latency in milliseconds
    pub total_latency_ms: u64,
    /// Model inference latency in milliseconds
    pub model_latency_ms: u64,
    /// Cumulative tool call latency in milliseconds
    pub tool_latency_ms: u64,
    /// Estimated cost in USD (if available)
    pub cost_usd: Option<f64>,
}

/// Convenience builder for creating test/example traces.
impl InteractionTrace {
    /// Create a minimal trace for testing purposes.
    #[cfg(test)]
    pub fn test_trace(trace_id: &str, model: &str, success: bool) -> Self {
        Self {
            trace_id: trace_id.to_string(),
            worldline_id: "wl-test-001".to_string(),
            agent_package: Some("test-agent@1.0.0".to_string()),
            model: model.to_string(),
            backend: "test-backend".to_string(),
            timestamp: chrono::Utc::now(),
            input: TraceInput {
                system_prompt: Some("You are a helpful assistant.".to_string()),
                messages: vec![TraceMessage {
                    role: "user".to_string(),
                    content: "Hello, can you help me?".to_string(),
                    timestamp: Some(chrono::Utc::now()),
                }],
                temperature: Some(0.7),
                max_tokens: Some(1024),
                available_tools: vec!["search".to_string()],
            },
            output: TraceOutput {
                content: "Sure, I'd be happy to help!".to_string(),
                tool_calls: vec![],
                finish_reason: "stop".to_string(),
                token_usage: TokenUsageTrace {
                    prompt_tokens: 50,
                    completion_tokens: 20,
                    total_tokens: 70,
                },
            },
            tool_calls: vec![],
            outcome: TraceOutcome {
                success,
                human_rating: None,
                auto_score: if success { Some(0.9) } else { Some(0.3) },
                labels: vec!["general".to_string()],
                error: if success {
                    None
                } else {
                    Some("Task failed".to_string())
                },
                feedback: None,
            },
            metrics: TraceMetrics {
                total_latency_ms: 250,
                model_latency_ms: 200,
                tool_latency_ms: 0,
                cost_usd: Some(0.002),
            },
            provenance: vec!["evt-001".to_string()],
        }
    }

    /// Create a test trace that includes tool calls.
    #[cfg(test)]
    pub fn test_trace_with_tools(trace_id: &str, model: &str) -> Self {
        let mut trace = Self::test_trace(trace_id, model, true);
        trace.tool_calls = vec![TraceToolCall {
            tool_name: "search".to_string(),
            arguments: serde_json::json!({"query": "rust programming"}),
            result: serde_json::json!({"results": ["result1", "result2"]}),
            success: true,
            latency_ms: 45,
            guard_decision: "allow".to_string(),
        }];
        trace.metrics.tool_latency_ms = 45;
        trace
    }
}
