//! Maple top-level integration suite.
//!
//! This crate provides a stable Maple namespace wrapper over
//! `worldline-integration`.

pub use worldline_integration::*;

#[cfg(test)]
mod tests {
    #[test]
    fn exports_kernel_helpers() {
        let _ = crate::helpers::KernelOptions::default();
    }
}
