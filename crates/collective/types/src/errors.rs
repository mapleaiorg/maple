//! Error types for the Collective layer

use crate::{CapabilityId, CollectiveId, PermitId, RoleId};
use resonator_types::ResonatorId;

/// Errors that can occur in Collective operations
#[derive(Debug, thiserror::Error)]
pub enum CollectiveError {
    #[error("Member not found: {0}")]
    MemberNotFound(ResonatorId),

    #[error("Member already exists: {0}")]
    MemberAlreadyExists(ResonatorId),

    #[error("Member not active: {0}")]
    MemberNotActive(ResonatorId),

    #[error("Role not found: {0}")]
    RoleNotFound(RoleId),

    #[error("Role already exists: {0}")]
    RoleAlreadyExists(RoleId),

    #[error("Capability not found: {0}")]
    CapabilityNotFound(CapabilityId),

    #[error("Permit expired: {0}")]
    PermitExpired(PermitId),

    #[error("Permit not found: {0}")]
    PermitNotFound(PermitId),

    #[error("Insufficient budget: required {required}, available {available}")]
    InsufficientBudget { required: u64, available: u64 },

    #[error("Insufficient attention: required {required}, available {available}")]
    InsufficientAttention { required: u64, available: u64 },

    #[error("Threshold not met: required {required} signatures, have {current}")]
    ThresholdNotMet { required: u32, current: u32 },

    #[error("Invalid membership operation: {0}")]
    InvalidMembership(String),

    #[error("Treasury error: {0}")]
    TreasuryError(String),

    #[error("Policy violation: {0}")]
    PolicyViolation(String),

    #[error("Collective not active: {0}")]
    CollectiveNotActive(CollectiveId),

    #[error("Collective not found: {0}")]
    CollectiveNotFound(CollectiveId),

    #[error("No coupling slots available")]
    NoCouplingSlots,

    #[error("Workflow quota exceeded")]
    WorkflowQuotaExceeded,

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Escrow not found: {0}")]
    EscrowNotFound(String),
}

/// Result type alias for collective operations
pub type CollectiveResult<T> = Result<T, CollectiveError>;
