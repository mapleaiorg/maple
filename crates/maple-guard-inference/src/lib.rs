//! MAPLE Guard Inference Gateway — Model Call Controls
//!
//! Enforces model allowlists/blocklists, parameter bounding, budget tracking,
//! and rate limiting for every LLM invocation.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ─── Types ───────────────────────────────────────────────────────────

/// Roles in an inference conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// An inference request passing through the gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub agent_id: Option<String>,
}

fn default_max_tokens() -> u32 {
    4096
}
fn default_temperature() -> f32 {
    0.7
}

/// Gateway response wrapping the inference result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
    pub finished_at: DateTime<Utc>,
}

// ─── Model Access ────────────────────────────────────────────────────

/// Verdict from the model access check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelAccessVerdict {
    Allowed,
    Denied(String),
}

/// Policy controlling which models agents may invoke.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelAccessPolicy {
    /// Explicit allowlist — if non-empty, only these models are allowed.
    pub allowlist: HashSet<String>,
    /// Explicit blocklist — these models are always denied.
    pub blocklist: HashSet<String>,
}

impl ModelAccessPolicy {
    /// Check whether a model is allowed by this policy.
    pub fn check(&self, model: &str) -> ModelAccessVerdict {
        if self.blocklist.contains(model) {
            return ModelAccessVerdict::Denied(format!("model '{}' is blocklisted", model));
        }
        if !self.allowlist.is_empty() && !self.allowlist.contains(model) {
            return ModelAccessVerdict::Denied(format!("model '{}' not in allowlist", model));
        }
        ModelAccessVerdict::Allowed
    }
}

// ─── Parameter Bounding ──────────────────────────────────────────────

/// Parameter bounds for inference requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamBounds {
    pub max_tokens_limit: u32,
    pub temperature_min: f32,
    pub temperature_max: f32,
    pub top_p_min: f32,
    pub top_p_max: f32,
}

impl Default for ParamBounds {
    fn default() -> Self {
        Self {
            max_tokens_limit: 16384,
            temperature_min: 0.0,
            temperature_max: 2.0,
            top_p_min: 0.0,
            top_p_max: 1.0,
        }
    }
}

impl ParamBounds {
    /// Clamp request parameters to within bounds.
    pub fn clamp(&self, request: &mut InferenceRequest) {
        if request.max_tokens > self.max_tokens_limit {
            debug!(
                original = request.max_tokens,
                clamped = self.max_tokens_limit,
                "Clamping max_tokens"
            );
            request.max_tokens = self.max_tokens_limit;
        }
        request.temperature = request
            .temperature
            .clamp(self.temperature_min, self.temperature_max);
        if let Some(ref mut top_p) = request.top_p {
            *top_p = top_p.clamp(self.top_p_min, self.top_p_max);
        }
    }
}

// ─── Budget Tracking ─────────────────────────────────────────────────

/// Budget entry tracking spend per agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetEntry {
    pub agent_id: String,
    pub total_tokens_used: u64,
    pub total_requests: u64,
    pub budget_limit_tokens: Option<u64>,
}

/// Budget tracker for per-agent token usage.
#[derive(Debug, Default)]
pub struct BudgetTracker {
    entries: HashMap<String, BudgetEntry>,
}

/// Budget-related errors.
#[derive(Debug, thiserror::Error)]
pub enum BudgetError {
    #[error("budget exceeded for agent '{agent_id}': used {used} of {limit} tokens")]
    BudgetExceeded {
        agent_id: String,
        used: u64,
        limit: u64,
    },
}

impl BudgetTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the token budget for an agent.
    pub fn set_budget(&mut self, agent_id: &str, limit: u64) {
        let entry = self.entries.entry(agent_id.to_string()).or_insert(BudgetEntry {
            agent_id: agent_id.to_string(),
            total_tokens_used: 0,
            total_requests: 0,
            budget_limit_tokens: None,
        });
        entry.budget_limit_tokens = Some(limit);
    }

    /// Check if the agent can afford the estimated tokens.
    pub fn check(&self, agent_id: &str, estimated_tokens: u64) -> Result<(), BudgetError> {
        if let Some(entry) = self.entries.get(agent_id) {
            if let Some(limit) = entry.budget_limit_tokens {
                if entry.total_tokens_used + estimated_tokens > limit {
                    return Err(BudgetError::BudgetExceeded {
                        agent_id: agent_id.to_string(),
                        used: entry.total_tokens_used,
                        limit,
                    });
                }
            }
        }
        Ok(())
    }

    /// Record token usage for an agent.
    pub fn record_usage(&mut self, agent_id: &str, tokens: u64) {
        let entry = self.entries.entry(agent_id.to_string()).or_insert(BudgetEntry {
            agent_id: agent_id.to_string(),
            total_tokens_used: 0,
            total_requests: 0,
            budget_limit_tokens: None,
        });
        entry.total_tokens_used += tokens;
        entry.total_requests += 1;
    }

    /// Get the budget entry for an agent.
    pub fn get_entry(&self, agent_id: &str) -> Option<&BudgetEntry> {
        self.entries.get(agent_id)
    }
}

// ─── Rate Limiting ───────────────────────────────────────────────────

/// Rate limit error.
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("rate limit exceeded for '{key}': {requests_in_window} requests in window")]
    Exceeded {
        key: String,
        requests_in_window: u64,
    },
}

/// Simple sliding-window rate limiter.
#[derive(Debug)]
pub struct RateLimiter {
    max_requests: u64,
    window: Duration,
    windows: HashMap<String, Vec<Instant>>,
}

impl RateLimiter {
    pub fn new(max_requests: u64, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            windows: HashMap::new(),
        }
    }

    /// Check if a request is allowed and record it.
    pub fn check_and_record(&mut self, key: &str) -> Result<(), RateLimitError> {
        let now = Instant::now();
        let cutoff = now - self.window;
        let timestamps = self.windows.entry(key.to_string()).or_default();
        timestamps.retain(|t| *t > cutoff);

        if timestamps.len() as u64 >= self.max_requests {
            return Err(RateLimitError::Exceeded {
                key: key.to_string(),
                requests_in_window: timestamps.len() as u64,
            });
        }

        timestamps.push(now);
        Ok(())
    }
}

// ─── Gateway Configuration ──────────────────────────────────────────

/// Configuration for the inference gateway.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub model_access: ModelAccessPolicy,
    pub param_bounds: ParamBounds,
    pub rate_limit_requests_per_minute: Option<u64>,
    pub default_budget_tokens: Option<u64>,
}

/// Errors from the inference gateway.
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("model access denied: {0}")]
    AccessDenied(String),
    #[error("budget exceeded: {0}")]
    Budget(#[from] BudgetError),
    #[error("rate limit exceeded: {0}")]
    RateLimit(#[from] RateLimitError),
    #[error("internal error: {0}")]
    Internal(String),
}

/// Request context for the gateway pipeline.
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub agent_id: String,
    pub request_id: String,
}

/// The inference gateway — enforcement point for all model calls.
pub struct InferenceGateway {
    config: GatewayConfig,
    budget_tracker: BudgetTracker,
    rate_limiter: Option<RateLimiter>,
}

impl InferenceGateway {
    /// Create a new gateway with the given configuration.
    pub fn new(config: GatewayConfig) -> Self {
        let rate_limiter = config
            .rate_limit_requests_per_minute
            .map(|rpm| RateLimiter::new(rpm, Duration::from_secs(60)));
        Self {
            config,
            budget_tracker: BudgetTracker::new(),
            rate_limiter,
        }
    }

    /// Set the token budget for an agent.
    pub fn set_budget(&mut self, agent_id: &str, limit: u64) {
        self.budget_tracker.set_budget(agent_id, limit);
    }

    /// Pre-flight check: validate a request before sending to the model.
    pub fn preflight(
        &mut self,
        request: &mut InferenceRequest,
        ctx: &RequestContext,
    ) -> Result<(), GatewayError> {
        match self.config.model_access.check(&request.model) {
            ModelAccessVerdict::Allowed => {}
            ModelAccessVerdict::Denied(reason) => {
                warn!(agent_id = %ctx.agent_id, model = %request.model, "Model access denied");
                return Err(GatewayError::AccessDenied(reason));
            }
        }
        self.config.param_bounds.clamp(request);
        self.budget_tracker
            .check(&ctx.agent_id, request.max_tokens as u64)?;
        if let Some(ref mut limiter) = self.rate_limiter {
            limiter.check_and_record(&ctx.agent_id)?;
        }
        info!(
            agent_id = %ctx.agent_id,
            model = %request.model,
            max_tokens = request.max_tokens,
            "Request pre-flight passed"
        );
        Ok(())
    }

    /// Post-flight: record token usage after a successful model call.
    pub fn postflight(&mut self, ctx: &RequestContext, usage: &TokenUsage) {
        self.budget_tracker
            .record_usage(&ctx.agent_id, usage.total_tokens);
        info!(
            agent_id = %ctx.agent_id,
            tokens = usage.total_tokens,
            "Token usage recorded"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_access_allow_all() {
        let policy = ModelAccessPolicy::default();
        assert_eq!(policy.check("gpt-4"), ModelAccessVerdict::Allowed);
    }

    #[test]
    fn test_model_access_blocklist() {
        let mut policy = ModelAccessPolicy::default();
        policy.blocklist.insert("dangerous-model".to_string());
        assert_eq!(
            policy.check("dangerous-model"),
            ModelAccessVerdict::Denied("model 'dangerous-model' is blocklisted".to_string())
        );
        assert_eq!(policy.check("safe-model"), ModelAccessVerdict::Allowed);
    }

    #[test]
    fn test_model_access_allowlist() {
        let mut policy = ModelAccessPolicy::default();
        policy.allowlist.insert("approved-model".to_string());
        assert_eq!(policy.check("approved-model"), ModelAccessVerdict::Allowed);
        assert!(matches!(
            policy.check("unapproved-model"),
            ModelAccessVerdict::Denied(_)
        ));
    }

    #[test]
    fn test_param_bounds_clamping() {
        let bounds = ParamBounds {
            max_tokens_limit: 1000,
            temperature_min: 0.0,
            temperature_max: 1.5,
            top_p_min: 0.0,
            top_p_max: 1.0,
        };
        let mut request = InferenceRequest {
            model: "test".to_string(),
            messages: vec![],
            max_tokens: 50000,
            temperature: 3.0,
            top_p: Some(2.0),
            agent_id: None,
        };
        bounds.clamp(&mut request);
        assert_eq!(request.max_tokens, 1000);
        assert!((request.temperature - 1.5).abs() < f32::EPSILON);
        assert!((request.top_p.unwrap() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_budget_tracker_within_limit() {
        let mut tracker = BudgetTracker::new();
        tracker.set_budget("agent-1", 10000);
        tracker.record_usage("agent-1", 5000);
        assert!(tracker.check("agent-1", 4000).is_ok());
    }

    #[test]
    fn test_budget_tracker_exceeds_limit() {
        let mut tracker = BudgetTracker::new();
        tracker.set_budget("agent-1", 10000);
        tracker.record_usage("agent-1", 9000);
        assert!(tracker.check("agent-1", 2000).is_err());
    }

    #[test]
    fn test_budget_tracker_no_limit() {
        let tracker = BudgetTracker::new();
        assert!(tracker.check("agent-1", 999999).is_ok());
    }

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let mut limiter = RateLimiter::new(5, Duration::from_secs(60));
        for _ in 0..5 {
            assert!(limiter.check_and_record("agent-1").is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_excess() {
        let mut limiter = RateLimiter::new(3, Duration::from_secs(60));
        for _ in 0..3 {
            limiter.check_and_record("agent-1").unwrap();
        }
        assert!(limiter.check_and_record("agent-1").is_err());
    }

    #[test]
    fn test_gateway_preflight_denied_model() {
        let mut config = GatewayConfig::default();
        config
            .model_access
            .blocklist
            .insert("blocked-model".to_string());
        let mut gw = InferenceGateway::new(config);

        let mut req = InferenceRequest {
            model: "blocked-model".to_string(),
            messages: vec![],
            max_tokens: 100,
            temperature: 0.7,
            top_p: None,
            agent_id: Some("agent-1".to_string()),
        };
        let ctx = RequestContext {
            agent_id: "agent-1".to_string(),
            request_id: "req-1".to_string(),
        };
        assert!(gw.preflight(&mut req, &ctx).is_err());
    }

    #[test]
    fn test_gateway_preflight_success() {
        let config = GatewayConfig::default();
        let mut gw = InferenceGateway::new(config);

        let mut req = InferenceRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message {
                role: Role::User,
                content: "Hello".to_string(),
            }],
            max_tokens: 100,
            temperature: 0.7,
            top_p: None,
            agent_id: Some("agent-1".to_string()),
        };
        let ctx = RequestContext {
            agent_id: "agent-1".to_string(),
            request_id: "req-1".to_string(),
        };
        assert!(gw.preflight(&mut req, &ctx).is_ok());
    }

    #[test]
    fn test_gateway_postflight_records_usage() {
        let config = GatewayConfig::default();
        let mut gw = InferenceGateway::new(config);
        gw.set_budget("agent-1", 100000);

        let ctx = RequestContext {
            agent_id: "agent-1".to_string(),
            request_id: "req-1".to_string(),
        };
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };
        gw.postflight(&ctx, &usage);

        let entry = gw.budget_tracker.get_entry("agent-1").unwrap();
        assert_eq!(entry.total_tokens_used, 150);
        assert_eq!(entry.total_requests, 1);
    }
}
