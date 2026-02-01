//! Distributed tracing for PALM
//!
//! Provides OpenTelemetry-compatible tracing with context propagation.

pub mod context;
pub mod exporter;
pub mod spans;

pub use context::{PalmContext, PropagationContext};
pub use exporter::{init_tracing, shutdown_tracing, TracingConfig};
pub use spans::{PalmSpan, SpanBuilder, SpanKind, SpanStatus, SpanValue};
