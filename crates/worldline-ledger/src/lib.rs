//! Phase A compatibility facade for WorldLine ledger APIs.
//!
//! This crate provides a migration-friendly entrypoint by re-exporting:
//! - `maple-kernel-provenance` (provenance index and audit linkage)
//! - `maple-kernel-fabric` (event records used by provenance workflows)
//! - `maple-mwl-types` (canonical ledger IDs and shared primitives)
//!
//! The facade does not alter runtime behavior; it only standardizes naming.

pub use maple_kernel_fabric as fabric;
pub use maple_kernel_provenance as provenance;
pub use maple_mwl_types as types;

#[cfg(test)]
mod tests {
    use super::provenance::ProvenanceIndex;

    #[test]
    fn facade_exports_provenance_index() {
        let index = ProvenanceIndex::new();
        assert_eq!(index.len(), 0);
    }
}
