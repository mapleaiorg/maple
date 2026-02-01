//! Tamper-evident audit logging for PALM
//!
//! Provides secure, immutable audit trails with cryptographic integrity verification.

pub mod entry;
pub mod integrity;
pub mod query;
pub mod sink;

pub use entry::{AuditEntry, AuditEntryBuilder, AuditAction, AuditOutcome};
pub use integrity::{IntegrityChain, IntegrityVerifier};
pub use query::{AuditQuery, AuditQueryBuilder};
pub use sink::{AuditSink, FileAuditSink, MemoryAuditSink};
