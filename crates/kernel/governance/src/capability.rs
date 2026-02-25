use std::collections::HashMap;

use maple_mwl_types::{
    Capability, CapabilityId, CapabilityScope, EffectDomain, RiskClass, TemporalAnchor,
    TemporalBounds, WorldlineId,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::AasError;

/// Capability Manager — bounded authority grants.
///
/// Per Whitepaper §6.3: "A capability defines the maximum scope of Commitments
/// an agent is authorized to declare."
///
/// Capabilities are:
/// - Granted by AAS (the normative authority)
/// - Scoped to specific effect domains
/// - Temporally bounded
/// - Revocable with recorded reason
pub struct CapabilityManager {
    /// Active capability grants per worldline
    grants: HashMap<WorldlineId, Vec<CapabilityGrant>>,
    /// Revocation history (append-only)
    revocations: Vec<RevocationRecord>,
    /// Next capability ID counter
    next_id: u64,
}

/// A capability grant with metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityGrant {
    pub capability: Capability,
    pub granted_at: TemporalAnchor,
    pub granted_by: String,
    pub active: bool,
}

/// Record of a capability revocation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RevocationRecord {
    pub capability_id: CapabilityId,
    pub worldline: WorldlineId,
    pub reason: String,
    pub revoked_at: TemporalAnchor,
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self {
            grants: HashMap::new(),
            revocations: Vec::new(),
            next_id: 1,
        }
    }

    /// Grant a capability to a worldline.
    ///
    /// Returns the assigned CapabilityId.
    pub fn grant(&mut self, to: &WorldlineId, cap: Capability) -> Result<CapabilityId, AasError> {
        let caps = self.grants.entry(to.clone()).or_default();

        // Check for duplicate: same id and domain
        let already_held = caps.iter().any(|g| g.active && g.capability.id == cap.id);

        if already_held {
            return Err(AasError::DuplicateCapability(cap.id.clone(), to.clone()));
        }

        let cap_id = CapabilityId(cap.id.clone());

        let grant = CapabilityGrant {
            capability: cap,
            granted_at: TemporalAnchor::now(0),
            granted_by: "AAS".into(),
            active: true,
        };

        info!(
            worldline = %to,
            capability = %cap_id.0,
            "Capability granted"
        );

        caps.push(grant);
        self.next_id += 1;

        Ok(cap_id)
    }

    /// Convenience: grant a capability by specifying only the essentials.
    pub fn grant_simple(
        &mut self,
        to: &WorldlineId,
        cap_id: impl Into<String>,
        name: impl Into<String>,
        domain: EffectDomain,
        risk_class: RiskClass,
    ) -> Result<CapabilityId, AasError> {
        let cap = Capability {
            id: cap_id.into(),
            name: name.into(),
            effect_domain: domain,
            scope: CapabilityScope {
                max_targets: None,
                allowed_targets: None,
                max_consequence_value: None,
                constraints: vec![],
            },
            temporal_validity: TemporalBounds {
                starts: TemporalAnchor::genesis(),
                expires: None,
                review_at: None,
            },
            risk_class,
            issuer: to.clone(),
            revocation_conditions: vec![],
        };

        self.grant(to, cap)
    }

    /// Revoke a capability.
    ///
    /// Records the revocation with reason. The capability is marked inactive
    /// but never deleted (audit trail preserved).
    pub fn revoke(
        &mut self,
        cap_id: &CapabilityId,
        wid: &WorldlineId,
        reason: &str,
    ) -> Result<(), AasError> {
        let caps = self
            .grants
            .get_mut(wid)
            .ok_or_else(|| AasError::CapabilityNotFound(cap_id.0.clone()))?;

        let grant = caps
            .iter_mut()
            .find(|g| g.active && g.capability.id == cap_id.0)
            .ok_or_else(|| AasError::CapabilityNotFound(cap_id.0.clone()))?;

        grant.active = false;

        let record = RevocationRecord {
            capability_id: cap_id.clone(),
            worldline: wid.clone(),
            reason: reason.into(),
            revoked_at: TemporalAnchor::now(0),
        };

        warn!(
            worldline = %wid,
            capability = %cap_id.0,
            reason = %reason,
            "Capability revoked"
        );

        self.revocations.push(record);
        Ok(())
    }

    /// Check if a worldline holds a specific (active) capability.
    pub fn check(&self, wid: &WorldlineId, required: &CapabilityId) -> bool {
        self.grants
            .get(wid)
            .map(|caps| {
                caps.iter()
                    .any(|g| g.active && g.capability.id == required.0)
            })
            .unwrap_or(false)
    }

    /// Get all active capabilities for a worldline.
    pub fn get_capabilities(&self, wid: &WorldlineId) -> Vec<&Capability> {
        self.grants
            .get(wid)
            .map(|caps| {
                caps.iter()
                    .filter(|g| g.active)
                    .map(|g| &g.capability)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all active grants for a worldline (with metadata).
    pub fn get_grants(&self, wid: &WorldlineId) -> Vec<&CapabilityGrant> {
        self.grants
            .get(wid)
            .map(|caps| caps.iter().filter(|g| g.active).collect())
            .unwrap_or_default()
    }

    /// Get revocation history.
    pub fn revocation_history(&self) -> &[RevocationRecord] {
        &self.revocations
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement the Gate's CapabilityProvider trait so the real CapabilityManager
/// can be used in the Commitment Gate pipeline (replacing mocks).
impl maple_kernel_gate::CapabilityProvider for CapabilityManager {
    fn has_capability(&self, wid: &WorldlineId, cap: &CapabilityId) -> bool {
        self.check(wid, cap)
    }

    fn get_capabilities(&self, wid: &WorldlineId) -> Vec<Capability> {
        self.get_capabilities(wid).into_iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_mwl_types::IdentityMaterial;

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    #[test]
    fn grant_and_check_capability() {
        let mut mgr = CapabilityManager::new();
        let wid = test_worldline();

        let cap_id = mgr
            .grant_simple(
                &wid,
                "CAP-COMM",
                "Communication",
                EffectDomain::Communication,
                RiskClass::Low,
            )
            .unwrap();

        assert!(mgr.check(&wid, &cap_id));
        assert_eq!(mgr.get_capabilities(&wid).len(), 1);
    }

    #[test]
    fn revoke_capability() {
        let mut mgr = CapabilityManager::new();
        let wid = test_worldline();

        let cap_id = mgr
            .grant_simple(
                &wid,
                "CAP-COMM",
                "Communication",
                EffectDomain::Communication,
                RiskClass::Low,
            )
            .unwrap();
        assert!(mgr.check(&wid, &cap_id));

        mgr.revoke(&cap_id, &wid, "Policy update").unwrap();
        assert!(!mgr.check(&wid, &cap_id));
        assert_eq!(mgr.get_capabilities(&wid).len(), 0);
    }

    #[test]
    fn revocation_preserved_in_history() {
        let mut mgr = CapabilityManager::new();
        let wid = test_worldline();

        let cap_id = mgr
            .grant_simple(
                &wid,
                "CAP-COMM",
                "Communication",
                EffectDomain::Communication,
                RiskClass::Low,
            )
            .unwrap();
        mgr.revoke(&cap_id, &wid, "Security concern").unwrap();

        assert_eq!(mgr.revocation_history().len(), 1);
        assert_eq!(mgr.revocation_history()[0].reason, "Security concern");
    }

    #[test]
    fn duplicate_grant_rejected() {
        let mut mgr = CapabilityManager::new();
        let wid = test_worldline();

        mgr.grant_simple(
            &wid,
            "CAP-COMM",
            "Communication",
            EffectDomain::Communication,
            RiskClass::Low,
        )
        .unwrap();
        let result = mgr.grant_simple(
            &wid,
            "CAP-COMM",
            "Communication",
            EffectDomain::Communication,
            RiskClass::Low,
        );
        assert!(result.is_err());
    }

    #[test]
    fn revoke_nonexistent_capability_fails() {
        let mut mgr = CapabilityManager::new();
        let wid = test_worldline();
        let cap_id = CapabilityId("NONEXISTENT".into());

        assert!(mgr.revoke(&cap_id, &wid, "test").is_err());
    }

    #[test]
    fn regrant_after_revocation() {
        let mut mgr = CapabilityManager::new();
        let wid = test_worldline();

        let cap_id = mgr
            .grant_simple(
                &wid,
                "CAP-COMM",
                "Communication",
                EffectDomain::Communication,
                RiskClass::Low,
            )
            .unwrap();
        mgr.revoke(&cap_id, &wid, "temporary").unwrap();

        // Re-grant after revocation should succeed
        let new_id = mgr
            .grant_simple(
                &wid,
                "CAP-COMM",
                "Communication",
                EffectDomain::Communication,
                RiskClass::Low,
            )
            .unwrap();
        assert!(mgr.check(&wid, &new_id));
    }

    #[test]
    fn implements_gate_capability_provider() {
        use maple_kernel_gate::CapabilityProvider;

        let mut mgr = CapabilityManager::new();
        let wid = test_worldline();
        let cap_id = CapabilityId("CAP-COMM".into());

        mgr.grant_simple(
            &wid,
            "CAP-COMM",
            "Communication",
            EffectDomain::Communication,
            RiskClass::Low,
        )
        .unwrap();

        // Use the trait method
        assert!(CapabilityProvider::has_capability(&mgr, &wid, &cap_id));
        assert_eq!(CapabilityProvider::get_capabilities(&mgr, &wid).len(), 1);
    }
}
