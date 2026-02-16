//! Core WorldLine APIs and shared domain model.
//!
//! This crate is the canonical import surface for identity and core type
//! primitives.

pub use worldline_identity as identity;
pub use worldline_types as types;

pub use identity::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::{identity::IdentityManager, types::IdentityMaterial, types::WorldlineId};

    #[test]
    fn facade_exports_worldline_identity_types() {
        let mut manager = IdentityManager::new();
        let material = IdentityMaterial::GenesisHash([7u8; 32]);
        let id = manager
            .create_worldline(material.clone())
            .expect("worldline should be created");

        let derived = WorldlineId::derive(&material);
        assert_eq!(id, derived);
    }
}
