//! Workflow DSL for MAPLE v2
//!
//! A domain-specific language for defining commitment programs
//! declaratively. The DSL compiles down to `WorkflowDefinition`
//! from the `workflow-types` crate.
//!
//! # DSL Syntax
//!
//! ```text
//! WORKFLOW "Document Review" {
//!     VERSION "1.0"
//!     TIMEOUT 86400
//!
//!     ROLES {
//!         author: "Document author"
//!         reviewer: "Document reviewer"
//!         approver: "Final approver"
//!     }
//!
//!     NODE start TYPE start
//!     NODE submit TYPE action {
//!         ROLE author
//!         COMMITMENT "Submit document for review"
//!         TIMEOUT 3600
//!     }
//!     NODE review TYPE action {
//!         ROLE reviewer
//!         COMMITMENT "Review the submitted document"
//!         RECEIPT CommitmentFulfilled
//!         TIMEOUT 7200
//!         ESCALATION timeout_retry 3
//!     }
//!     NODE end TYPE end
//!
//!     EDGES {
//!         start -> submit
//!         submit -> review ON receipt CommitmentFulfilled
//!         review -> end ON receipt CommitmentFulfilled
//!     }
//!
//!     ESCALATION {
//!         ON timeout -> abort "Review timed out"
//!     }
//! }
//! ```
//!
//! # Usage
//!
//! ```rust
//! use workflow_dsl::compile;
//! use collective_types::CollectiveId;
//! use resonator_types::ResonatorId;
//!
//! let dsl = r#"
//! WORKFLOW "Simple" {
//!     NODE start TYPE start
//!     NODE task TYPE action {
//!         COMMITMENT "Do the thing"
//!     }
//!     NODE end TYPE end
//!     EDGES {
//!         start -> task
//!         task -> end
//!     }
//! }
//! "#;
//!
//! let def = compile(dsl, CollectiveId::new("test"), ResonatorId::new("author")).unwrap();
//! assert_eq!(def.name, "Simple");
//! assert_eq!(def.node_count(), 3);
//! ```

#![deny(unsafe_code)]

mod compiler;
mod errors;
mod lexer;
mod parser;
mod validator;

pub use compiler::compile;
pub use errors::{DslError, DslResult};
pub use lexer::{Lexer, Token, TokenKind};
pub use parser::{ParsedEdge, ParsedEscalation, ParsedNode, ParsedRole, ParsedWorkflow};
pub use validator::validate;
