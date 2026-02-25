//! Observation system invariants -- constants and runtime checks.
//!
//! These invariants are enforced, not optional:
//! - **I.OBS-1**: Overhead < 1% of total execution time
//! - **I.OBS-2**: Observation is MEANING input only -- never directly triggers action
//! - **I.OBS-3**: All observation data is provenance-tagged
//! - **I.OBS-4**: Memory usage is bounded
//! - **I.OBS-5**: Sampling never drops to zero for any event type

use crate::error::{ObservationError, ObservationResult};

/// I.OBS-1: Maximum fraction of execution time observation may consume.
pub const MAX_OVERHEAD_FRACTION: f64 = 0.01;

/// I.OBS-4: Maximum memory the observation subsystem may use (bytes).
pub const MAX_OBSERVATION_MEMORY_BYTES: usize = 64 * 1024 * 1024; // 64 MB

/// I.OBS-5: Minimum allowed sampling rate.
pub const MIN_SAMPLING_RATE: f64 = 0.001;

/// Default ring buffer capacity for raw observation events.
pub const DEFAULT_RING_BUFFER_CAPACITY: usize = 65_536;

/// Maximum retained aggregated windows per time-window size.
pub const MAX_WINDOWS_PER_SIZE: usize = 1440;

/// Runtime invariant checker for the observation subsystem.
pub struct InvariantChecker;

impl InvariantChecker {
    /// Check that memory usage is within the configured budget.
    pub fn check_memory_usage(current_bytes: usize, budget: usize) -> ObservationResult<()> {
        if current_bytes > budget {
            return Err(ObservationError::MemoryBudgetExceeded {
                used: current_bytes,
                limit: budget,
            });
        }
        Ok(())
    }

    /// Check that observation overhead is within the allowed fraction.
    pub fn check_overhead(observation_ns: u64, total_ns: u64) -> ObservationResult<()> {
        if total_ns == 0 {
            return Ok(());
        }
        let fraction = observation_ns as f64 / total_ns as f64;
        if fraction > MAX_OVERHEAD_FRACTION {
            return Err(ObservationError::InvariantViolation {
                invariant: "I.OBS-1".into(),
                detail: format!(
                    "observation overhead {:.4}% exceeds maximum {:.2}%",
                    fraction * 100.0,
                    MAX_OVERHEAD_FRACTION * 100.0
                ),
            });
        }
        Ok(())
    }

    /// Validate that a sampling rate is within allowed bounds.
    pub fn validate_sampling_rate(rate: f64) -> ObservationResult<()> {
        if rate < MIN_SAMPLING_RATE || rate > 1.0 {
            return Err(ObservationError::InvalidSamplingRate {
                rate,
                min: MIN_SAMPLING_RATE,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_within_budget_passes() {
        assert!(InvariantChecker::check_memory_usage(1000, 2000).is_ok());
    }

    #[test]
    fn memory_over_budget_fails() {
        assert!(InvariantChecker::check_memory_usage(3000, 2000).is_err());
    }

    #[test]
    fn memory_at_budget_passes() {
        assert!(InvariantChecker::check_memory_usage(2000, 2000).is_ok());
    }

    #[test]
    fn overhead_within_limit_passes() {
        // 0.5% overhead
        assert!(InvariantChecker::check_overhead(5, 1000).is_ok());
    }

    #[test]
    fn overhead_over_limit_fails() {
        // 2% overhead
        assert!(InvariantChecker::check_overhead(20, 1000).is_err());
    }

    #[test]
    fn overhead_with_zero_total_passes() {
        assert!(InvariantChecker::check_overhead(100, 0).is_ok());
    }

    #[test]
    fn sampling_rate_valid() {
        assert!(InvariantChecker::validate_sampling_rate(1.0).is_ok());
        assert!(InvariantChecker::validate_sampling_rate(0.5).is_ok());
        assert!(InvariantChecker::validate_sampling_rate(MIN_SAMPLING_RATE).is_ok());
    }

    #[test]
    fn sampling_rate_too_low() {
        assert!(InvariantChecker::validate_sampling_rate(0.0).is_err());
        assert!(InvariantChecker::validate_sampling_rate(0.0001).is_err());
    }

    #[test]
    fn sampling_rate_too_high() {
        assert!(InvariantChecker::validate_sampling_rate(1.1).is_err());
    }

    #[test]
    fn constants_are_reasonable() {
        assert!(MAX_OVERHEAD_FRACTION > 0.0 && MAX_OVERHEAD_FRACTION < 0.1);
        assert!(MAX_OBSERVATION_MEMORY_BYTES > 1024 * 1024); // at least 1MB
        assert!(MIN_SAMPLING_RATE > 0.0 && MIN_SAMPLING_RATE < 0.01);
        assert!(DEFAULT_RING_BUFFER_CAPACITY > 1024);
    }
}
