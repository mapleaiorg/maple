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
pub mod agent_kernel;
pub mod allocator;
pub mod cognition;
pub mod config;
pub mod fabrics;
pub mod invariants;
pub mod runtime_core;
pub mod scheduler;
pub mod telemetry;
pub mod temporal;
pub mod types;

// Re-exports for convenience
pub use agent_kernel::{
    AgentAuditEvent, AgentExecutionProfile, AgentHandleRequest, AgentHandleResponse, AgentHost,
    AgentKernel, AgentKernelError, AgentRegistration, CapabilityDescriptor, CapabilityExecution,
    CapabilityExecutor, CommitmentGateway, EchoCapability, SimulatedTransferCapability,
};
pub use cognition::{
    LlamaAdapter, ModelAdapter, ModelBackend, ModelRequest, ModelResponse, StructuredCognition,
    SuggestedTool, ValidationStatus, VendorAdapter,
};
pub use runtime_core::{
    ContinuityProof, CouplingHandle, MapleRuntime, ResonatorHandle, ResonatorIdentitySpec,
    ResonatorSpec, ScheduleHandle,
};

pub use types::{
    AttentionBudget, AttentionBudgetSpec, AttentionClass, Commitment, CommitmentContent,
    CommitmentStatus, Coupling, CouplingParams, CouplingPersistence, CouplingScope, LocalTimestamp,
    PresenceConfig, PresenceState, ResonatorId, ResonatorProfile, SymmetryType, TemporalAnchor,
};

pub use invariants::{ArchitecturalInvariant, InvariantViolation};

// Error types
pub use types::{
    AttentionError, BootstrapError, CommitmentError, CouplingError, PresenceError,
    RegistrationError, ResumeError, SchedulingError, ShutdownError, TemporalError,
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

        let presence = PresenceState::new();
        let result = resonator.signal_presence(presence).await;
        assert!(result.is_ok(), "Presence signaling failed: {:?}", result);

        // A second immediate signal should still be rate-limited.
        let result = resonator.signal_presence(PresenceState::new()).await;
        assert!(matches!(result, Err(PresenceError::RateLimitExceeded)));
    }

    #[tokio::test]
    async fn test_shutdown() {
        let config = config::RuntimeConfig::default();
        let runtime = MapleRuntime::bootstrap(config).await.unwrap();

        let result = runtime.shutdown().await;
        assert!(result.is_ok());
    }
}
