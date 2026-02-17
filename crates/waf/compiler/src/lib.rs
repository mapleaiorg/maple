#![deny(unsafe_code)]
//! # maple-waf-compiler
//!
//! Adaptive Compiler Integration for the WorldLine Autopoietic Factory.
//!
//! Provides hardware-aware compilation strategy selection, sandboxed
//! compilation backends, and content-addressed executable artifacts.

pub mod compiler;
pub mod error;
pub mod sandbox;
pub mod strategy;
pub mod types;

pub use compiler::WafCompiler;
pub use error::CompilerError;
pub use sandbox::{CompilationSandbox, SimulatedSandbox};
pub use strategy::CompilationStrategy;
pub use types::{CompilationConfig, CompilationTarget, ExecutableArtifact};
