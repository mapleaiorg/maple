//! RCL Type System - Resonance Commitment Language
//!
//! Constitutional type separation: Meaning ≠ Intent ≠ Commitment ≠ Consequence

#![deny(unsafe_code)]
#![cfg_attr(feature = "strict-docs", warn(missing_docs))]
#![cfg_attr(not(feature = "strict-docs"), allow(missing_docs))]
#![warn(rust_2018_idioms)]

mod resonance;
mod identity;
mod temporal;
mod capability;
mod uncertainty;
mod errors;

pub use resonance::*;
pub use identity::*;
pub use temporal::*;
pub use capability::*;
pub use uncertainty::*;
pub use errors::*;

/// Schema version
pub const SCHEMA_VERSION: &str = "1.0.0";
/// Commitment boundary level
pub const COMMITMENT_BOUNDARY_LEVEL: u8 = 2;
