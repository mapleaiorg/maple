//! MWL SDK — CLI commands, REST API routes, and Python bindings.
//!
//! This crate provides the external interface to the MWL kernel layer:
//!
//! - **CLI** (`--features cli`): Clap-based MWL subcommands for the maple CLI
//! - **API** (`--features api`): Axum router with REST endpoints for the PALM daemon
//! - **Python** (`--features python`): PyO3 bindings for the Python SDK
//!
//! By default, `cli` and `api` features are enabled. The `python` feature
//! requires PyO3 and is opt-in.
//!
//! ## CLI Integration
//!
//! ```ignore
//! use maple_kernel_sdk::cli::{MwlCommands, handle_mwl_command};
//!
//! // Add to your Commands enum:
//! Mwl { #[command(subcommand)] command: MwlCommands },
//!
//! // Dispatch:
//! Commands::Mwl { command } => handle_mwl_command(command, &endpoint, &client).await,
//! ```
//!
//! In this repository, `maple-cli` exposes these as direct groups:
//! `maple worldline|commit|provenance|financial|policy|kernel`.
//!
//! ## API Integration
//!
//! ```ignore
//! use maple_kernel_sdk::api::mwl_router;
//!
//! let api_routes = Router::new()
//!     .merge(mwl_router());
//! ```
//!
//! ## Python SDK
//!
//! ```python
//! from maple import MapleSdk, CommitmentBuilder
//!
//! sdk = MapleSdk.connect("http://localhost:8080")
//! builder = CommitmentBuilder("wl-123")
//! commitment = builder.scope("communication", ["wl-456"]).build()
//! ```

// ──────────────────────────────────────────────
// Feature-gated modules
// ──────────────────────────────────────────────

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "api")]
pub mod api;

pub mod python;

// ──────────────────────────────────────────────
// Re-exports for convenience
// ──────────────────────────────────────────────

// CLI re-exports
#[cfg(feature = "cli")]
pub use cli::{
    handle_mwl_command,
    // Response DTOs
    AuditEventResponse,
    AuditTrailResponse,
    BalanceResponse,
    CommitmentResponse,
    KernelMetricsResponse,
    KernelStatusResponse,
    MwlCommands,
    PolicyResponse,
    ProvenanceNodeResponse,
    ProvenanceResponse,
    WorldlineResponse,
};

// API re-exports
#[cfg(feature = "api")]
pub use api::mwl_router;

// Python re-exports (always available for docs, functional with python feature)
pub use python::{
    PyAuditEvent, PyBalanceProjection, PyCommitmentBuilder, PyCommitmentDeclaration,
    PyCommitmentResult, PyMapleSdk, PyWorldline,
};
