//! Resilience patterns for PALM health management.
//!
//! Provides circuit breaker, recovery actions, and resilience controller
//! for maintaining fleet health.

mod circuit_breaker;
mod recovery;
mod controller;

pub use circuit_breaker::{CircuitBreaker, CircuitState};
pub use recovery::{RecoveryAction, RecoveryContext, RecoveryOutcome, NotifySeverity};
pub use controller::{
    ResilienceController, RecoveryExecutor, RecoveryPolicyGate,
    NoOpRecoveryExecutor, FailingRecoveryExecutor, DefaultRecoveryPolicyGate
};
