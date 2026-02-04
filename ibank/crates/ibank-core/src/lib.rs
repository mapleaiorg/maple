//! iBank core implementation on top of MAPLE Runtime.
//!
//! This crate enforces financial accountability invariants with explicit stage gating,
//! deterministic risk policy checks, accountable wire messages, and append-only audit logs.

#![deny(unsafe_code)]

pub mod aggregation;
pub mod connectors;
pub mod error;
pub mod flow;
pub mod ledger;
pub mod policy;
pub mod protocol;
pub mod router;
pub mod runtime;
pub mod storage;
pub mod types;

pub use aggregation::{
    AggregationConnector, AggregationUser, AssetPair, BalanceRecord, Balances, ComplianceSignal,
    ConnectorCaps, FieldProvenance, Limits, NormalizedBalance, NormalizedComplianceStatus,
    NormalizedTransaction, QuoteRecord, Quotes, RouteCandidate, SnapshotProof, TimeRange,
    TransactionRecord, TxDirection, Txns, UnifiedLedgerAssembler, UnifiedLedgerView,
};
pub use connectors::{ConnectorRegistry, SettlementConnector};
pub use error::IBankError;
pub use flow::{ConsequenceStage, ConsequenceStageMachine};
pub use ledger::{AppendOnlyLedger, LedgerEntry, LedgerEntryKind};
pub use policy::{AutonomyMode, RiskDecision, RiskPolicyConfig, RiskPolicyEngine};
pub use protocol::OriginAuthority;
pub use router::IBankRouter;
pub use runtime::{IBankEngine, IBankEngineConfig, LatestUnifiedSnapshot};
pub use storage::{LedgerStorageConfig, PersistentLedger};
pub use types::{
    AccountableWireMessage, AuditWitness, CommitmentRecord, CommitmentReference, ConnectorReceipt,
    ConsequenceRecord, ExecutionMode, HandleRequest, HandleResponse, HandleStatus, HumanApproval,
    MeaningField, MeaningRecord, RiskReport, RouteResult, TransferIntent, TransferPayload,
};
