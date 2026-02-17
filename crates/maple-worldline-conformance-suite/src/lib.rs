//! Maple top-level conformance suite.
//!
//! This crate provides a stable Maple namespace wrapper over
//! `worldline-conformance`.

pub use worldline_conformance::*;

#[cfg(test)]
mod tests {
    #[test]
    fn exports_all_invariants() {
        assert!(!crate::invariants::ALL_INVARIANT_IDS.is_empty());
        assert!(crate::invariants::ALL_INVARIANT_IDS.contains(&"I.9"));
    }
}
