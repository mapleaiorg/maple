//! WorldLine Constitutional Conformance Suite
//!
//! Verifies the constitutional invariant set of the Maple WorldLine Framework.
//! Each invariant is tested as an independent, self-contained assertion.
//!
//! Run with: `cargo test -p worldline-conformance`

pub mod invariants;
pub mod report;

#[cfg(test)]
mod tests;
