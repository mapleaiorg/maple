//! Core type definitions for MAPLE Resonance Runtime

mod attention;
mod commitment;
mod coupling;
mod errors;
mod ids;
mod presence;
mod profile;
mod temporal;

pub use attention::*;
pub use commitment::*;
pub use coupling::*;
pub use ids::*;
pub use presence::*;
pub use profile::*;
pub use temporal::*;

// Re-export errors individually to avoid conflicts
pub use coupling::CouplingValidationError;
pub use errors::{
    AttentionError, BootstrapError, CommitmentError, ConsequenceError, CouplingError,
    InvariantViolation, PresenceError, RegistrationError, ResumeError, SchedulingError,
    ShutdownError, TemporalError,
};
