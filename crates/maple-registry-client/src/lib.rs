//! MAPLE Registry Client — OCI Distribution Spec client for push/pull/mirror operations.
//!
//! This crate provides:
//!
//! - **auth**: Registry authentication with multi-source credential resolution
//!   (environment variables, MAPLE credential file, Docker config fallback).
//! - **client**: Full OCI Distribution Spec client supporting blob push/pull,
//!   manifest push/pull, tag listing, and high-level package push/pull.

pub mod auth;
pub mod client;

// Re-export primary types for convenience.
pub use auth::{
    AuthError, CredentialEntry, CredentialStore, DockerAuthEntry, DockerConfig, RegistryAuth,
    apply_auth, decode_docker_auth,
};
pub use client::{
    DEFAULT_REGISTRY, OciReference, OciRegistryClient, RegistryError, humanize_bytes, sha2_hex,
};
