#![deny(unsafe_code)]
//! # maple-waf-wlir
//!
//! WLIR (WorldLine Intermediate Representation) Factory Module System.
//!
//! Provides S-expression parsing, operator definitions, module verification,
//! and provenance-tracked factory modules for the WAF pipeline.

pub mod error;
pub mod module;
pub mod operators;
pub mod parser;
pub mod types;
pub mod verifier;

pub use error::WlirError;
pub use module::WlirFactoryModule;
pub use operators::{OperatorBody, OperatorDefinition};
pub use parser::{parse_sexpr, SExpr};
pub use types::{AxiomaticConstraints, ProvenanceHeader};
pub use verifier::ModuleVerifier;
