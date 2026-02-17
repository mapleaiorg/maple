//! Compatibility wrapper for legacy `maple-mwl-conformance` imports.
//!
//! New code should depend on `worldline-conformance`.

pub use worldline_conformance::*;

#[cfg(test)]
mod tests {
    #[test]
    fn wrapper_exports_invariants() {
        assert_eq!(crate::invariants::ALL_INVARIANT_IDS.len(), 26);
    }
}
