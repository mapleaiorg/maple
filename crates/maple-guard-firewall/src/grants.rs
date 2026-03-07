//! Capability grant model for the MAPLE firewall.
//!
//! A [`CapabilityGrant`] gives an agent permission to invoke a specific tool
//! with specific scope, rate limits, and conditions.

use serde::{Deserialize, Serialize};

/// A capability grant gives an agent permission to invoke a specific tool
/// with specific scope, rate limits, and conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGrant {
    /// Grant identifier.
    pub id: String,
    /// The agent (worldline) this grant is for.
    pub grantee: String,
    /// Tool being granted (e.g., `"zendesk.ticket.read"`). Supports glob patterns.
    pub tool: String,
    /// Scope restrictions.
    pub scope: GrantScope,
    /// Is human approval required for each invocation?
    pub requires_approval: bool,
    /// Maximum invocations per time window.
    pub rate_limit: Option<RateLimit>,
    /// Temporal validity start.
    pub valid_from: chrono::DateTime<chrono::Utc>,
    /// Temporal validity end (if `None`, grant does not expire).
    pub valid_until: Option<chrono::DateTime<chrono::Utc>>,
    /// Conditions that must hold for grant to be active.
    pub conditions: Vec<GrantCondition>,
    /// Who issued this grant.
    pub issuer: String,
    /// Purpose / justification.
    pub purpose: String,
}

/// Scope restrictions within a capability grant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantScope {
    /// Allowed operations within the tool (read, write, delete, etc.).
    pub operations: Vec<String>,
    /// Resource scope patterns (e.g., `"tickets/team-a/*"`).
    pub resources: Vec<String>,
    /// Maximum value per operation (for financial tools).
    pub max_value: Option<f64>,
    /// Idempotency required?
    pub require_idempotency: bool,
}

/// Rate limit configuration for a capability grant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum invocations per minute.
    pub max_per_minute: Option<u32>,
    /// Maximum invocations per hour.
    pub max_per_hour: Option<u32>,
    /// Maximum invocations per day.
    pub max_per_day: Option<u32>,
}

/// Conditions that must be met for a grant to be active.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GrantCondition {
    /// Data classification must match one of the allowed values.
    DataClassification {
        /// Allowed classification levels.
        allowed: Vec<String>,
    },
    /// Request must be within business hours.
    BusinessHours {
        /// IANA timezone name.
        timezone: String,
        /// Start hour (0-23).
        start_hour: u8,
        /// End hour (0-23).
        end_hour: u8,
    },
    /// Requires MFA / step-up authentication.
    RequireMfa,
    /// Only from specific IP ranges (for API access).
    IpRange {
        /// CIDR ranges.
        cidrs: Vec<String>,
    },
}

impl GrantScope {
    /// Create a permissive scope with given operations and no resource constraints.
    pub fn with_operations(operations: Vec<String>) -> Self {
        Self {
            operations,
            resources: Vec::new(),
            max_value: None,
            require_idempotency: false,
        }
    }
}
