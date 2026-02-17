#![deny(unsafe_code)]
//! # maple-waf-evolution-engine
//!
//! The Σ (synthesis) engine — LLM-powered code synthesis and hypothesis evaluation
//! for the WorldLine Autopoietic Factory.
//!
//! Enforces **I.WAF-2: Synthesis Traceability** — every code delta traceable to intent.

pub mod error;
pub mod evaluator;
pub mod prompt_builder;
pub mod synthesizer;
pub mod types;

pub use error::EvolutionError;
pub use evaluator::HypothesisEvaluator;
pub use prompt_builder::SystemPromptBuilder;
pub use synthesizer::{FailingSynthesizer, SimulatedSynthesizer, Synthesizer};
pub use types::{HardwareContext, Hypothesis, SynthesisResult};
