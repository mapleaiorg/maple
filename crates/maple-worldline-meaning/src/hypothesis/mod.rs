//! Hypothesis generation for meaning formation.
//!
//! Multiple generators propose interpretations from different perspectives.
//! Each generator implements the `HypothesisGenerator` trait and produces
//! `Hypothesis` objects that feed into the evidence evaluator.

pub mod generators;
pub mod types;

pub use generators::{
    CodePathGenerator, ComponentIsolationGenerator, EnvironmentalChangeGenerator,
    HistoricalPatternGenerator, InteractionPatternGenerator, ResourcePressureGenerator,
};
pub use types::{Hypothesis, HypothesisGenerator, ObservationSummary, SubsystemSummaryView};
