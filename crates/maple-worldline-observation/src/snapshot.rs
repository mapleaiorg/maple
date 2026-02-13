//! Observation snapshots — point-in-time summaries of system behavior.
//!
//! Snapshots are the primary output of the observation subsystem, consumed
//! by the meaning formation engine (Prompt 13).

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::usage::UsageAnalyticsSnapshot;

/// A point-in-time summary of the observation subsystem.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObservationSnapshot {
    /// When this snapshot was taken.
    pub timestamp: DateTime<Utc>,
    /// Total events observed since start.
    pub total_events_observed: u64,
    /// Current effective sampling rate.
    pub current_sampling_rate: f64,
    /// Estimated memory usage in bytes.
    pub memory_usage_bytes: usize,
    /// Per-subsystem summaries.
    pub subsystem_summaries: HashMap<String, SubsystemSummary>,
    /// Usage analytics snapshot.
    pub usage: UsageAnalyticsSnapshot,
}

impl ObservationSnapshot {
    /// Is the observation subsystem healthy?
    ///
    /// Healthy means: memory within budget, sampling rate above minimum.
    pub fn is_healthy(&self) -> bool {
        use crate::invariants::{MAX_OBSERVATION_MEMORY_BYTES, MIN_SAMPLING_RATE};
        self.memory_usage_bytes <= MAX_OBSERVATION_MEMORY_BYTES
            && self.current_sampling_rate >= MIN_SAMPLING_RATE
    }

    /// Total error count across all subsystems.
    pub fn total_errors(&self) -> u64 {
        self.subsystem_summaries
            .values()
            .map(|s| s.error_count)
            .sum()
    }
}

/// Summary of observations for a single kernel subsystem.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubsystemSummary {
    /// Number of events observed from this subsystem.
    pub events_observed: u64,
    /// Average latency (if applicable).
    pub avg_latency: Option<Duration>,
    /// Maximum latency observed.
    pub max_latency: Option<Duration>,
    /// Number of error/failure events.
    pub error_count: u64,
    /// Error rate (errors / total).
    pub error_rate: f64,
}

impl SubsystemSummary {
    /// Create an empty summary.
    pub fn empty() -> Self {
        Self {
            events_observed: 0,
            avg_latency: None,
            max_latency: None,
            error_count: 0,
            error_rate: 0.0,
        }
    }

    /// Update the summary with a new observation.
    pub fn record(&mut self, latency: Option<Duration>, is_error: bool) {
        self.events_observed += 1;
        if is_error {
            self.error_count += 1;
        }
        self.error_rate = if self.events_observed > 0 {
            self.error_count as f64 / self.events_observed as f64
        } else {
            0.0
        };

        if let Some(lat) = latency {
            self.max_latency = Some(match self.max_latency {
                Some(existing) => existing.max(lat),
                None => lat,
            });
            // Incremental average
            let old_avg = self.avg_latency.unwrap_or(Duration::ZERO);
            let n = self.events_observed as f64;
            let new_avg_ns =
                old_avg.as_nanos() as f64 + (lat.as_nanos() as f64 - old_avg.as_nanos() as f64) / n;
            self.avg_latency = Some(Duration::from_nanos(new_avg_ns as u64));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsystem_summary_recording() {
        let mut summary = SubsystemSummary::empty();
        summary.record(Some(Duration::from_millis(10)), false);
        summary.record(Some(Duration::from_millis(20)), false);
        summary.record(Some(Duration::from_millis(30)), true);

        assert_eq!(summary.events_observed, 3);
        assert_eq!(summary.error_count, 1);
        assert!((summary.error_rate - 1.0 / 3.0).abs() < 0.01);
        assert_eq!(summary.max_latency, Some(Duration::from_millis(30)));

        // Average should be ~20ms
        let avg = summary.avg_latency.unwrap();
        assert!(avg.as_millis() >= 19 && avg.as_millis() <= 21);
    }

    #[test]
    fn subsystem_summary_no_latency() {
        let mut summary = SubsystemSummary::empty();
        summary.record(None, false);
        summary.record(None, true);

        assert_eq!(summary.events_observed, 2);
        assert_eq!(summary.error_count, 1);
        assert!(summary.avg_latency.is_none());
        assert!(summary.max_latency.is_none());
    }

    #[test]
    fn observation_snapshot_health_check() {
        let snapshot = ObservationSnapshot {
            timestamp: Utc::now(),
            total_events_observed: 100,
            current_sampling_rate: 0.5,
            memory_usage_bytes: 1024,
            subsystem_summaries: HashMap::new(),
            usage: UsageAnalyticsSnapshot {
                total_operations: 50,
                estimated_unique_worldlines: 5,
                estimated_unique_commitments: 10,
                estimated_unique_event_types: 3,
            },
        };
        assert!(snapshot.is_healthy());
    }

    #[test]
    fn snapshot_unhealthy_memory() {
        let snapshot = ObservationSnapshot {
            timestamp: Utc::now(),
            total_events_observed: 100,
            current_sampling_rate: 0.5,
            memory_usage_bytes: 128 * 1024 * 1024, // 128MB > 64MB budget
            subsystem_summaries: HashMap::new(),
            usage: UsageAnalyticsSnapshot {
                total_operations: 0,
                estimated_unique_worldlines: 0,
                estimated_unique_commitments: 0,
                estimated_unique_event_types: 0,
            },
        };
        assert!(!snapshot.is_healthy());
    }

    #[test]
    fn snapshot_total_errors() {
        let mut summaries = HashMap::new();
        summaries.insert(
            "fabric".to_string(),
            SubsystemSummary {
                events_observed: 100,
                avg_latency: None,
                max_latency: None,
                error_count: 5,
                error_rate: 0.05,
            },
        );
        summaries.insert(
            "gate".to_string(),
            SubsystemSummary {
                events_observed: 50,
                avg_latency: None,
                max_latency: None,
                error_count: 3,
                error_rate: 0.06,
            },
        );

        let snapshot = ObservationSnapshot {
            timestamp: Utc::now(),
            total_events_observed: 150,
            current_sampling_rate: 1.0,
            memory_usage_bytes: 1024,
            subsystem_summaries: summaries,
            usage: UsageAnalyticsSnapshot {
                total_operations: 150,
                estimated_unique_worldlines: 10,
                estimated_unique_commitments: 20,
                estimated_unique_event_types: 5,
            },
        };
        assert_eq!(snapshot.total_errors(), 8);
    }

    #[test]
    fn snapshot_serialization() {
        let snapshot = ObservationSnapshot {
            timestamp: Utc::now(),
            total_events_observed: 42,
            current_sampling_rate: 0.8,
            memory_usage_bytes: 2048,
            subsystem_summaries: HashMap::new(),
            usage: UsageAnalyticsSnapshot {
                total_operations: 42,
                estimated_unique_worldlines: 3,
                estimated_unique_commitments: 7,
                estimated_unique_event_types: 2,
            },
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: ObservationSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total_events_observed, 42);
    }
}
