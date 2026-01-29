//! Core type definitions for MAPLE Resonance Runtime

mod ids;
mod profile;
mod attention;
mod coupling;
mod presence;
mod commitment;
mod temporal;
mod errors;

pub use ids::*;
pub use profile::*;
pub use attention::*;
pub use coupling::*;
pub use presence::*;
pub use commitment::*;
pub use temporal::*;

// Re-export errors individually to avoid conflicts
pub use errors::{
    BootstrapError, ShutdownError, RegistrationError, ResumeError,
    PresenceError, CouplingError, AttentionError, InvariantViolation,
    CommitmentError, ConsequenceError, SchedulingError, TemporalError,
};
pub use coupling::CouplingValidationError;
