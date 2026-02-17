//! WorldLine End-to-End Integration Tests
//!
//! Comprehensive cross-crate integration tests exercising the full
//! Resonance Architecture lifecycle and all kernel components together.
//!
//! Run with: `cargo test -p worldline-integration`

/// Shared test helpers — kernel setup, worldline creation, etc.
pub mod helpers;

#[cfg(test)]
mod commitment_denial;
#[cfg(test)]
mod cross_profile;
#[cfg(test)]
mod failure_recovery;
#[cfg(test)]
mod financial_settlement;
#[cfg(test)]
mod human_agency;
#[cfg(test)]
mod mrp_enforcement;
#[cfg(test)]
mod resonance_lifecycle;
