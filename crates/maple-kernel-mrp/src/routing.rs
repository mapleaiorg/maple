use maple_mwl_types::WorldlineId;
use serde::{Deserialize, Serialize};

/// Route decision — what happens to an envelope after validation.
#[derive(Clone, Debug)]
pub enum RouteDecision {
    /// Deliver to cognition layer (for MEANING and INTENT)
    DeliverToCognition(Vec<WorldlineId>),
    /// Route through Commitment Gate (for COMMITMENT)
    RouteToGate,
    /// Deliver as consequence observation (for CONSEQUENCE)
    DeliverAsConsequence(WorldlineId),
    /// Reject the envelope (integrity, type mismatch, escalation, etc.)
    Reject(RejectionReason),
    /// Quarantine suspicious envelope
    Quarantine(String),
    /// Envelope has expired (TTL exceeded)
    Expired,
}

/// Reason for envelope rejection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RejectionReason {
    /// I.MRP-1 violation: attempted implicit type escalation
    EscalationViolation { from: String, to: String },
    /// Header/payload type mismatch
    TypeMismatch { declared: String, actual: String },
    /// Integrity verification failed (tampered envelope)
    IntegrityFailure,
    /// TTL expired
    Expired,
    /// Consequence not from execution layer
    InvalidConsequenceOrigin,
    /// Meaning/Intent attempted to reach execution
    CognitionOnlyViolation,
}

/// Record of an escalation violation — for accountability audit trail.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EscalationRecord {
    pub envelope_id: uuid::Uuid,
    pub origin: WorldlineId,
    pub declared_type: String,
    pub attempted_type: String,
    pub timestamp: maple_mwl_types::TemporalAnchor,
}
