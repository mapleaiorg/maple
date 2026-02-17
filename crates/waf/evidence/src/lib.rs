#![deny(unsafe_code)]
//! # maple-waf-evidence
//!
//! Evidence & Equivalence System for the WorldLine Autopoietic Factory.
//!
//! Enforces **I.WAF-5: Evidence Completeness** — no swap without satisfying EvidenceBundle.
//!
//! ## Key Types
//!
//! - [`EvidenceBundle`] — Content-addressed evidence package (tests + invariants + repro build)
//! - [`EvidenceBuilder`] — Orchestrates evidence collection
//! - [`TestRunner`] / [`SimulatedTestRunner`] — Test execution interface
//! - [`InvariantChecker`] / [`SimulatedInvariantChecker`] — Invariant verification
//! - [`EquivalenceTier`] — E0 (tests) through E3 (ZK proof)

pub mod builder;
pub mod bundle;
pub mod error;
pub mod invariant_checker;
pub mod test_runner;
pub mod types;

pub use builder::EvidenceBuilder;
pub use bundle::EvidenceBundle;
pub use error::EvidenceError;
pub use invariant_checker::{InvariantChecker, SimulatedInvariantChecker, ALL_INVARIANT_IDS};
pub use test_runner::{SimulatedTestRunner, TestRunner};
pub use types::{EquivalenceTier, InvariantResult, ReproBuildResult, TestResult};
