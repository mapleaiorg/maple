//! Circuit breaker for backend health management.
//!
//! Implements the circuit-breaker pattern to prevent cascading failures
//! when backends become unhealthy. A tripped breaker stops routing to
//! a backend until a recovery probe succeeds.

use std::collections::HashMap;

/// Circuit breaker states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Backend is healthy, requests flow normally.
    Closed,
    /// Backend has experienced failures, allowing limited probe requests.
    HalfOpen,
    /// Backend is considered down, no requests are sent.
    Open,
}

/// Configuration for a circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit.
    pub failure_threshold: u32,
    /// Duration in seconds the circuit stays open before moving to half-open.
    pub recovery_timeout_secs: u64,
    /// Number of successful probes required in half-open state before closing.
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            recovery_timeout_secs: 30,
            success_threshold: 1,
        }
    }
}

/// Per-backend circuit breaker state.
#[derive(Debug, Clone)]
struct BreakerState {
    state: CircuitState,
    consecutive_failures: u32,
    consecutive_successes: u32,
    last_failure_epoch_secs: Option<u64>,
}

impl BreakerState {
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_failure_epoch_secs: None,
        }
    }
}

/// Manages circuit breakers for all backends in the pool.
#[derive(Debug)]
pub struct CircuitBreakerManager {
    config: CircuitBreakerConfig,
    breakers: HashMap<String, BreakerState>,
}

impl CircuitBreakerManager {
    /// Create a new circuit breaker manager with the given config.
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            breakers: HashMap::new(),
        }
    }

    /// Check whether a backend is allowed to receive requests.
    ///
    /// `now_epoch_secs` is the current wall-clock time as seconds since Unix epoch.
    pub fn is_allowed(&mut self, backend_id: &str, now_epoch_secs: u64) -> bool {
        let breaker = self
            .breakers
            .entry(backend_id.to_string())
            .or_insert_with(BreakerState::new);

        match breaker.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if recovery timeout has elapsed
                if let Some(last_failure) = breaker.last_failure_epoch_secs {
                    if now_epoch_secs.saturating_sub(last_failure)
                        >= self.config.recovery_timeout_secs
                    {
                        breaker.state = CircuitState::HalfOpen;
                        breaker.consecutive_successes = 0;
                        tracing::info!(
                            backend = backend_id,
                            "Circuit breaker transitioned to half-open"
                        );
                        true
                    } else {
                        false
                    }
                } else {
                    // No failure timestamp recorded — treat as closed
                    breaker.state = CircuitState::Closed;
                    true
                }
            }
            CircuitState::HalfOpen => {
                // Allow a probe request
                true
            }
        }
    }

    /// Record a successful request to a backend.
    pub fn record_success(&mut self, backend_id: &str) {
        let breaker = self
            .breakers
            .entry(backend_id.to_string())
            .or_insert_with(BreakerState::new);

        breaker.consecutive_failures = 0;
        breaker.consecutive_successes += 1;

        if breaker.state == CircuitState::HalfOpen
            && breaker.consecutive_successes >= self.config.success_threshold
        {
            breaker.state = CircuitState::Closed;
            tracing::info!(
                backend = backend_id,
                "Circuit breaker closed after successful probes"
            );
        }
    }

    /// Record a failed request to a backend.
    pub fn record_failure(&mut self, backend_id: &str, now_epoch_secs: u64) {
        let breaker = self
            .breakers
            .entry(backend_id.to_string())
            .or_insert_with(BreakerState::new);

        breaker.consecutive_successes = 0;
        breaker.consecutive_failures += 1;
        breaker.last_failure_epoch_secs = Some(now_epoch_secs);

        match breaker.state {
            CircuitState::Closed => {
                if breaker.consecutive_failures >= self.config.failure_threshold {
                    breaker.state = CircuitState::Open;
                    tracing::warn!(
                        backend = backend_id,
                        failures = breaker.consecutive_failures,
                        "Circuit breaker opened"
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open re-opens the circuit
                breaker.state = CircuitState::Open;
                tracing::warn!(
                    backend = backend_id,
                    "Circuit breaker re-opened from half-open state"
                );
            }
            CircuitState::Open => {
                // Already open, just update the failure timestamp
            }
        }
    }

    /// Get the current state for a backend. Returns `Closed` if no state is tracked.
    pub fn state(&self, backend_id: &str) -> CircuitState {
        self.breakers
            .get(backend_id)
            .map(|b| b.state)
            .unwrap_or(CircuitState::Closed)
    }

    /// Reset all circuit breakers to closed state.
    pub fn reset_all(&mut self) {
        self.breakers.clear();
    }

    /// Reset a single backend's circuit breaker.
    pub fn reset(&mut self, backend_id: &str) {
        self.breakers.remove(backend_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_closed() {
        let manager = CircuitBreakerManager::new(CircuitBreakerConfig::default());
        assert_eq!(manager.state("backend-a"), CircuitState::Closed);
    }

    #[test]
    fn test_opens_after_threshold_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout_secs: 30,
            success_threshold: 1,
        };
        let mut manager = CircuitBreakerManager::new(config);

        manager.record_failure("b1", 100);
        assert_eq!(manager.state("b1"), CircuitState::Closed);
        manager.record_failure("b1", 101);
        assert_eq!(manager.state("b1"), CircuitState::Closed);
        manager.record_failure("b1", 102);
        assert_eq!(manager.state("b1"), CircuitState::Open);
    }

    #[test]
    fn test_open_breaker_blocks_requests() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout_secs: 60,
            success_threshold: 1,
        };
        let mut manager = CircuitBreakerManager::new(config);
        manager.record_failure("b1", 100);
        assert!(!manager.is_allowed("b1", 110));
    }

    #[test]
    fn test_recovery_transitions_to_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout_secs: 30,
            success_threshold: 1,
        };
        let mut manager = CircuitBreakerManager::new(config);
        manager.record_failure("b1", 100);
        assert_eq!(manager.state("b1"), CircuitState::Open);

        // After recovery timeout, should transition to half-open
        assert!(manager.is_allowed("b1", 131));
        assert_eq!(manager.state("b1"), CircuitState::HalfOpen);
    }

    #[test]
    fn test_half_open_closes_on_success() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout_secs: 10,
            success_threshold: 1,
        };
        let mut manager = CircuitBreakerManager::new(config);
        manager.record_failure("b1", 100);

        // Transition to half-open
        manager.is_allowed("b1", 111);
        assert_eq!(manager.state("b1"), CircuitState::HalfOpen);

        // Success closes it
        manager.record_success("b1");
        assert_eq!(manager.state("b1"), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_reopens_on_failure() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout_secs: 10,
            success_threshold: 2,
        };
        let mut manager = CircuitBreakerManager::new(config);
        manager.record_failure("b1", 100);

        // Transition to half-open
        manager.is_allowed("b1", 111);
        assert_eq!(manager.state("b1"), CircuitState::HalfOpen);

        // Failure re-opens
        manager.record_failure("b1", 112);
        assert_eq!(manager.state("b1"), CircuitState::Open);
    }

    #[test]
    fn test_success_resets_failure_count() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout_secs: 30,
            success_threshold: 1,
        };
        let mut manager = CircuitBreakerManager::new(config);
        manager.record_failure("b1", 100);
        manager.record_failure("b1", 101);
        manager.record_success("b1"); // resets failures
        manager.record_failure("b1", 103);
        // Only 1 consecutive failure now, not 3
        assert_eq!(manager.state("b1"), CircuitState::Closed);
    }

    #[test]
    fn test_reset_clears_state() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout_secs: 30,
            success_threshold: 1,
        };
        let mut manager = CircuitBreakerManager::new(config);
        manager.record_failure("b1", 100);
        assert_eq!(manager.state("b1"), CircuitState::Open);

        manager.reset("b1");
        assert_eq!(manager.state("b1"), CircuitState::Closed);
    }
}
