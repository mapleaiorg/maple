//! Capability types: what actions are possible within a Collective
//!
//! Capabilities define the space of possible actions. They are granted
//! to Resonators through Roles and require Permits for execution.

use crate::CollectiveId;
use chrono::{DateTime, Utc};
use rcf_types::ScopeConstraint;
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};

/// Unique identifier for a Capability
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub String);

impl CapabilityId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A Capability defines what actions are possible
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capability {
    /// Unique capability identifier
    pub id: CapabilityId,
    /// Human-readable name
    pub name: String,
    /// Description of the capability
    pub description: String,
    /// The type of action this capability enables
    pub action_type: ActionType,
    /// Permit templates required for this capability
    pub required_permits: Vec<PermitTemplate>,
    /// What receipts must be produced when exercising this capability
    pub receipt_requirements: Vec<ReceiptRequirement>,
}

impl Capability {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        action_type: ActionType,
    ) -> Self {
        Self {
            id: CapabilityId::generate(),
            name: name.into(),
            description: description.into(),
            action_type,
            required_permits: Vec::new(),
            receipt_requirements: Vec::new(),
        }
    }

    pub fn with_id(mut self, id: CapabilityId) -> Self {
        self.id = id;
        self
    }

    pub fn with_permit_template(mut self, template: PermitTemplate) -> Self {
        self.required_permits.push(template);
        self
    }

    pub fn with_receipt_requirement(mut self, req: ReceiptRequirement) -> Self {
        self.receipt_requirements.push(req);
        self
    }
}

/// Types of actions a Capability can enable
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    /// Execute a commitment
    Execute,
    /// Read data or state
    Read,
    /// Write or modify state
    Write,
    /// Approve actions by others
    Approve,
    /// Delegate authority to others
    Delegate,
    /// Audit operations
    Audit,
    /// Custom action type
    Custom(String),
}

/// A template for permits required by a capability
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PermitTemplate {
    /// The capability this template is for
    pub capability_id: CapabilityId,
    /// Default scope for permits issued from this template
    pub scope_template: ScopeConstraint,
    /// Default limits for permits issued from this template
    pub default_limits: PermitLimits,
}

impl PermitTemplate {
    pub fn new(capability_id: CapabilityId) -> Self {
        Self {
            capability_id,
            scope_template: ScopeConstraint::default(),
            default_limits: PermitLimits::default(),
        }
    }

    pub fn with_scope(mut self, scope: ScopeConstraint) -> Self {
        self.scope_template = scope;
        self
    }

    pub fn with_limits(mut self, limits: PermitLimits) -> Self {
        self.default_limits = limits;
        self
    }
}

/// Limits on permit usage (shared by PermitTemplate and Permit)
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PermitLimits {
    /// Maximum number of times the permit can be used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<u64>,
    /// Maximum financial value per use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<u64>,
    /// Maximum concurrent active uses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,
    /// Rate limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimit>,
}

impl PermitLimits {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_uses(mut self, max: u64) -> Self {
        self.max_uses = Some(max);
        self
    }

    pub fn with_max_value(mut self, max: u64) -> Self {
        self.max_value = Some(max);
        self
    }

    pub fn with_max_concurrent(mut self, max: u32) -> Self {
        self.max_concurrent = Some(max);
        self
    }

    pub fn with_rate_limit(mut self, rate_limit: RateLimit) -> Self {
        self.rate_limit = Some(rate_limit);
        self
    }

    /// Check if any limits are set
    pub fn is_unlimited(&self) -> bool {
        self.max_uses.is_none()
            && self.max_value.is_none()
            && self.max_concurrent.is_none()
            && self.rate_limit.is_none()
    }
}

/// Rate limiting configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum operations per period
    pub max_per_period: u64,
    /// Period duration in seconds
    pub period_secs: u64,
}

impl RateLimit {
    pub fn new(max_per_period: u64, period_secs: u64) -> Self {
        Self {
            max_per_period,
            period_secs,
        }
    }

    /// Per-minute rate limit
    pub fn per_minute(max: u64) -> Self {
        Self::new(max, 60)
    }

    /// Per-hour rate limit
    pub fn per_hour(max: u64) -> Self {
        Self::new(max, 3600)
    }
}

/// What receipts must be produced when exercising a capability
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptRequirement {
    /// The type of receipt required
    pub receipt_type: ReceiptType,
    /// Whether this receipt is mandatory (vs. best-effort)
    pub mandatory: bool,
}

impl ReceiptRequirement {
    pub fn mandatory(receipt_type: ReceiptType) -> Self {
        Self {
            receipt_type,
            mandatory: true,
        }
    }

    pub fn optional(receipt_type: ReceiptType) -> Self {
        Self {
            receipt_type,
            mandatory: false,
        }
    }
}

/// Types of receipts that can be emitted
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReceiptType {
    /// A commitment was fulfilled
    CommitmentFulfilled,
    /// A commitment was broken
    CommitmentBroken,
    /// A workflow step was completed
    WorkflowStep,
    /// An audit action was performed
    Audit,
    /// A financial transaction occurred
    Financial,
    /// A dispute was resolved
    DisputeResolution,
    /// Collaboration between entities completed
    CollaborationCompleted,
    /// A dispute was won
    DisputeWon,
    /// A dispute was lost
    DisputeLost,
    /// Custom receipt type
    Custom(String),
}

/// Who granted a capability or permit
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrantAuthority {
    /// Granted by a Collective
    Collective(CollectiveId),
    /// Granted by an individual Resonator
    Individual(ResonatorId),
}

/// A grant of a capability to a specific resonator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityGrant {
    /// The capability being granted
    pub capability_id: CapabilityId,
    /// Who receives the capability
    pub grantee: ResonatorId,
    /// Who authorized the grant
    pub granted_by: GrantAuthority,
    /// When granted
    pub granted_at: DateTime<Utc>,
    /// Optional expiry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl CapabilityGrant {
    pub fn new(
        capability_id: CapabilityId,
        grantee: ResonatorId,
        granted_by: GrantAuthority,
    ) -> Self {
        Self {
            capability_id,
            grantee,
            granted_by,
            granted_at: Utc::now(),
            expires_at: None,
        }
    }

    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn is_active(&self) -> bool {
        match self.expires_at {
            Some(expiry) => Utc::now() < expiry,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_id() {
        let id = CapabilityId::generate();
        assert!(!id.0.is_empty());
        let named = CapabilityId::new("read-data");
        assert_eq!(format!("{}", named), "read-data");
    }

    #[test]
    fn test_capability_builder() {
        let cap = Capability::new("Read Orders", "Read order data", ActionType::Read)
            .with_id(CapabilityId::new("read-orders"))
            .with_receipt_requirement(ReceiptRequirement::mandatory(ReceiptType::Audit));

        assert_eq!(cap.name, "Read Orders");
        assert_eq!(cap.action_type, ActionType::Read);
        assert_eq!(cap.receipt_requirements.len(), 1);
        assert!(cap.receipt_requirements[0].mandatory);
    }

    #[test]
    fn test_permit_limits() {
        let unlimited = PermitLimits::default();
        assert!(unlimited.is_unlimited());

        let limited = PermitLimits::new()
            .with_max_uses(100)
            .with_max_value(50_000)
            .with_rate_limit(RateLimit::per_hour(60));
        assert!(!limited.is_unlimited());
        assert_eq!(limited.max_uses, Some(100));
    }

    #[test]
    fn test_grant_authority() {
        let collective = GrantAuthority::Collective(CollectiveId::new("coll-1"));
        let individual = GrantAuthority::Individual(ResonatorId::new("res-1"));
        assert_ne!(collective, individual);
    }

    #[test]
    fn test_receipt_types() {
        let fulfilled = ReceiptType::CommitmentFulfilled;
        let custom = ReceiptType::Custom("my-receipt".into());
        assert_ne!(fulfilled, custom);
    }
}
