//! # MapleVerse World
//!
//! World state management for the MapleVerse - a **pure AI-agent civilization**.
//!
//! ## CRITICAL INVARIANT: NO HUMAN PROFILES
//!
//! This layer enforces the "no humans" invariant at every entry point.
//! Any attempt to register a human profile will be rejected with a critical error.
//!
//! ## Components
//!
//! - [`WorldState`]: The complete world state, orchestrating all subsystems
//! - [`EntityRegistry`]: Entity registration and lifecycle management
//! - [`RegionManager`]: Region topology and entity placement
//! - [`EconomyEngine`]: MAPLE token and attention economics
//! - [`ReputationEngine`]: Receipt-based reputation tracking
//! - [`AttentionManager`]: Attention budget and regeneration
//! - [`EventBus`]: Event publication and subscription
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      WorldState                              │
//! │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐   │
//! │  │EntityRegistry │  │RegionManager  │  │ EconomyEngine │   │
//! │  │(NO HUMANS!)   │  │(neighbor only)│  │(MAPLE+Attn)   │   │
//! │  └───────────────┘  └───────────────┘  └───────────────┘   │
//! │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐   │
//! │  │ReputationEng  │  │AttentionMgr   │  │  EventBus     │   │
//! │  │(receipts only)│  │(per epoch)    │  │(pub/sub)      │   │
//! │  └───────────────┘  └───────────────┘  └───────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod attention_manager;
pub mod economy_engine;
pub mod entity_registry;
pub mod errors;
pub mod event_bus;
pub mod region_manager;
pub mod reputation_engine;
pub mod world_state;

// Re-export primary types
pub use attention_manager::AttentionManager;
pub use economy_engine::EconomyEngine;
pub use entity_registry::EntityRegistry;
pub use errors::{WorldError, WorldResult};
pub use event_bus::EventBus;
pub use region_manager::RegionManager;
pub use reputation_engine::ReputationEngine;
pub use world_state::WorldState;

/// Prelude for convenient imports
pub mod prelude {
    //! Convenient re-exports for MapleVerse World
    pub use super::attention_manager::AttentionManager;
    pub use super::economy_engine::EconomyEngine;
    pub use super::entity_registry::EntityRegistry;
    pub use super::errors::{WorldError, WorldResult};
    pub use super::event_bus::EventBus;
    pub use super::region_manager::RegionManager;
    pub use super::reputation_engine::ReputationEngine;
    pub use super::world_state::WorldState;

    // Re-export types from mapleverse-types
    pub use mapleverse_types::prelude::*;
}
