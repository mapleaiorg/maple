//! MAPLE Model Router — policy-based model routing with circuit breaking.
//!
//! Routes inference requests to the optimal backend based on policies,
//! model capabilities, cost budgets, and health status.

pub mod backend;
pub mod circuit_breaker;
pub mod policy;

pub use backend::{BackendHealth, BackendInfo, BackendPricing, BackendRegistry};
pub use circuit_breaker::{CircuitBreakerConfig, CircuitBreakerManager, CircuitState};
pub use policy::{
    ClassificationRule, CostBudget, FallbackPolicy, ModelPreference, RoutingPolicy,
};
