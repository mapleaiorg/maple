//! Commitment Gate — 7-Stage Pipeline enforcing the Commitment Boundary.
//!
//! Per Whitepaper §2.10: "The Commitment Boundary is the hard architectural boundary
//! between cognition and action. No data, message, or control flow may cross this
//! boundary unless it is explicitly typed as a Commitment and approved by governance."
//!
//! ## Constitutional Invariants
//!
//! - **I.3 (Commitment Boundary)**: Only explicit commitments cross into execution.
//!   Intent does NOT imply action.
//! - **I.5 (Pre-Execution Accountability)**: Accountability established BEFORE
//!   execution begins. Post-hoc attribution is forbidden.
//! - **I.CG-1 (Decision Immutability)**: PolicyDecisionCards are immutable once recorded.
//! - **I.AAS-3 (Ledger Immutability)**: Commitment ledger is append-only.
//! - **Fail-Closed Pipeline Configuration**: submissions are rejected unless all
//!   seven stages are present in canonical order.
//! - **Explicit Lifecycle Transitions**: outcomes are recorded only from executable
//!   states; terminal states cannot be mutated.
//!
//! ## 7-Stage Pipeline
//!
//! 1. **Declaration** — Structural validation
//! 2. **Identity Binding** — WorldlineId + ContinuityChain verification
//! 3. **Capability Check** — Declared capabilities sufficient for scope
//! 4. **Policy Evaluation** — Governance policies applied
//! 5. **Risk Assessment** — Risk thresholds checked
//! 6. **Co-signature Collection** — Multi-party approval (if required)
//! 7. **Final Decision** — PolicyDecisionCard emitted and recorded

pub mod context;
pub mod declaration;
pub mod error;
pub mod gate;
pub mod ledger;
pub mod mocks;
pub mod stages;
pub mod traits;

pub use context::{CoSignature, GateContext, StageResult};
pub use declaration::{CommitmentDeclaration, CommitmentDeclarationBuilder};
pub use error::{GateError, LedgerError};
pub use gate::{AdjudicationResult, CommitmentGate, CommitmentOutcome, GateConfig};
pub use ledger::{CommitmentLedger, LedgerEntry, LedgerFilter, LifecycleEvent};
pub use mocks::{MockCapabilityProvider, MockPolicyProvider};
pub use stages::risk::RiskConfig;
pub use stages::{
    CapabilityCheckStage, CoSignatureStage, DeclarationStage, FinalDecisionStage,
    IdentityBindingStage, PolicyEvaluationStage, RiskAssessmentStage,
};
pub use traits::{CapabilityProvider, GateStage, PolicyProvider};
