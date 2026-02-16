use serde::{Deserialize, Serialize};

/// WorldlineId — persistent identity without accounts/sessions.
///
/// NOT a UUID or account reference. It is a stable, distinguishable pattern
/// derived from identity material but not reducible to any single cryptographic key.
///
/// Properties (per Resonance Architecture v1.1 §3.2.2):
/// - Persistence: Remains referable across interactions
/// - Distinctiveness: Can be distinguished from other identities
/// - Continuity: Allows attribution of past actions and commitments
/// - Non-Anthropomorphism: Does not imply personhood, rights, or authority
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldlineId {
    /// Primary identity hash — derived from identity material, NOT the raw key
    identity_hash: [u8; 32],
    /// Human-readable label (optional, non-authoritative)
    label: Option<String>,
    /// Version of the identity derivation scheme
    derivation_version: u8,
}

impl WorldlineId {
    /// Derive a WorldlineId from identity material.
    /// The material can include: cryptographic public key, organizational identifier,
    /// genesis event hash, or combination thereof.
    ///
    /// CRITICAL: The identity IS NOT the crypto key. The key supports the identity.
    pub fn derive(material: &IdentityMaterial) -> Self {
        let hash = material.compute_hash();
        Self {
            identity_hash: *hash.as_bytes(),
            label: None,
            derivation_version: 1,
        }
    }

    /// Derive with a human-readable label attached.
    pub fn derive_with_label(material: &IdentityMaterial, label: impl Into<String>) -> Self {
        let mut id = Self::derive(material);
        id.label = Some(label.into());
        id
    }

    /// Create a WorldlineId for testing purposes.
    /// Each call produces a unique, random identity.
    pub fn ephemeral() -> Self {
        let random_bytes: [u8; 32] = {
            let uuid = uuid::Uuid::new_v4();
            let mut buf = [0u8; 32];
            let uuid_bytes = uuid.as_bytes();
            buf[..16].copy_from_slice(uuid_bytes);
            let uuid2 = uuid::Uuid::new_v4();
            buf[16..].copy_from_slice(uuid2.as_bytes());
            buf
        };
        let hash = blake3::hash(&random_bytes);
        Self {
            identity_hash: *hash.as_bytes(),
            label: Some("ephemeral".to_string()),
            derivation_version: 1,
        }
    }

    /// Verify that a piece of identity material matches this WorldlineId.
    pub fn verify_material(&self, material: &IdentityMaterial) -> bool {
        let derived = Self::derive(material);
        self.identity_hash == derived.identity_hash
            && self.derivation_version == derived.derivation_version
    }

    /// Short display form (first 8 bytes hex).
    pub fn short_id(&self) -> String {
        hex::encode(&self.identity_hash[..8])
    }

    /// Access the raw identity hash bytes.
    pub fn identity_hash(&self) -> &[u8; 32] {
        &self.identity_hash
    }

    /// Access the label if set.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Access the derivation version.
    pub fn derivation_version(&self) -> u8 {
        self.derivation_version
    }
}

impl std::fmt::Display for WorldlineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref label) = self.label {
            write!(f, "{}({})", label, self.short_id())
        } else {
            write!(f, "wl:{}", self.short_id())
        }
    }
}

/// Material used to derive identity. NOT the identity itself.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IdentityMaterial {
    /// Ed25519 public key (most common for AI agents)
    Ed25519PublicKey([u8; 32]),
    /// Organization identifier + derivation salt
    Organizational { org_id: String, salt: [u8; 16] },
    /// Composite: multiple sources combined
    Composite(Vec<IdentityMaterial>),
    /// Genesis event hash (for programmatically-created worldlines)
    GenesisHash([u8; 32]),
}

impl IdentityMaterial {
    /// Compute the blake3 hash of this material for identity derivation.
    pub fn compute_hash(&self) -> blake3::Hash {
        let mut hasher = blake3::Hasher::new();
        // Domain separation tag
        hasher.update(b"maple-mwl-identity-v1:");
        self.hash_into(&mut hasher);
        hasher.finalize()
    }

    fn hash_into(&self, hasher: &mut blake3::Hasher) {
        match self {
            IdentityMaterial::Ed25519PublicKey(key) => {
                hasher.update(b"ed25519:");
                hasher.update(key);
            }
            IdentityMaterial::Organizational { org_id, salt } => {
                hasher.update(b"org:");
                hasher.update(org_id.as_bytes());
                hasher.update(b":");
                hasher.update(salt);
            }
            IdentityMaterial::Composite(materials) => {
                hasher.update(b"composite:");
                let count = materials.len() as u32;
                hasher.update(&count.to_le_bytes());
                for m in materials {
                    m.hash_into(hasher);
                    hasher.update(b"|");
                }
            }
            IdentityMaterial::GenesisHash(hash) => {
                hasher.update(b"genesis:");
                hasher.update(hash);
            }
        }
    }
}

/// Hex encoding helpers (no external dep needed — small utility).
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            s.push(HEX_CHARS[(b >> 4) as usize] as char);
            s.push(HEX_CHARS[(b & 0xf) as usize] as char);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        for (i, b) in key.iter_mut().enumerate() {
            *b = i as u8;
        }
        key
    }

    #[test]
    fn derive_from_ed25519_produces_consistent_id() {
        let material = IdentityMaterial::Ed25519PublicKey(test_key());
        let id1 = WorldlineId::derive(&material);
        let id2 = WorldlineId::derive(&material);
        assert_eq!(id1, id2);
    }

    #[test]
    fn derive_from_different_material_produces_different_ids() {
        let m1 = IdentityMaterial::Ed25519PublicKey(test_key());
        let m2 = IdentityMaterial::Ed25519PublicKey([0xff; 32]);
        let id1 = WorldlineId::derive(&m1);
        let id2 = WorldlineId::derive(&m2);
        assert_ne!(id1, id2);
    }

    #[test]
    fn derive_is_deterministic() {
        let material = IdentityMaterial::GenesisHash([42u8; 32]);
        let id1 = WorldlineId::derive(&material);
        let id2 = WorldlineId::derive(&material);
        assert_eq!(id1.identity_hash, id2.identity_hash);
    }

    #[test]
    fn ephemeral_produces_unique_ids() {
        let id1 = WorldlineId::ephemeral();
        let id2 = WorldlineId::ephemeral();
        assert_ne!(id1, id2);
    }

    #[test]
    fn verify_material_returns_true_for_matching() {
        let material = IdentityMaterial::Ed25519PublicKey(test_key());
        let id = WorldlineId::derive(&material);
        assert!(id.verify_material(&material));
    }

    #[test]
    fn verify_material_returns_false_for_non_matching() {
        let m1 = IdentityMaterial::Ed25519PublicKey(test_key());
        let m2 = IdentityMaterial::Ed25519PublicKey([0xff; 32]);
        let id = WorldlineId::derive(&m1);
        assert!(!id.verify_material(&m2));
    }

    #[test]
    fn display_with_label() {
        let material = IdentityMaterial::Ed25519PublicKey(test_key());
        let id = WorldlineId::derive_with_label(&material, "agent-alpha");
        let display = format!("{}", id);
        assert!(display.starts_with("agent-alpha("));
        assert!(display.ends_with(')'));
    }

    #[test]
    fn display_without_label() {
        let material = IdentityMaterial::Ed25519PublicKey(test_key());
        let id = WorldlineId::derive(&material);
        let display = format!("{}", id);
        assert!(display.starts_with("wl:"));
    }

    #[test]
    fn short_id_is_16_hex_chars() {
        let id = WorldlineId::ephemeral();
        assert_eq!(id.short_id().len(), 16);
    }

    #[test]
    fn serialization_roundtrip() {
        let material = IdentityMaterial::Ed25519PublicKey(test_key());
        let id = WorldlineId::derive_with_label(&material, "test");
        let json = serde_json::to_string(&id).unwrap();
        let restored: WorldlineId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, restored);
    }

    #[test]
    fn composite_material_derives_consistently() {
        let m = IdentityMaterial::Composite(vec![
            IdentityMaterial::Ed25519PublicKey(test_key()),
            IdentityMaterial::GenesisHash([1u8; 32]),
        ]);
        let id1 = WorldlineId::derive(&m);
        let id2 = WorldlineId::derive(&m);
        assert_eq!(id1, id2);
    }

    #[test]
    fn organizational_material_derives() {
        let m = IdentityMaterial::Organizational {
            org_id: "maple-ai".to_string(),
            salt: [7u8; 16],
        };
        let id = WorldlineId::derive(&m);
        assert!(id.verify_material(&m));
    }
}
