//! # PALM Control Plane
//!
//! Unified facade over all PALM subsystems with integrated
//! policy enforcement and event aggregation.
//!
//! ## Overview
//!
//! The `PalmControlPlane` is the single entry point for all PALM operations.
//! It composes all PALM modules (registry, deployment, health, state) behind
//! a unified API that:
//!
//! - Enforces policy gates before operations
//! - Emits events for monitoring and audit
//! - Coordinates between subsystems
//!
//! ## Key Components
//!
//! - [`PalmControlPlane`]: Main facade for all operations
//! - [`PalmControlPlaneBuilder`]: Builder for configuring the control plane
//! - [`RequestContext`]: Context for each request (actor, platform, etc.)
//! - [`ControlPlaneOperation`]: All operations that can be performed
//! - [`EventAggregator`]: Unified event stream from all subsystems
//!
//! ## Example
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use palm_control::{
//!     PalmControlPlane, PalmControlPlaneBuilder, RequestContext,
//! };
//! use palm_types::PlatformProfile;
//! use palm_deployment::{AllowAllPolicyGate, InMemoryDeploymentStateStore};
//! use palm_registry::{InMemoryAgentRegistry, InMemoryInstanceRegistry};
//! use palm_state::{
//!     checkpoint::MockRuntimeStateGatherer,
//!     commitment_reconcile::MockAasClient,
//!     coupling_restore::MockCouplingRuntime,
//!     migration::MockMigrationRuntime,
//!     restore::{MockContinuityVerifier, MockRuntimeStateRestorer},
//!     storage::InMemoryStateStorage,
//! };
//!
//! # async fn example() {
//! // Build control plane with all dependencies
//! let control_plane = PalmControlPlaneBuilder::new(PlatformProfile::Development)
//!     .with_agent_registry(Arc::new(InMemoryAgentRegistry::new()))
//!     .with_instance_registry(Arc::new(InMemoryInstanceRegistry::new()))
//!     .with_policy_gate(Arc::new(AllowAllPolicyGate))
//!     .with_state_storage(Arc::new(InMemoryStateStorage::new()))
//!     .with_deployment_state_store(Arc::new(InMemoryDeploymentStateStore::new()))
//!     .with_state_gatherer(Arc::new(MockRuntimeStateGatherer::new()))
//!     .with_state_restorer(Arc::new(MockRuntimeStateRestorer::new()))
//!     .with_continuity_verifier(Arc::new(MockContinuityVerifier::new()))
//!     .with_coupling_runtime(Arc::new(MockCouplingRuntime::all_present()))
//!     .with_migration_runtime(Arc::new(MockMigrationRuntime::new("node-1")))
//!     .with_aas_client(Arc::new(MockAasClient::all_pending()))
//!     .build()
//!     .expect("Failed to build control plane");
//!
//! // Register an agent spec
//! let ctx = RequestContext::default();
//! let spec = palm_types::AgentSpec::new("my-agent", semver::Version::new(1, 0, 0));
//! let spec_id = control_plane.register_spec(spec, &ctx).await.unwrap();
//!
//! // Create a deployment
//! let config = palm_control::CreateDeploymentConfig::with_replicas(3);
//! let deployment = control_plane.create_deployment(&spec_id, config, &ctx).await.unwrap();
//!
//! println!("Created deployment: {}", deployment.id);
//! # }
//! ```
//!
//! ## Policy Enforcement
//!
//! All operations go through policy checks:
//!
//! 1. Human approval requirements (platform-specific)
//! 2. Policy gate evaluation (via `PolicyGate` trait)
//! 3. Audit event emission
//!
//! ## Event Aggregation
//!
//! The control plane aggregates events from all subsystems:
//!
//! - Deployment events (created, completed, failed, etc.)
//! - Instance events (started, terminated, health changes)
//! - State events (checkpoints, restores, migrations)
//! - Health events (probes, recovery actions)
//!
//! Subscribe to the unified stream:
//!
//! ```rust,no_run
//! # use palm_control::PalmControlPlane;
//! # fn example(cp: &PalmControlPlane) {
//! let mut events = cp.subscribe_events();
//! // Use events in a loop
//! # }
//! ```

#![deny(unsafe_code)]
#![cfg_attr(feature = "strict-docs", warn(missing_docs))]
#![cfg_attr(not(feature = "strict-docs"), allow(missing_docs))]

pub mod builder;
pub mod context;
pub mod control_plane;
pub mod error;
pub mod events;
pub mod operations;

// Re-exports
pub use builder::PalmControlPlaneBuilder;
pub use context::{Actor, RequestContext};
pub use control_plane::{CreateDeploymentConfig, PalmControlPlane};
pub use error::{ControlPlaneError, Result};
pub use events::{EventAggregator, EventForwarderBuilder};
pub use operations::{ControlPlaneOperation, NodeId, OperationCategory};
