#![deny(unsafe_code)]
//! # maple-waf-genesis
//!
//! 4-Phase Genesis Boot Protocol for the WorldLine Autopoietic Factory.
//!
//! ## Phases
//!
//! 1. **Substrate Attestation** — verify hardware and dependencies
//! 2. **Axiomatic Anchoring** — lock all 14 invariants
//! 3. **Observer Activation** — start resonance monitor
//! 4. **Reflexive Awakening** — first self-observation

pub mod config;
pub mod error;
pub mod genesis;

pub use config::SeedConfig;
pub use error::GenesisError;
pub use genesis::{create_worldline, genesis_boot, GenesisPhase, GenesisResult, Worldline};
