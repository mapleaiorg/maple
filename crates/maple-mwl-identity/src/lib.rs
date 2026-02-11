//! WorldLine identity model with continuity chains.
//!
//! Implements persistent identity without accounts or sessions (I.1: Worldline Primacy).
//! Identity persists across restarts, migrations, and key rotations through continuity chains.

pub mod continuity;
pub mod error;
pub mod manager;
pub mod bridge;

pub use continuity::{ContinuityChain, ContinuityContext, ContinuitySegment, KeyRef};
pub use error::{ContinuityError, IdentityError};
pub use manager::{IdentityManager, IdentityRecord};
pub use bridge::{resonator_id_to_worldline, ResonatorIdentity};
