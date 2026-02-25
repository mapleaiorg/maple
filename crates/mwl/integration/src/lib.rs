//! Legacy compatibility wrapper for `maple-mwl-integration` imports.
//!
//! Prefer `maple-worldline-integration-suite` (Maple namespace) or
//! `worldline-integration` (canonical namespace) for new code.

pub use worldline_integration::*;

#[cfg(test)]
mod tests {
    #[test]
    fn wrapper_exports_helpers() {
        let _ = crate::helpers::KernelOptions::default();
    }
}
