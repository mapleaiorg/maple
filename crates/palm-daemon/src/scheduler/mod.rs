//! Scheduler and reconciliation loop
//!
//! The scheduler is responsible for:
//! - Periodically reconciling desired vs actual state
//! - Scaling deployments up/down
//! - Health monitoring and auto-healing
//! - Metrics collection

mod reconciler;

pub use reconciler::Scheduler;
