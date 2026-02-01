//! Resonance-specific metrics

use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry};

/// Metrics for Resonance AI interactions
pub struct ResonanceMetrics {
    /// Total AI invocations
    pub ai_invocations_total: IntCounterVec,

    /// AI invocation duration
    pub ai_invocation_duration_seconds: HistogramVec,

    /// Token usage
    pub tokens_used_total: IntCounterVec,

    /// Current token rate
    pub token_rate: IntGaugeVec,

    /// Memory operations
    pub memory_operations_total: IntCounterVec,

    /// Memory size
    pub memory_size_bytes: IntGaugeVec,

    /// Tool calls
    pub tool_calls_total: IntCounterVec,

    /// Tool call duration
    pub tool_call_duration_seconds: HistogramVec,

    /// Context window usage
    pub context_window_usage: IntGaugeVec,

    /// Conversation turns
    pub conversation_turns_total: IntCounterVec,
}

impl ResonanceMetrics {
    /// Create and register resonance metrics
    pub fn new(registry: &Registry) -> Self {
        let ai_invocations_total = IntCounterVec::new(
            Opts::new("resonance_ai_invocations_total", "Total AI model invocations"),
            &["platform", "model", "outcome"],
        )
        .expect("Failed to create ai_invocations_total metric");
        registry
            .register(Box::new(ai_invocations_total.clone()))
            .expect("Failed to register ai_invocations_total");

        let ai_invocation_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "resonance_ai_invocation_duration_seconds",
                "AI invocation duration",
            )
            .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 120.0]),
            &["platform", "model"],
        )
        .expect("Failed to create ai_invocation_duration_seconds metric");
        registry
            .register(Box::new(ai_invocation_duration_seconds.clone()))
            .expect("Failed to register ai_invocation_duration_seconds");

        let tokens_used_total = IntCounterVec::new(
            Opts::new("resonance_tokens_used_total", "Total tokens used"),
            &["platform", "model", "token_type"],
        )
        .expect("Failed to create tokens_used_total metric");
        registry
            .register(Box::new(tokens_used_total.clone()))
            .expect("Failed to register tokens_used_total");

        let token_rate = IntGaugeVec::new(
            Opts::new("resonance_token_rate", "Current token rate (tokens per minute)"),
            &["platform", "model"],
        )
        .expect("Failed to create token_rate metric");
        registry
            .register(Box::new(token_rate.clone()))
            .expect("Failed to register token_rate");

        let memory_operations_total = IntCounterVec::new(
            Opts::new("resonance_memory_operations_total", "Memory system operations"),
            &["platform", "operation", "memory_type"],
        )
        .expect("Failed to create memory_operations_total metric");
        registry
            .register(Box::new(memory_operations_total.clone()))
            .expect("Failed to register memory_operations_total");

        let memory_size_bytes = IntGaugeVec::new(
            Opts::new("resonance_memory_size_bytes", "Memory size in bytes"),
            &["platform", "memory_type", "agent_id"],
        )
        .expect("Failed to create memory_size_bytes metric");
        registry
            .register(Box::new(memory_size_bytes.clone()))
            .expect("Failed to register memory_size_bytes");

        let tool_calls_total = IntCounterVec::new(
            Opts::new("resonance_tool_calls_total", "Total tool calls"),
            &["platform", "tool_name", "outcome"],
        )
        .expect("Failed to create tool_calls_total metric");
        registry
            .register(Box::new(tool_calls_total.clone()))
            .expect("Failed to register tool_calls_total");

        let tool_call_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "resonance_tool_call_duration_seconds",
                "Tool call execution duration",
            )
            .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0, 30.0]),
            &["platform", "tool_name"],
        )
        .expect("Failed to create tool_call_duration_seconds metric");
        registry
            .register(Box::new(tool_call_duration_seconds.clone()))
            .expect("Failed to register tool_call_duration_seconds");

        let context_window_usage = IntGaugeVec::new(
            Opts::new(
                "resonance_context_window_usage",
                "Context window usage (tokens)",
            ),
            &["platform", "model", "agent_id"],
        )
        .expect("Failed to create context_window_usage metric");
        registry
            .register(Box::new(context_window_usage.clone()))
            .expect("Failed to register context_window_usage");

        let conversation_turns_total = IntCounterVec::new(
            Opts::new("resonance_conversation_turns_total", "Total conversation turns"),
            &["platform", "agent_id"],
        )
        .expect("Failed to create conversation_turns_total metric");
        registry
            .register(Box::new(conversation_turns_total.clone()))
            .expect("Failed to register conversation_turns_total");

        Self {
            ai_invocations_total,
            ai_invocation_duration_seconds,
            tokens_used_total,
            token_rate,
            memory_operations_total,
            memory_size_bytes,
            tool_calls_total,
            tool_call_duration_seconds,
            context_window_usage,
            conversation_turns_total,
        }
    }

    /// Record an AI invocation
    pub fn record_ai_invocation(
        &self,
        platform: &str,
        model: &str,
        outcome: &str,
        duration_secs: f64,
    ) {
        self.ai_invocations_total
            .with_label_values(&[platform, model, outcome])
            .inc();
        self.ai_invocation_duration_seconds
            .with_label_values(&[platform, model])
            .observe(duration_secs);
    }

    /// Record token usage
    pub fn record_tokens(&self, platform: &str, model: &str, input_tokens: u64, output_tokens: u64) {
        self.tokens_used_total
            .with_label_values(&[platform, model, "input"])
            .inc_by(input_tokens);
        self.tokens_used_total
            .with_label_values(&[platform, model, "output"])
            .inc_by(output_tokens);
    }

    /// Set current token rate
    pub fn set_token_rate(&self, platform: &str, model: &str, rate: i64) {
        self.token_rate
            .with_label_values(&[platform, model])
            .set(rate);
    }

    /// Record a memory operation
    pub fn record_memory_operation(
        &self,
        platform: &str,
        operation: &str,
        memory_type: &str,
    ) {
        self.memory_operations_total
            .with_label_values(&[platform, operation, memory_type])
            .inc();
    }

    /// Set memory size
    pub fn set_memory_size(&self, platform: &str, memory_type: &str, agent_id: &str, size: i64) {
        self.memory_size_bytes
            .with_label_values(&[platform, memory_type, agent_id])
            .set(size);
    }

    /// Record a tool call
    pub fn record_tool_call(
        &self,
        platform: &str,
        tool_name: &str,
        outcome: &str,
        duration_secs: f64,
    ) {
        self.tool_calls_total
            .with_label_values(&[platform, tool_name, outcome])
            .inc();
        self.tool_call_duration_seconds
            .with_label_values(&[platform, tool_name])
            .observe(duration_secs);
    }

    /// Set context window usage
    pub fn set_context_window_usage(&self, platform: &str, model: &str, agent_id: &str, tokens: i64) {
        self.context_window_usage
            .with_label_values(&[platform, model, agent_id])
            .set(tokens);
    }

    /// Record a conversation turn
    pub fn record_conversation_turn(&self, platform: &str, agent_id: &str) {
        self.conversation_turns_total
            .with_label_values(&[platform, agent_id])
            .inc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resonance_metrics() {
        let registry = Registry::new();
        let metrics = ResonanceMetrics::new(&registry);

        metrics.record_ai_invocation("development", "claude-3-opus", "success", 2.5);
        metrics.record_tokens("development", "claude-3-opus", 1000, 500);
        metrics.record_tool_call("development", "web_search", "success", 0.5);
        metrics.record_memory_operation("development", "store", "episodic");
        metrics.set_context_window_usage("development", "claude-3-opus", "agent-1", 50000);

        let families = registry.gather();
        assert!(!families.is_empty());
    }
}
