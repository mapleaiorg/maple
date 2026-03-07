//! MAPLE Model Manager core types and storage.
//!
//! This crate provides:
//! - **metadata**: Complete model metadata types (architecture, tokenizer,
//!   capabilities, license, inference defaults, format).
//! - **modelfile**: MapleModelfile YAML parsing with governance and
//!   benchmark constraints.
//! - **store**: Local on-disk model storage with versioned indexing
//!   and usage tracking.

pub mod metadata;
pub mod modelfile;
pub mod store;

// Re-export primary types for convenience.
pub use metadata::{
    ContextInfo, InferenceDefaults, ModelArchitecture, ModelCapability, ModelFormat,
    ModelLicense, ModelMetadata, PromptTemplate, QuantizationInfo, TokenizerInfo,
};
pub use modelfile::{
    DataClassificationRule, MapleModelfile, ModelBenchmarkConfig, ModelGovernance,
    ModelfileError,
};
pub use store::{
    ModelIndex, ModelIndexEntry, ModelListEntry, ModelStore, ModelStoreError,
    ModelVersionInfo,
};
