//! Compatibility wrapper for legacy `maple-mwl-integration` imports.
//!
//! New code should depend on `worldline-integration`.

pub use worldline_integration::*;

#[cfg(test)]
mod tests {
    #[test]
    fn wrapper_exports_helpers() {
        let _ = crate::helpers::KernelOptions::default();
    }
}
