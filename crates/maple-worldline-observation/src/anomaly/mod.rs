//! Anomaly detection engine with multiple parallel algorithms and fusion.
//!
//! Runs statistical (z-score), percentile shift, trend (CUSUM), pattern,
//! and correlation detectors in parallel, then fuses results with temporal
//! clustering and component-based grouping.
//!
//! ## Architecture
//!
//! ```text
//!   current value + MetricBaseline
//!       │
//!       ├──► StatisticalAnomaly (z-score)
//!       ├──► PercentileAnomaly (p99 shift)
//!       ├──► TrendAnomaly (CUSUM)
//!       ├──► PatternAnomaly (seasonal)
//!       └──► CorrelationAnomaly (metric pairs)
//!             │
//!             ▼
//!       AnomalyFusion (agreement + dedup) ──► PerformanceAnomaly
//! ```

pub mod detector;
pub mod types;

pub use detector::{
    AnomalyAlgorithm, AnomalyDetector, CorrelationAnomaly, PatternAnomaly,
    PercentileAnomaly, StatisticalAnomaly, TrendAnomaly,
};
pub use types::{
    AnomalyCategory, AnomalyDetectorConfig, AnomalyId, AnomalySeverity, ComponentId,
    PerformanceAnomaly, RawAnomaly,
};

/// Default temporal clustering window for anomaly fusion (5 minutes).
pub const DEFAULT_FUSION_WINDOW_SECS: u64 = 300;

/// Default minimum detector agreement for anomaly acceptance.
pub const DEFAULT_MIN_AGREEMENT: f64 = 0.5;

/// Default z-score threshold for statistical anomaly detection.
pub const DEFAULT_Z_SCORE_THRESHOLD: f64 = 3.0;

/// Maximum retained anomalies (memory bound).
pub const MAX_ANOMALIES: usize = 256;
