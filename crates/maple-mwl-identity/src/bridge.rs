//! Bridge between existing ResonatorId and new WorldlineId.
//! This ensures backward compatibility with existing examples.

use maple_mwl_types::{IdentityMaterial, WorldlineId};

/// Convert existing resonator identifiers to WorldlineId.
/// Hashes the string to create a GenesisHash material, then derives through the standard path.
pub fn resonator_id_to_worldline(resonator_id: &str) -> WorldlineId {
    let hash = blake3::hash(resonator_id.as_bytes());
    let material = IdentityMaterial::GenesisHash(*hash.as_bytes());
    WorldlineId::derive_with_label(&material, resonator_id)
}

/// Re-export WorldlineId as a type alias for migration.
pub type ResonatorIdentity = WorldlineId;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resonator_id_to_worldline_is_deterministic() {
        let wid1 = resonator_id_to_worldline("agent-001");
        let wid2 = resonator_id_to_worldline("agent-001");
        assert_eq!(wid1, wid2);
    }

    #[test]
    fn different_resonator_ids_produce_different_worldlines() {
        let wid1 = resonator_id_to_worldline("agent-001");
        let wid2 = resonator_id_to_worldline("agent-002");
        assert_ne!(wid1, wid2);
    }

    #[test]
    fn preserves_label() {
        let wid = resonator_id_to_worldline("my-resonator");
        assert_eq!(wid.label(), Some("my-resonator"));
    }

    #[test]
    fn display_includes_resonator_name() {
        let wid = resonator_id_to_worldline("alpha");
        let display = format!("{}", wid);
        assert!(display.contains("alpha"));
    }
}
