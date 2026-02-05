//! Trace context propagation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// PALM-specific trace context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PalmContext {
    /// Trace ID (propagated across services)
    pub trace_id: String,

    /// Span ID (unique to this span)
    pub span_id: String,

    /// Parent span ID (if any)
    pub parent_span_id: Option<String>,

    /// Platform profile
    pub platform: String,

    /// Deployment ID (if applicable)
    pub deployment_id: Option<String>,

    /// Instance ID (if applicable)
    pub instance_id: Option<String>,

    /// Agent ID (if applicable)
    pub agent_id: Option<String>,

    /// Additional baggage items
    #[serde(default)]
    pub baggage: HashMap<String, String>,
}

impl PalmContext {
    /// Create a new root context
    pub fn new_root(platform: impl Into<String>) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            span_id: Uuid::new_v4().to_string(),
            parent_span_id: None,
            platform: platform.into(),
            deployment_id: None,
            instance_id: None,
            agent_id: None,
            baggage: HashMap::new(),
        }
    }

    /// Create a child context
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: Uuid::new_v4().to_string(),
            parent_span_id: Some(self.span_id.clone()),
            platform: self.platform.clone(),
            deployment_id: self.deployment_id.clone(),
            instance_id: self.instance_id.clone(),
            agent_id: self.agent_id.clone(),
            baggage: self.baggage.clone(),
        }
    }

    /// Set deployment context
    pub fn with_deployment(mut self, deployment_id: impl Into<String>) -> Self {
        self.deployment_id = Some(deployment_id.into());
        self
    }

    /// Set instance context
    pub fn with_instance(mut self, instance_id: impl Into<String>) -> Self {
        self.instance_id = Some(instance_id.into());
        self
    }

    /// Set agent context
    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Add a baggage item
    pub fn with_baggage(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.baggage.insert(key.into(), value.into());
        self
    }
}

/// Context propagation for cross-service tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationContext {
    /// W3C Trace Context traceparent header value
    pub traceparent: String,

    /// W3C Trace Context tracestate header value
    pub tracestate: Option<String>,

    /// PALM-specific context (serialized as JSON)
    pub palm_context: Option<String>,
}

impl PropagationContext {
    /// Create propagation context from PALM context
    pub fn from_palm_context(ctx: &PalmContext) -> Self {
        // Format: version-trace_id-span_id-flags
        let traceparent = format!(
            "00-{}-{}-01",
            ctx.trace_id.replace('-', ""),
            &ctx.span_id.replace('-', "")[..16]
        );

        let palm_json = serde_json::to_string(ctx).ok();

        Self {
            traceparent,
            tracestate: Some(format!("palm={}", ctx.platform)),
            palm_context: palm_json,
        }
    }

    /// Extract PALM context from propagation headers
    pub fn to_palm_context(&self, default_platform: &str) -> PalmContext {
        // Try to deserialize embedded PALM context first
        if let Some(ref palm_json) = self.palm_context {
            if let Ok(ctx) = serde_json::from_str::<PalmContext>(palm_json) {
                return ctx;
            }
        }

        // Fall back to parsing traceparent
        let parts: Vec<&str> = self.traceparent.split('-').collect();
        if parts.len() >= 3 {
            let trace_id = parts[1].to_string();
            let span_id = parts[2].to_string();

            return PalmContext {
                trace_id,
                span_id: Uuid::new_v4().to_string(),
                parent_span_id: Some(span_id),
                platform: default_platform.to_string(),
                deployment_id: None,
                instance_id: None,
                agent_id: None,
                baggage: HashMap::new(),
            };
        }

        // Couldn't parse, create new root
        PalmContext::new_root(default_platform)
    }

    /// Convert to HTTP headers
    pub fn to_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("traceparent".to_string(), self.traceparent.clone());
        if let Some(ref state) = self.tracestate {
            headers.insert("tracestate".to_string(), state.clone());
        }
        if let Some(ref palm) = self.palm_context {
            headers.insert("x-palm-context".to_string(), palm.clone());
        }
        headers
    }

    /// Parse from HTTP headers
    pub fn from_headers(headers: &HashMap<String, String>) -> Option<Self> {
        let traceparent = headers.get("traceparent")?.clone();
        Some(Self {
            traceparent,
            tracestate: headers.get("tracestate").cloned(),
            palm_context: headers.get("x-palm-context").cloned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palm_context_creation() {
        let ctx = PalmContext::new_root("development")
            .with_deployment("deploy-1")
            .with_instance("instance-1")
            .with_agent("agent-1")
            .with_baggage("user_id", "user-123");

        assert_eq!(ctx.platform, "development");
        assert_eq!(ctx.deployment_id, Some("deploy-1".to_string()));
        assert_eq!(ctx.instance_id, Some("instance-1".to_string()));
        assert_eq!(ctx.agent_id, Some("agent-1".to_string()));
        assert_eq!(ctx.baggage.get("user_id"), Some(&"user-123".to_string()));
    }

    #[test]
    fn test_child_context() {
        let parent = PalmContext::new_root("development").with_deployment("deploy-1");
        let child = parent.child();

        assert_eq!(child.trace_id, parent.trace_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id.clone()));
        assert_ne!(child.span_id, parent.span_id);
        assert_eq!(child.deployment_id, parent.deployment_id);
    }

    #[test]
    fn test_propagation_context() {
        let ctx = PalmContext::new_root("development");
        let prop = PropagationContext::from_palm_context(&ctx);

        assert!(prop.traceparent.starts_with("00-"));
        assert!(prop.tracestate.is_some());

        let headers = prop.to_headers();
        assert!(headers.contains_key("traceparent"));

        let restored = PropagationContext::from_headers(&headers).unwrap();
        let palm_ctx = restored.to_palm_context("development");
        assert_eq!(palm_ctx.trace_id, ctx.trace_id);
    }
}
