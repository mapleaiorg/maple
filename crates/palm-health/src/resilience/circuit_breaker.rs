//! Circuit breaker pattern for resilience.
//!
//! Prevents cascading failures by tracking failure rates and
//! temporarily blocking requests to unhealthy instances.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use palm_types::InstanceId;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::config::CircuitBreakerConfig;

/// State of a circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally.
    Closed,

    /// Circuit is open, requests are blocked.
    Open,

    /// Circuit is testing if the instance has recovered.
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::Open => write!(f, "open"),
            CircuitState::HalfOpen => write!(f, "half-open"),
        }
    }
}

/// Circuit breaker for a single instance.
///
/// Tracks failures and successes, transitioning between states:
/// - Closed: Normal operation, requests allowed
/// - Open: Too many failures, requests blocked
/// - Half-Open: Testing if instance recovered, limited requests allowed
pub struct CircuitBreaker {
    /// Instance this breaker is for.
    instance_id: InstanceId,

    /// Current state.
    state: RwLock<CircuitState>,

    /// Configuration.
    config: CircuitBreakerConfig,

    /// Number of consecutive failures in closed state.
    failure_count: AtomicU32,

    /// Number of consecutive successes in half-open state.
    success_count: AtomicU32,

    /// Number of requests allowed through in half-open state.
    half_open_requests: AtomicU32,

    /// Timestamp when circuit opened (milliseconds since epoch).
    opened_at: AtomicU64,

    /// Time of last state change.
    last_transition: RwLock<DateTime<Utc>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker for an instance.
    pub fn new(instance_id: InstanceId, config: CircuitBreakerConfig) -> Self {
        Self {
            instance_id,
            state: RwLock::new(CircuitState::Closed),
            config,
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            half_open_requests: AtomicU32::new(0),
            opened_at: AtomicU64::new(0),
            last_transition: RwLock::new(Utc::now()),
        }
    }

    /// Get the instance ID.
    pub fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    /// Get the current state.
    pub fn state(&self) -> CircuitState {
        // Check if we should transition from open to half-open
        self.check_timeout();
        *self.state.read().unwrap()
    }

    /// Check if a request should be allowed.
    pub fn allow_request(&self) -> bool {
        // First check for timeout-based transition
        self.check_timeout();

        let state = self.state.read().unwrap();
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => {
                // Allow limited requests in half-open state
                let current = self.half_open_requests.fetch_add(1, Ordering::SeqCst);
                current < self.config.half_open_max_requests
            }
        }
    }

    /// Record a successful operation.
    pub fn record_success(&self) {
        let mut state = self.state.write().unwrap();

        match *state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;

                if successes >= self.config.success_threshold {
                    // Enough successes, close the circuit
                    info!(
                        instance_id = %self.instance_id,
                        successes = successes,
                        "Circuit breaker closing after successful recovery"
                    );
                    self.transition_to(&mut state, CircuitState::Closed);
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
                debug!(
                    instance_id = %self.instance_id,
                    "Success recorded while circuit open"
                );
            }
        }
    }

    /// Record a failed operation.
    pub fn record_failure(&self) {
        let mut state = self.state.write().unwrap();

        match *state {
            CircuitState::Closed => {
                let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;

                if failures >= self.config.failure_threshold {
                    // Too many failures, open the circuit
                    warn!(
                        instance_id = %self.instance_id,
                        failures = failures,
                        "Circuit breaker opening due to failures"
                    );
                    self.transition_to(&mut state, CircuitState::Open);
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open goes back to open
                warn!(
                    instance_id = %self.instance_id,
                    "Circuit breaker re-opening after half-open failure"
                );
                self.transition_to(&mut state, CircuitState::Open);
            }
            CircuitState::Open => {
                // Already open, nothing to do
            }
        }
    }

    /// Force the circuit to a specific state.
    pub fn force_state(&self, new_state: CircuitState) {
        let mut state = self.state.write().unwrap();
        info!(
            instance_id = %self.instance_id,
            old_state = %*state,
            new_state = %new_state,
            "Circuit breaker state forced"
        );
        self.transition_to(&mut state, new_state);
    }

    /// Reset the circuit breaker to closed state.
    pub fn reset(&self) {
        self.force_state(CircuitState::Closed);
    }

    /// Get circuit breaker statistics.
    pub fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            instance_id: self.instance_id.clone(),
            state: self.state(),
            failure_count: self.failure_count.load(Ordering::SeqCst),
            success_count: self.success_count.load(Ordering::SeqCst),
            last_transition: *self.last_transition.read().unwrap(),
        }
    }

    /// Check if reset timeout has passed and transition from open to half-open.
    fn check_timeout(&self) {
        let state = *self.state.read().unwrap();
        if state != CircuitState::Open {
            return;
        }

        let opened_at = self.opened_at.load(Ordering::SeqCst);
        if opened_at == 0 {
            return;
        }

        let now = Instant::now();
        let opened_instant = Instant::now()
            - Duration::from_millis(
                (chrono::Utc::now().timestamp_millis() as u64).saturating_sub(opened_at),
            );

        if now.duration_since(opened_instant) >= self.config.reset_timeout {
            let mut state = self.state.write().unwrap();
            if *state == CircuitState::Open {
                info!(
                    instance_id = %self.instance_id,
                    "Circuit breaker transitioning to half-open after timeout"
                );
                self.transition_to(&mut state, CircuitState::HalfOpen);
            }
        }
    }

    /// Transition to a new state, resetting counters as needed.
    fn transition_to(&self, state: &mut CircuitState, new_state: CircuitState) {
        *state = new_state;
        *self.last_transition.write().unwrap() = Utc::now();

        match new_state {
            CircuitState::Closed => {
                self.failure_count.store(0, Ordering::SeqCst);
                self.success_count.store(0, Ordering::SeqCst);
                self.opened_at.store(0, Ordering::SeqCst);
            }
            CircuitState::Open => {
                self.success_count.store(0, Ordering::SeqCst);
                self.half_open_requests.store(0, Ordering::SeqCst);
                self.opened_at.store(
                    chrono::Utc::now().timestamp_millis() as u64,
                    Ordering::SeqCst,
                );
            }
            CircuitState::HalfOpen => {
                self.success_count.store(0, Ordering::SeqCst);
                self.half_open_requests.store(0, Ordering::SeqCst);
            }
        }
    }
}

/// Statistics for a circuit breaker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerStats {
    /// Instance ID.
    pub instance_id: InstanceId,

    /// Current state.
    pub state: CircuitState,

    /// Number of recorded failures.
    pub failure_count: u32,

    /// Number of recorded successes (in half-open).
    pub success_count: u32,

    /// Time of last state transition.
    pub last_transition: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CircuitBreakerConfig {
        CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            reset_timeout: Duration::from_millis(100),
            half_open_max_requests: 2,
        }
    }

    #[test]
    fn test_circuit_breaker_closed_to_open() {
        let breaker = CircuitBreaker::new(InstanceId::generate(), test_config());

        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.allow_request());

        // Record failures until threshold
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);

        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);

        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.allow_request());
    }

    #[test]
    fn test_circuit_breaker_success_resets_failures() {
        let breaker = CircuitBreaker::new(InstanceId::generate(), test_config());

        breaker.record_failure();
        breaker.record_failure();
        breaker.record_success(); // Should reset failure count

        breaker.record_failure();
        breaker.record_failure();
        // Still closed because we reset
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open_to_closed() {
        let breaker = CircuitBreaker::new(InstanceId::generate(), test_config());

        // Force to half-open for testing
        breaker.force_state(CircuitState::HalfOpen);
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Record successes until threshold
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open_failure() {
        let breaker = CircuitBreaker::new(InstanceId::generate(), test_config());

        breaker.force_state(CircuitState::HalfOpen);

        // Any failure in half-open goes back to open
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
    }
}
