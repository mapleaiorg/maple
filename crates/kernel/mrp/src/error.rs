use maple_mwl_types::ResonanceType;
use thiserror::Error;

/// Errors from the MRP Envelope System.
#[derive(Error, Debug)]
pub enum MrpError {
    #[error("escalation violation: attempted {from:?} → {to:?}")]
    EscalationViolation {
        from: ResonanceType,
        to: ResonanceType,
    },

    #[error("type mismatch: header declares {declared:?} but payload is {actual:?}")]
    TypeMismatch {
        declared: ResonanceType,
        actual: ResonanceType,
    },

    #[error("integrity verification failed: {0}")]
    IntegrityFailure(String),

    #[error("envelope expired (TTL exceeded)")]
    Expired,

    #[error("routing error: {0}")]
    RoutingError(String),

    #[error("consequence not from execution layer: {0}")]
    InvalidConsequenceOrigin(String),

    #[error("commitment envelope requires gate routing")]
    CommitmentRequiresGate,

    #[error("meaning/intent cannot reach execution layer")]
    CognitionOnlyType,

    #[error("builder error: {0}")]
    BuilderError(String),

    #[error("missing required field: {0}")]
    MissingField(String),

    #[error("gate error: {0}")]
    GateError(#[from] maple_kernel_gate::GateError),

    #[error("fabric error: {0}")]
    FabricError(#[from] maple_kernel_fabric::FabricError),
}

/// Specific escalation violation record — for accountability.
#[derive(Clone, Debug)]
pub struct EscalationViolation {
    pub from: ResonanceType,
    pub to: ResonanceType,
    pub message: String,
}

/// Integrity error — tampered or invalid envelope.
#[derive(Clone, Debug)]
pub struct IntegrityError {
    pub expected_hash: [u8; 32],
    pub actual_hash: [u8; 32],
    pub message: String,
}

/// Type mismatch error — header/payload disagreement.
#[derive(Clone, Debug)]
pub struct TypeMismatchError {
    pub declared: ResonanceType,
    pub actual: ResonanceType,
}
