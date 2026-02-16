//! WorldLine runtime orchestration and kernel subsystem entrypoints.
//!
//! This crate exposes runtime orchestration and core kernel services through a
//! consistent WorldLine namespace.

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
