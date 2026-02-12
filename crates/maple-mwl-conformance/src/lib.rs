//! MWL Constitutional Conformance Suite
//!
//! Verifies all 26 constitutional invariants of the Maple WorldLine Framework.
//! Each invariant is tested as an independent, self-contained assertion.
//!
//! Run with: `cargo test -p maple-mwl-conformance`

pub mod invariants;
pub mod report;

#[cfg(test)]
mod tests;
