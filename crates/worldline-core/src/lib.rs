//! Phase A compatibility facade for WorldLine core primitives.
//!
//! This crate intentionally re-exports existing MAPLE crates without introducing
//! behavioral changes:
//!
//! - `maple-mwl-types` for canonical WorldLine type system
//! - `maple-mwl-identity` for identity/continuity management
//!
//! During the migration window, existing `maple-*` imports remain fully supported.
//! New code can adopt `worldline-core` as a stable facade.

pub use maple_mwl_identity as identity;
pub use maple_mwl_types as types;

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
