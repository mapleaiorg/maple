//! # MAPLE Resonance Runtime
//!
//! The foundational AI framework for Mapleverse, Finalverse, and iBank.
//!
//! ## Overview
//!
//! MAPLE (Multi-Agent Platform for Learning and Evolution) is a world-class
//! multi-agent AI framework built entirely on **Resonance Architecture** principles.
//!
//! Unlike traditional agent frameworks (Google A2A, Anthropic MCP) that treat
//! agents as isolated processes communicating via messages, MAPLE treats every
//! entity as a **Resonator** participating in continuous, stateful **resonance**.
//!
//! ## Key Features
//!
//! - **Resonance-Native**: Built from the ground up on presence → coupling → meaning → intent → commitment → consequence
//! - **8 Architectural Invariants**: Compile-time and runtime enforced safety guarantees
//! - **Attention Economics**: Prevents runaway resource consumption and coercive patterns
//! - **Commitment Accountability**: Every consequential action is attributable and auditable
//! - **Human Agency Protection**: Architectural, not policy-based, safeguards
//!
//! ## Architecture
//!
//! ```text
//! Traditional: Agent A --[message]--> Agent B --[message]--> Agent C
//!
//! MAPLE:      Resonator A <==[coupling]==> Resonator B <==[coupling]==> Resonator C
//!                 ↑                            ↑                            ↑
//!            [presence]                  [presence]                   [presence]
//!                 ↓                            ↓                            ↓
//!            [meaning] -----------------> [meaning] -----------------> [meaning]
//!                 ↓                            ↓                            ↓
//!            [intent] ------------------> [commitment] --------------> [consequence]
//! ```
//!
//! ## Quick Start
//!
//! ```no_run
//! use maple_runtime::{MapleRuntime, config::RuntimeConfig, ResonatorSpec};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Bootstrap runtime
//!     let config = RuntimeConfig::default();
//!     let runtime = MapleRuntime::bootstrap(config).await?;
//!
//!     // Register a Resonator
//!     let spec = ResonatorSpec::default();
//!     let resonator = runtime.register_resonator(spec).await?;
//!
//!     // Resonator is now active and can participate in resonance
//!
//!     // Shutdown gracefully
//!     runtime.shutdown().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Platform-Specific Configurations
//!
//! ### Mapleverse (Pure AI Agents)
//!
//! ```no_run
//! use maple_runtime::{MapleRuntime, config::mapleverse_runtime_config};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = mapleverse_runtime_config();
//! let runtime = MapleRuntime::bootstrap(config).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Finalverse (Human-AI Coexistence)
//!
//! ```no_run
//! use maple_runtime::{MapleRuntime, config::finalverse_runtime_config};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = finalverse_runtime_config();
//! let runtime = MapleRuntime::bootstrap(config).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### iBank (Autonomous Finance)
//!
//! ```no_run
//! use maple_runtime::{MapleRuntime, config::ibank_runtime_config};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ibank_runtime_config();
//! let runtime = MapleRuntime::bootstrap(config).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## The 8 Canonical Invariants
//!
//! These invariants are **enforced at runtime** and violations are treated as system errors:
//!
//! 1. **Presence precedes meaning**: A Resonator must be present before it can form or receive meaning
//! 2. **Meaning precedes intent**: Intent cannot be formed without sufficient meaning
//! 3. **Intent precedes commitment**: Commitments cannot be created without stabilized intent
//! 4. **Commitment precedes consequence**: No consequence may occur without an explicit commitment
//! 5. **Coupling is bounded by attention**: Coupling strength cannot exceed available attention
//! 6. **Safety overrides optimization**: Safety constraints take precedence over performance
//! 7. **Human agency cannot be bypassed**: Human Resonators must always be able to disengage
//! 8. **Failure must be explicit**: All failures must be surfaced, never hidden

#![cfg_attr(feature = "strict-docs", warn(missing_docs))]
#![cfg_attr(not(feature = "strict-docs"), allow(missing_docs))]
#![warn(clippy::all)]

// Core modules
pub mod types;
pub mod runtime_core;
pub mod fabrics;
pub mod allocator;
pub mod invariants;
pub mod temporal;
pub mod scheduler;
pub mod config;
pub mod telemetry;

// Re-exports for convenience
pub use runtime_core::{
    MapleRuntime, ResonatorHandle, CouplingHandle, ScheduleHandle,
    ResonatorSpec, ResonatorIdentitySpec, ContinuityProof,
};

pub use types::{
    ResonatorId, ResonatorProfile, PresenceState, PresenceConfig,
    Coupling, CouplingParams, CouplingScope, CouplingPersistence, SymmetryType,
    AttentionBudget, AttentionBudgetSpec, AttentionClass,
    Commitment, CommitmentContent, CommitmentStatus,
    TemporalAnchor, LocalTimestamp,
};

pub use invariants::{ArchitecturalInvariant, InvariantViolation};

// Error types
pub use types::{
    BootstrapError, ShutdownError, RegistrationError, ResumeError,
    PresenceError, CouplingError, AttentionError, CommitmentError,
    SchedulingError, TemporalError,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_runtime_bootstrap() {
        let config = config::RuntimeConfig::default();
        let runtime = MapleRuntime::bootstrap(config).await;
        assert!(runtime.is_ok());
    }

    #[tokio::test]
    async fn test_resonator_registration() {
        let config = config::RuntimeConfig::default();
        let runtime = MapleRuntime::bootstrap(config).await.unwrap();

        let spec = ResonatorSpec::default();
        let result = runtime.register_resonator(spec).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mapleverse_config() {
        let config = config::mapleverse_runtime_config();
        let runtime = MapleRuntime::bootstrap(config).await;
        assert!(runtime.is_ok());
    }

    #[tokio::test]
    async fn test_finalverse_config() {
        let config = config::finalverse_runtime_config();
        let runtime = MapleRuntime::bootstrap(config).await;
        assert!(runtime.is_ok());
    }

    #[tokio::test]
    async fn test_ibank_config() {
        let config = config::ibank_runtime_config();
        let runtime = MapleRuntime::bootstrap(config).await;
        assert!(runtime.is_ok());
    }

    #[tokio::test]
    async fn test_presence_signaling() {
        let config = config::RuntimeConfig::default();
        let runtime = MapleRuntime::bootstrap(config).await.unwrap();

        let spec = ResonatorSpec::default();
        let resonator = runtime.register_resonator(spec).await.unwrap();

        // Wait a bit to avoid rate limiting from initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;

        let presence = PresenceState::new();
        let result = resonator.signal_presence(presence).await;
        assert!(result.is_ok(), "Presence signaling failed: {:?}", result);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let config = config::RuntimeConfig::default();
        let runtime = MapleRuntime::bootstrap(config).await.unwrap();

        let result = runtime.shutdown().await;
        assert!(result.is_ok());
    }
}
