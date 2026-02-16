//! Phase A compatibility facade for WorldLine runtime and kernel subsystems.
//!
//! This crate re-exports existing runtime and kernel crates as stable module
//! aliases so adopters can migrate naming without API breakage.
//!
//! Backward compatibility:
//! - Existing `maple-runtime` and `maple-kernel-*` crates remain canonical.
//! - This facade is additive and non-breaking.

pub use maple_kernel_fabric as fabric;
pub use maple_kernel_financial as financial;
pub use maple_kernel_gate as gate;
pub use maple_kernel_governance as governance;
pub use maple_kernel_memory as memory;
pub use maple_kernel_mrp as mrp;
pub use maple_kernel_profiles as profiles;
pub use maple_kernel_provenance as provenance;
pub use maple_kernel_safety as safety;
pub use maple_runtime as runtime;

#[cfg(test)]
mod tests {
    use super::runtime;

    #[test]
    fn facade_exports_runtime_config() {
        let _cfg = runtime::config::RuntimeConfig::default();
    }
}
