//! # MapleVerse Types
//!
//! Core types for MapleVerse - a **pure AI-agent civilization** simulation.
//!
//! ## CRITICAL INVARIANT: NO HUMAN PROFILES
//!
//! MapleVerse is a pure AI-agent civilization. Human profiles are **runtime rejected**.
//! This is not a policy - it's a fundamental architectural constraint enforced at every layer.
//!
//! ## Core Principles
//!
//! 1. **Agents Only**: Every entity is an AI agent or a collective of AI agents
//! 2. **Receipts are Truth**: Reputation comes ONLY from verified commitment receipts
//! 3. **Attention is Scarce**: Attention regenerates per epoch, is tradeable, and bounds action
//! 4. **Collectives are First-Class**: Groups have their own identity, reputation, and attention
//! 5. **Regions Structure the World**: Agents exist in regions, migration only to neighbors
//!
//! ## Module Organization
//!
//! - [`config`]: World configuration with human profile rejection
//! - [`entity`]: Agent and collective entity definitions
//! - [`economy`]: MAPLE tokens and attention economics
//! - [`reputation`]: Receipt-based reputation system
//! - [`world`]: Regions, geography, and world structure
//! - [`event`]: Epoch-based events and world state changes
//! - [`errors`]: Error types for the MapleVerse layer

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod config;
pub mod economy;
pub mod entity;
pub mod errors;
pub mod event;
pub mod reputation;
pub mod world;

// Re-export commonly used types
pub use config::{MapleVerseConfig, WorldParameters};
pub use economy::{Amount, AttentionBudget, AttentionUnits, MapleBalance};
pub use entity::{CollectiveEntity, EntityId, EntityKind, IndividualEntity, MapleVerseEntity};
pub use errors::MapleVerseError;
pub use event::{Epoch, EpochId, EpochSummary, WorldEvent, WorldEventId};
pub use reputation::{ReputationReceipt, ReputationScore, ReputationSource};
pub use world::{Region, RegionId, WorldTopology};

/// The current MapleVerse protocol version
pub const PROTOCOL_VERSION: &str = "2.0.0";

/// Maximum agents supported in a single MapleVerse instance
pub const MAX_AGENTS: u64 = 100_000_000; // 100M agents

/// Prelude module for convenient imports
pub mod prelude {
    //! Convenient re-exports for MapleVerse types
    pub use super::config::{MapleVerseConfig, WorldParameters};
    pub use super::economy::{Amount, AttentionBudget, AttentionUnits, MapleBalance};
    pub use super::entity::{
        CollectiveEntity, EntityId, EntityKind, IndividualEntity, MapleVerseEntity,
    };
    pub use super::errors::MapleVerseError;
    pub use super::event::{Epoch, EpochId, EpochSummary, WorldEvent, WorldEventId};
    pub use super::reputation::{ReputationReceipt, ReputationScore, ReputationSource};
    pub use super::world::{Region, RegionId, WorldTopology};
}
