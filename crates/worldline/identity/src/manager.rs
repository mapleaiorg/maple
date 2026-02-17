use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use worldline_types::{IdentityMaterial, TemporalAnchor, WorldlineId};

use crate::continuity::{ContinuityChain, ContinuityContext, KeyRef};
use crate::error::IdentityError;

/// IdentityManager — manages worldline identities and their continuity.
pub struct IdentityManager {
    /// Known worldline identities
    identities: HashMap<WorldlineId, IdentityRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityRecord {
    pub worldline_id: WorldlineId,
    pub material: IdentityMaterial,
    pub continuity: ContinuityChain,
    pub created_at: TemporalAnchor,
    pub last_seen: TemporalAnchor,
    pub metadata: HashMap<String, String>,
}

impl IdentityManager {
    pub fn new() -> Self {
        Self {
            identities: HashMap::new(),
        }
    }

    /// Create a new worldline identity.
    pub fn create_worldline(
        &mut self,
        material: IdentityMaterial,
    ) -> Result<WorldlineId, IdentityError> {
        let wid = WorldlineId::derive(&material);

        if self.identities.contains_key(&wid) {
            return Err(IdentityError::AlreadyExists);
        }

        // Derive a key reference from the material for the initial continuity segment
        let key_ref = key_ref_from_material(&material);
        let now = TemporalAnchor::now(0);
        let chain = ContinuityChain::new(wid.clone(), key_ref);

        let record = IdentityRecord {
            worldline_id: wid.clone(),
            material,
            continuity: chain,
            created_at: now,
            last_seen: now,
            metadata: HashMap::new(),
        };

        self.identities.insert(wid.clone(), record);
        Ok(wid)
    }

    /// Resume a worldline (start new continuity segment).
    pub fn resume_worldline(
        &mut self,
        wid: &WorldlineId,
        key_ref: KeyRef,
        state_hash: [u8; 32],
    ) -> Result<ContinuityContext, IdentityError> {
        let record = self
            .identities
            .get_mut(wid)
            .ok_or_else(|| IdentityError::NotFound(wid.short_id()))?;

        record.continuity.start_segment(key_ref, state_hash)?;
        record.last_seen = TemporalAnchor::now(0);

        record
            .continuity
            .current_context()
            .ok_or(IdentityError::NotActive)
    }

    /// Suspend a worldline (end current segment).
    pub fn suspend_worldline(
        &mut self,
        wid: &WorldlineId,
        state_hash: [u8; 32],
    ) -> Result<(), IdentityError> {
        let record = self
            .identities
            .get_mut(wid)
            .ok_or_else(|| IdentityError::NotFound(wid.short_id()))?;

        record.continuity.end_segment(state_hash)?;
        record.last_seen = TemporalAnchor::now(0);
        Ok(())
    }

    /// Look up a worldline by ID.
    pub fn lookup(&self, wid: &WorldlineId) -> Option<&IdentityRecord> {
        self.identities.get(wid)
    }

    /// Verify identity material matches a worldline.
    pub fn verify(&self, wid: &WorldlineId, material: &IdentityMaterial) -> bool {
        wid.verify_material(material)
    }

    /// Get continuity context for commitment attribution.
    pub fn continuity_context(&self, wid: &WorldlineId) -> Option<ContinuityContext> {
        self.identities
            .get(wid)
            .and_then(|record| record.continuity.current_context())
    }
}

impl Default for IdentityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Derive a KeyRef from identity material (for initial chain creation).
fn key_ref_from_material(material: &IdentityMaterial) -> KeyRef {
    let fingerprint = *material.compute_hash().as_bytes();
    match material {
        IdentityMaterial::Ed25519PublicKey(_) => KeyRef {
            key_id: format!("derived-{}", hex_short(&fingerprint)),
            algorithm: "ed25519".to_string(),
            fingerprint,
        },
        IdentityMaterial::Organizational { org_id, .. } => KeyRef {
            key_id: format!("org-{}", org_id),
            algorithm: "organizational".to_string(),
            fingerprint,
        },
        IdentityMaterial::Composite(_) => KeyRef {
            key_id: format!("composite-{}", hex_short(&fingerprint)),
            algorithm: "composite".to_string(),
            fingerprint,
        },
        IdentityMaterial::GenesisHash(_) => KeyRef {
            key_id: format!("genesis-{}", hex_short(&fingerprint)),
            algorithm: "genesis".to_string(),
            fingerprint,
        },
    }
}

fn hex_short(bytes: &[u8; 32]) -> String {
    bytes[..4]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_material() -> IdentityMaterial {
        IdentityMaterial::Ed25519PublicKey([42u8; 32])
    }

    fn test_key_ref(id: &str) -> KeyRef {
        KeyRef {
            key_id: id.to_string(),
            algorithm: "ed25519".to_string(),
            fingerprint: [0u8; 32],
        }
    }

    #[test]
    fn create_worldline_returns_unique_id() {
        let mut mgr = IdentityManager::new();
        let m1 = IdentityMaterial::Ed25519PublicKey([1u8; 32]);
        let m2 = IdentityMaterial::Ed25519PublicKey([2u8; 32]);
        let wid1 = mgr.create_worldline(m1).unwrap();
        let wid2 = mgr.create_worldline(m2).unwrap();
        assert_ne!(wid1, wid2);
    }

    #[test]
    fn create_duplicate_fails() {
        let mut mgr = IdentityManager::new();
        let m = test_material();
        mgr.create_worldline(m.clone()).unwrap();
        assert!(mgr.create_worldline(m).is_err());
    }

    #[test]
    fn resume_worldline_starts_new_segment() {
        let mut mgr = IdentityManager::new();
        let wid = mgr.create_worldline(test_material()).unwrap();

        mgr.suspend_worldline(&wid, [1u8; 32]).unwrap();
        let ctx = mgr
            .resume_worldline(&wid, test_key_ref("key-1"), [1u8; 32])
            .unwrap();
        assert_eq!(ctx.segment_index, 1);
    }

    #[test]
    fn suspend_worldline_ends_current_segment() {
        let mut mgr = IdentityManager::new();
        let wid = mgr.create_worldline(test_material()).unwrap();

        mgr.suspend_worldline(&wid, [1u8; 32]).unwrap();
        assert!(mgr.continuity_context(&wid).is_none());
    }

    #[test]
    fn suspend_resume_preserves_identity() {
        let mut mgr = IdentityManager::new();
        let material = test_material();
        let wid = mgr.create_worldline(material.clone()).unwrap();

        // Suspend
        mgr.suspend_worldline(&wid, [10u8; 32]).unwrap();

        // Resume
        let ctx = mgr
            .resume_worldline(&wid, test_key_ref("key-1"), [10u8; 32])
            .unwrap();

        // Identity is preserved (I.1: Worldline Primacy)
        assert_eq!(ctx.worldline_id, wid);
        assert!(mgr.verify(&wid, &material));
    }

    #[test]
    fn continuity_context_for_active_worldline() {
        let mut mgr = IdentityManager::new();
        let wid = mgr.create_worldline(test_material()).unwrap();

        let ctx = mgr.continuity_context(&wid).unwrap();
        assert_eq!(ctx.worldline_id, wid);
        assert_eq!(ctx.segment_index, 0);
    }

    #[test]
    fn continuity_context_none_when_suspended() {
        let mut mgr = IdentityManager::new();
        let wid = mgr.create_worldline(test_material()).unwrap();
        mgr.suspend_worldline(&wid, [1u8; 32]).unwrap();
        assert!(mgr.continuity_context(&wid).is_none());
    }

    #[test]
    fn lookup_returns_record() {
        let mut mgr = IdentityManager::new();
        let wid = mgr.create_worldline(test_material()).unwrap();
        let record = mgr.lookup(&wid).unwrap();
        assert_eq!(record.worldline_id, wid);
    }

    #[test]
    fn lookup_unknown_returns_none() {
        let mgr = IdentityManager::new();
        let wid = WorldlineId::ephemeral();
        assert!(mgr.lookup(&wid).is_none());
    }

    #[test]
    fn verify_with_matching_material() {
        let mgr = IdentityManager::new();
        let material = test_material();
        let wid = WorldlineId::derive(&material);
        assert!(mgr.verify(&wid, &material));
    }

    #[test]
    fn verify_with_wrong_material() {
        let mgr = IdentityManager::new();
        let m1 = IdentityMaterial::Ed25519PublicKey([1u8; 32]);
        let m2 = IdentityMaterial::Ed25519PublicKey([2u8; 32]);
        let wid = WorldlineId::derive(&m1);
        assert!(!mgr.verify(&wid, &m2));
    }
}
