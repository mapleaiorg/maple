//! Permit types: scoped authority to perform actions
//!
//! A Permit grants a specific Resonator scoped authority to exercise
//! a Capability. Permits are time-bound, usage-limited, and auditable.

use crate::{CapabilityId, GrantAuthority, PermitLimits};
use chrono::{DateTime, Utc};
use rcf_types::EffectDomain;
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};

/// Unique identifier for a Permit
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PermitId(pub String);

impl PermitId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for PermitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A Permit grants scoped authority to perform a specific action
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Permit {
    /// Unique permit identifier
    pub id: PermitId,
    /// The capability this permit authorizes
    pub capability_id: CapabilityId,
    /// Who holds this permit
    pub grantee: ResonatorId,
    /// The scope of authorized actions
    pub scope: PermitScope,
    /// Usage limits
    pub limits: PermitLimits,
    /// When the permit expires
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<DateTime<Utc>>,
    /// Who issued the permit
    pub granter: GrantAuthority,
    /// When the permit was created
    pub created_at: DateTime<Utc>,
    /// Current status
    pub status: PermitStatus,
    /// Number of times used
    pub use_count: u64,
}

impl Permit {
    pub fn new(
        capability_id: CapabilityId,
        grantee: ResonatorId,
        scope: PermitScope,
        granter: GrantAuthority,
    ) -> Self {
        Self {
            id: PermitId::generate(),
            capability_id,
            grantee,
            scope,
            limits: PermitLimits::default(),
            expiry: None,
            granter,
            created_at: Utc::now(),
            status: PermitStatus::Active,
            use_count: 0,
        }
    }

    pub fn with_limits(mut self, limits: PermitLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn with_expiry(mut self, expiry: DateTime<Utc>) -> Self {
        self.expiry = Some(expiry);
        self
    }

    /// Check if the permit is currently usable
    pub fn is_usable(&self) -> bool {
        if self.status != PermitStatus::Active {
            return false;
        }

        // Check expiry
        if let Some(expiry) = self.expiry {
            if Utc::now() >= expiry {
                return false;
            }
        }

        // Check use count
        if let Some(max_uses) = self.limits.max_uses {
            if self.use_count >= max_uses {
                return false;
            }
        }

        true
    }

    /// Record a usage of this permit
    pub fn record_use(&mut self) {
        self.use_count += 1;

        // Auto-exhaust if max uses reached
        if let Some(max_uses) = self.limits.max_uses {
            if self.use_count >= max_uses {
                self.status = PermitStatus::Exhausted;
            }
        }
    }

    /// Revoke this permit
    pub fn revoke(&mut self) {
        self.status = PermitStatus::Revoked;
    }

    /// Check remaining uses (None = unlimited)
    pub fn remaining_uses(&self) -> Option<u64> {
        self.limits.max_uses.map(|max| max.saturating_sub(self.use_count))
    }
}

/// The scope of actions a permit authorizes
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PermitScope {
    /// Effect domains the permit covers
    pub domains: Vec<EffectDomain>,
    /// Specific targets the permit covers
    pub targets: Vec<String>,
    /// Operations allowed
    pub operations: Vec<String>,
}

impl PermitScope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_domain(mut self, domain: EffectDomain) -> Self {
        self.domains.push(domain);
        self
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.targets.push(target.into());
        self
    }

    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operations.push(operation.into());
        self
    }

    /// Check if this scope covers a specific domain
    pub fn covers_domain(&self, domain: &EffectDomain) -> bool {
        self.domains.is_empty() || self.domains.contains(domain)
    }

    /// Check if this scope covers a specific target
    pub fn covers_target(&self, target: &str) -> bool {
        self.targets.is_empty()
            || self.targets.iter().any(|t| t == "*" || t == target)
    }

    /// Check if this scope covers a specific operation
    pub fn covers_operation(&self, operation: &str) -> bool {
        self.operations.is_empty()
            || self.operations.iter().any(|o| o == "*" || o == operation)
    }
}

/// Status of a Permit
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PermitStatus {
    /// Permit is active and usable
    #[default]
    Active,
    /// Permit has expired
    Expired,
    /// Permit was explicitly revoked
    Revoked,
    /// Permit has been used up (max_uses reached)
    Exhausted,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CollectiveId;

    #[test]
    fn test_permit_id() {
        let id = PermitId::generate();
        assert!(!id.0.is_empty());
    }

    #[test]
    fn test_permit_lifecycle() {
        let mut permit = Permit::new(
            CapabilityId::new("read"),
            ResonatorId::new("res-1"),
            PermitScope::new()
                .with_domain(EffectDomain::Data)
                .with_operation("read"),
            GrantAuthority::Collective(CollectiveId::new("coll-1")),
        )
        .with_limits(PermitLimits::new().with_max_uses(3));

        assert!(permit.is_usable());
        assert_eq!(permit.remaining_uses(), Some(3));

        permit.record_use();
        assert!(permit.is_usable());
        assert_eq!(permit.remaining_uses(), Some(2));

        permit.record_use();
        permit.record_use();
        assert!(!permit.is_usable());
        assert_eq!(permit.status, PermitStatus::Exhausted);
        assert_eq!(permit.remaining_uses(), Some(0));
    }

    #[test]
    fn test_permit_revocation() {
        let mut permit = Permit::new(
            CapabilityId::new("write"),
            ResonatorId::new("res-1"),
            PermitScope::default(),
            GrantAuthority::Individual(ResonatorId::new("admin")),
        );

        assert!(permit.is_usable());
        permit.revoke();
        assert!(!permit.is_usable());
        assert_eq!(permit.status, PermitStatus::Revoked);
    }

    #[test]
    fn test_permit_scope() {
        let scope = PermitScope::new()
            .with_domain(EffectDomain::Finance)
            .with_target("account-123")
            .with_operation("transfer");

        assert!(scope.covers_domain(&EffectDomain::Finance));
        assert!(!scope.covers_domain(&EffectDomain::Data));
        assert!(scope.covers_target("account-123"));
        assert!(!scope.covers_target("account-456"));
        assert!(scope.covers_operation("transfer"));
        assert!(!scope.covers_operation("delete"));
    }

    #[test]
    fn test_empty_scope_covers_all() {
        let scope = PermitScope::new();
        assert!(scope.covers_domain(&EffectDomain::Finance));
        assert!(scope.covers_target("anything"));
        assert!(scope.covers_operation("anything"));
    }
}
