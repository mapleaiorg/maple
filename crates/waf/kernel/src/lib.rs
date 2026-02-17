#![deny(unsafe_code)]
//! # maple-waf-kernel
//!
//! Autopoietic Kernel Orchestrator for the WorldLine Autopoietic Factory.
//!
//! The kernel runs the continuous self-evolution loop, coordinating
//! dissonance detection, synthesis, compilation, swap gating, and
//! governance into a unified autopoietic cycle.

pub mod error;
pub mod kernel;
pub mod metrics;

pub use error::KernelError;
pub use kernel::{AutopoieticKernel, EvolutionStepResult};
pub use metrics::KernelMetrics;
