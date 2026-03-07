//! MAPLE Foundry Traces — trace collection and dataset curation.
//!
//! Captures interaction traces from production agent executions for
//! use in evaluation, distillation, and continuous improvement.

pub mod trace;

pub use trace::{
    InteractionTrace, TokenUsageTrace, TraceInput, TraceMessage, TraceOutcome,
    TraceOutput, TraceToolCall, TraceToolCallResult,
};
