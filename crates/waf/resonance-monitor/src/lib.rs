#![deny(unsafe_code)]
//! # maple-waf-resonance-monitor
//!
//! Dissonance detection — the sensory system of the WorldLine Autopoietic Factory.
//!
//! Monitors system metrics across three categories:
//! - **Semantic**: API friction, error patterns, workarounds
//! - **Computational**: CPU, memory, latency anomalies
//! - **Policy Drift**: boundary proximity, denial trends
//!
//! Produces [`IntentNode`]s for the context graph when dissonance exceeds thresholds.

pub mod detector;
pub mod error;
pub mod intent_builder;
pub mod orchestrator;
pub mod types;

pub use detector::DissonanceDetector;
pub use error::MonitorError;
pub use intent_builder::IntentBuilder;
pub use orchestrator::MonitorOrchestrator;
pub use types::{
    DissonanceCategory, DissonanceEvent, DissonanceThresholds, SystemMetrics,
};
