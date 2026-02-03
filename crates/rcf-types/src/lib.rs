//! RCF Type System - Resonance Commitment Language
//!
//! Constitutional type separation: Meaning ≠ Intent ≠ Commitment ≠ Consequence

#![deny(unsafe_code)]
#![cfg_attr(feature = "strict-docs", warn(missing_docs))]
#![cfg_attr(not(feature = "strict-docs"), allow(missing_docs))]
#![warn(rust_2018_idioms)]

mod capability;
mod errors;
mod identity;
mod resonance;
mod temporal;
mod uncertainty;

pub use capability::*;
pub use errors::*;
pub use identity::*;
pub use resonance::*;
pub use temporal::*;
pub use uncertainty::*;

/// Schema version
pub const SCHEMA_VERSION: &str = "1.0.0";
/// Commitment boundary level
pub const COMMITMENT_BOUNDARY_LEVEL: u8 = 2;
