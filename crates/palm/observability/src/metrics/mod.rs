//! Metrics collection and export for PALM
//!
//! Provides Prometheus-compatible metrics for monitoring PALM deployments.

pub mod collectors;
pub mod exporter;
pub mod registry;

pub use collectors::PalmMetrics;
pub use exporter::export_metrics;
pub use registry::MetricsRegistry;

#[cfg(feature = "http")]
pub use exporter::http::{metrics_handler, metrics_router, MetricsState};
