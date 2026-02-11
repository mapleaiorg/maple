//! MRP Envelope System — Constitutional Routing Protocol.
//!
//! Per Whitepaper §4.1: "MRP is a constitutional protocol. Where traditional
//! protocols optimize for throughput, MRP optimizes for preserving the
//! Commitment Boundary under all conditions."
//!
//! ## Constitutional Invariants
//!
//! - **I.MRP-1 (Non-Escalation)**: No envelope may be transformed into a
//!   higher-resonance type than the one it declares. MEANING cannot become
//!   INTENT implicitly.
//! - **I.8 (Substrate Independence)**: Semantics independent of transport.
//!
//! ## Routing Rules
//!
//! - **MEANING**: Freely routable within cognition, NEVER reaches execution
//! - **INTENT**: Routable for negotiation, NON-EXECUTABLE
//! - **COMMITMENT**: MUST route through Commitment Gate, immutable once declared
//! - **CONSEQUENCE**: Emitted ONLY by execution layer, never by agents

pub mod builder;
pub mod envelope;
pub mod error;
pub mod payloads;
pub mod router;
pub mod routing;

pub use builder::{
    CommitmentEnvelopeBuilder, ConsequenceEnvelopeBuilder, IntentEnvelopeBuilder,
    MeaningEnvelopeBuilder,
};
pub use envelope::{EnvelopeHeader, IntegrityBlock, MrpEnvelope, RoutingConstraints, TypedPayload};
pub use error::MrpError;
pub use payloads::{CommitmentPayload, ConsequencePayload, IntentPayload, MeaningPayload};
pub use router::{ExecutionLayer, MockExecutionLayer, MrpRouter};
pub use routing::{EscalationRecord, RejectionReason, RouteDecision};
