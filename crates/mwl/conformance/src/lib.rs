//! Legacy compatibility wrapper for `maple-mwl-conformance` imports.
//!
//! Prefer `maple-worldline-conformance-suite` (Maple namespace) or
//! `worldline-conformance` (canonical namespace) for new code.

pub use worldline_conformance::*;

#[cfg(test)]
mod tests {
    #[test]
    fn wrapper_exports_invariants() {
        assert!(!crate::invariants::ALL_INVARIANT_IDS.is_empty());
        assert!(crate::invariants::ALL_INVARIANT_IDS.contains(&"I.9"));
    }
}
