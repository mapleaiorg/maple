//! PALM Deployment Engine
//!
//! Orchestrates fleet-level deployments using pluggable strategies.
//! Delegates to instance registries for actual Resonator lifecycle operations.
//!
//! ## Architectural Boundaries
//!
//! - `resonator-runtime` owns: single Resonator creation, state, execution
//! - `palm-deployment` owns: fleet rollout strategies, instance coordination, traffic shifting
//! - `palm-state` owns: checkpoints, continuity (called BY deployment engine)
//! - `palm-health` owns: health probes (called BY deployment engine)
//!
//! ## Key Principle
//!
//! Deployment operations MUST call through interfaces, not embed logic.
//! This crate coordinates the deployment process but doesn't implement
//! the actual Resonator lifecycle - that's the responsibility of the runtime.
//!
//! ## Usage
//!
//! ```no_run
//! use palm_deployment::{DeploymentManager, AllowAllPolicyGate};
//! use palm_registry::{InMemoryAgentRegistry, InMemoryInstanceRegistry};
//! use palm_deployment::state::InMemoryDeploymentStateStore;
//! use palm_deployment::routing::DiscoveryRoutingManager;
//! use palm_types::{DeploymentStrategy, ReplicaConfig, PolicyContext};
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let agent_registry = Arc::new(InMemoryAgentRegistry::new());
//! let instance_registry = Arc::new(InMemoryInstanceRegistry::new());
//! let state_store = Arc::new(InMemoryDeploymentStateStore::new());
//! let routing_manager = Arc::new(DiscoveryRoutingManager::new());
//! let policy_gate = Arc::new(AllowAllPolicyGate);
//!
//! let manager = DeploymentManager::new(
//!     agent_registry.clone(),
//!     instance_registry,
//!     state_store,
//!     routing_manager,
//!     policy_gate,
//! );
//!
//! // Create a deployment
//! // let deployment = manager.create_deployment(
//! //     &spec_id,
//! //     DeploymentStrategy::default(),
//! //     ReplicaConfig::new(3),
//! //     &PolicyContext::default(),
//! // ).await?;
//! # Ok(())
//! # }
//! ```

#![deny(unsafe_code)]
#![cfg_attr(feature = "strict-docs", warn(missing_docs))]
#![cfg_attr(not(feature = "strict-docs"), allow(missing_docs))]

pub mod manager;
pub mod context;
pub mod scheduler;
pub mod strategies;
pub mod routing;
pub mod state;
pub mod error;

// Re-exports
pub use manager::{AllowAllPolicyGate, DeleteConfig, DeploymentManager, PolicyGate};
pub use context::DeploymentContext;
pub use scheduler::{DeploymentConfig, DeploymentScheduler, QueuedDeployment, UpdateConfig};
pub use strategies::{DeploymentExecutor, DeploymentResult};
pub use routing::DiscoveryRoutingManager;
pub use state::{DeploymentStateStore, InMemoryDeploymentStateStore, StateStoreError};
pub use error::{DeploymentError, Result};
