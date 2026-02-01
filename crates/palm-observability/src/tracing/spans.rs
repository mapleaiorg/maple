//! PALM span types and builders

use super::context::PalmContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{span, Level};

/// PALM span representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PalmSpan {
    /// Span name/operation
    pub name: String,

    /// Span kind
    pub kind: SpanKind,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time (when span is finished)
    pub end_time: Option<DateTime<Utc>>,

    /// Context information
    pub context: PalmContext,

    /// Span status
    pub status: SpanStatus,

    /// Span attributes
    pub attributes: HashMap<String, SpanValue>,

    /// Events recorded during span
    pub events: Vec<SpanEvent>,
}

impl PalmSpan {
    /// Create a new span builder
    pub fn builder(name: impl Into<String>) -> SpanBuilder {
        SpanBuilder::new(name)
    }

    /// Get duration if span is finished
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.end_time.map(|end| end - self.start_time)
    }

    /// Add an event to the span
    pub fn add_event(&mut self, name: impl Into<String>) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp: Utc::now(),
            attributes: HashMap::new(),
        });
    }

    /// Add an event with attributes
    pub fn add_event_with_attributes(
        &mut self,
        name: impl Into<String>,
        attributes: HashMap<String, SpanValue>,
    ) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp: Utc::now(),
            attributes,
        });
    }

    /// Set span status to error
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.status = SpanStatus::Error {
            message: message.into(),
        };
    }

    /// Mark span as finished
    pub fn finish(&mut self) {
        if self.end_time.is_none() {
            self.end_time = Some(Utc::now());
        }
    }

    /// Create a tracing span for integration with tracing crate
    pub fn to_tracing_span(&self) -> span::Span {
        span!(
            Level::INFO,
            "palm_span",
            name = %self.name,
            trace_id = %self.context.trace_id,
            span_id = %self.context.span_id,
            platform = %self.context.platform,
            deployment_id = ?self.context.deployment_id,
            instance_id = ?self.context.instance_id,
        )
    }
}

/// Span builder for fluent construction
#[derive(Debug)]
pub struct SpanBuilder {
    name: String,
    kind: SpanKind,
    context: Option<PalmContext>,
    attributes: HashMap<String, SpanValue>,
}

impl SpanBuilder {
    /// Create a new builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: SpanKind::Internal,
            context: None,
            attributes: HashMap::new(),
        }
    }

    /// Set span kind
    pub fn kind(mut self, kind: SpanKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set context
    pub fn context(mut self, context: PalmContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Add an attribute
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<SpanValue>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Build the span
    pub fn build(self, platform: impl Into<String>) -> PalmSpan {
        let context = self.context.unwrap_or_else(|| PalmContext::new_root(platform));
        PalmSpan {
            name: self.name,
            kind: self.kind,
            start_time: Utc::now(),
            end_time: None,
            context,
            status: SpanStatus::Ok,
            attributes: self.attributes,
            events: Vec::new(),
        }
    }

    /// Build a child span from a parent context
    pub fn build_child(self, parent: &PalmContext) -> PalmSpan {
        PalmSpan {
            name: self.name,
            kind: self.kind,
            start_time: Utc::now(),
            end_time: None,
            context: parent.child(),
            status: SpanStatus::Ok,
            attributes: self.attributes,
            events: Vec::new(),
        }
    }
}

/// Span kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanKind {
    /// Internal operation
    Internal,
    /// Server receiving request
    Server,
    /// Client sending request
    Client,
    /// Producer sending message
    Producer,
    /// Consumer receiving message
    Consumer,
}

/// Span status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanStatus {
    /// Unset status
    Unset,
    /// Operation succeeded
    Ok,
    /// Operation failed
    Error { message: String },
}

/// Span attribute value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SpanValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    StringArray(Vec<String>),
    IntArray(Vec<i64>),
}

impl From<String> for SpanValue {
    fn from(v: String) -> Self {
        SpanValue::String(v)
    }
}

impl From<&str> for SpanValue {
    fn from(v: &str) -> Self {
        SpanValue::String(v.to_string())
    }
}

impl From<i64> for SpanValue {
    fn from(v: i64) -> Self {
        SpanValue::Int(v)
    }
}

impl From<i32> for SpanValue {
    fn from(v: i32) -> Self {
        SpanValue::Int(v as i64)
    }
}

impl From<f64> for SpanValue {
    fn from(v: f64) -> Self {
        SpanValue::Float(v)
    }
}

impl From<bool> for SpanValue {
    fn from(v: bool) -> Self {
        SpanValue::Bool(v)
    }
}

/// Span event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    /// Event name
    pub name: String,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event attributes
    pub attributes: HashMap<String, SpanValue>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_builder() {
        let span = PalmSpan::builder("test_operation")
            .kind(SpanKind::Server)
            .attribute("http.method", "GET")
            .attribute("http.status_code", 200i64)
            .build("development");

        assert_eq!(span.name, "test_operation");
        assert_eq!(span.kind, SpanKind::Server);
        assert_eq!(span.status, SpanStatus::Ok);
        assert!(span.attributes.contains_key("http.method"));
    }

    #[test]
    fn test_child_span() {
        let parent_ctx = PalmContext::new_root("development");
        let child = PalmSpan::builder("child_operation")
            .kind(SpanKind::Internal)
            .build_child(&parent_ctx);

        assert_eq!(child.context.trace_id, parent_ctx.trace_id);
        assert_eq!(child.context.parent_span_id, Some(parent_ctx.span_id));
    }

    #[test]
    fn test_span_events() {
        let mut span = PalmSpan::builder("test").build("development");
        span.add_event("checkpoint_reached");
        span.add_event_with_attributes(
            "data_processed",
            [("records".to_string(), SpanValue::Int(100))]
                .into_iter()
                .collect(),
        );

        assert_eq!(span.events.len(), 2);
        assert_eq!(span.events[0].name, "checkpoint_reached");
        assert_eq!(span.events[1].name, "data_processed");
    }

    #[test]
    fn test_span_finish() {
        let mut span = PalmSpan::builder("test").build("development");
        assert!(span.end_time.is_none());

        span.finish();
        assert!(span.end_time.is_some());
        assert!(span.duration().is_some());
    }
}
