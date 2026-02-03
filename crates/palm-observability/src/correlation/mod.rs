//! Event correlation for PALM
//!
//! Correlates events across deployments, instances, and operations.

pub mod engine;

pub use engine::{CorrelatedEvent, CorrelationEngine, CorrelationId, EventCorrelation};
