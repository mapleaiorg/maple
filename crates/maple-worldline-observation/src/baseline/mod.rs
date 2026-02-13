//! Baseline engine for establishing per-metric behavioral baselines.
//!
//! Uses EWMA (Exponentially Weighted Moving Average) for online learning
//! and tracks percentile distributions, seasonal patterns, and distribution shape.
//!
//! ## Architecture
//!
//! ```text
//!   ObservationSnapshot ──► BaselineEngine ──► MetricBaseline (per metric)
//!                              │
//!                              ├── EWMA mean/variance
//!                              ├── Percentile buffer (bounded)
//!                              ├── Seasonal hour-of-day buckets
//!                              └── Distribution shape detection
//! ```

pub mod engine;
pub mod persistence;
pub mod types;

pub use engine::BaselineEngine;
pub use persistence::{BaselinePersistence, InMemoryBaseline, JsonFileBaseline};
pub use types::{
    BaselineConfig, DistributionModel, MetricBaseline, MetricId, PercentileEstimates,
    SeasonalPattern, SeasonalPatternType,
};

/// Default learning rate for EWMA adaptation.
pub const DEFAULT_LEARNING_RATE: f64 = 0.01;

/// Minimum samples before a baseline is considered established.
pub const DEFAULT_MIN_ESTABLISHMENT_SAMPLES: u64 = 1000;

/// Minimum duration before a baseline is considered established (24h in seconds).
pub const DEFAULT_MIN_ESTABLISHMENT_SECS: u64 = 86_400;

/// Default percentile buffer size per metric.
pub const DEFAULT_PERCENTILE_BUFFER_SIZE: usize = 10_000;

/// Maximum tracked metrics (memory bound).
pub const MAX_TRACKED_METRICS: usize = 256;
