//! WorldLine ledger, provenance, and replay service entrypoints.
//!
//! This crate exposes ledger and provenance services through a consistent
//! WorldLine namespace.

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
