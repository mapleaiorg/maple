//! Resilience patterns for PALM health management.
//!
//! Provides circuit breaker, recovery actions, and resilience controller
//! for maintaining fleet health.

mod circuit_breaker;
mod controller;
mod recovery;

pub use circuit_breaker::{CircuitBreaker, CircuitState};
pub use controller::{
    DefaultRecoveryPolicyGate, FailingRecoveryExecutor, NoOpRecoveryExecutor, RecoveryExecutor,
    RecoveryPolicyGate, ResilienceController,
};
pub use recovery::{NotifySeverity, RecoveryAction, RecoveryContext, RecoveryOutcome};
