use crate::model::{
    AgentCheckpoint, AuditAppend, AuditRecord, CommitmentRecord, ProjectionSnapshot, SemanticHit,
    SemanticRecord,
};
use crate::StorageResult;
use aas_types::{CommitmentOutcome, LifecycleStatus, PolicyDecisionCard};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rcf_commitment::{CommitmentId, RcfCommitment};

/// Generic query window for paged reads.
#[derive(Debug, Clone, Copy, Default)]
pub struct QueryWindow {
    pub limit: usize,
    pub offset: usize,
}

/// Storage interface for commitment lifecycle records.
#[async_trait]
pub trait CommitmentStore: Send + Sync {
    /// Insert a newly adjudicated commitment record.
    async fn create_commitment(
        &self,
        commitment: RcfCommitment,
        decision: PolicyDecisionCard,
        declared_at: DateTime<Utc>,
    ) -> StorageResult<()>;

    /// Transition lifecycle status from one state to another.
    async fn transition_lifecycle(
        &self,
        commitment_id: &CommitmentId,
        expected_from: LifecycleStatus,
        to: LifecycleStatus,
        updated_at: DateTime<Utc>,
    ) -> StorageResult<()>;

    /// Persist final outcome.
    async fn set_outcome(
        &self,
        commitment_id: &CommitmentId,
        outcome: CommitmentOutcome,
        final_status: LifecycleStatus,
    ) -> StorageResult<()>;

    /// Get one commitment record by id.
    async fn get_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> StorageResult<Option<CommitmentRecord>>;

    /// List records newest-first.
    async fn list_commitments(&self, window: QueryWindow) -> StorageResult<Vec<CommitmentRecord>>;
}

/// Storage interface for append-only audit events.
#[async_trait]
pub trait AuditStore: Send + Sync {
    /// Append an event and return the canonical, hash-linked stored record.
    async fn append_audit(&self, event: AuditAppend) -> StorageResult<AuditRecord>;

    /// Read events newest-first.
    async fn list_audit(&self, window: QueryWindow) -> StorageResult<Vec<AuditRecord>>;

    /// Get the latest audit hash anchor.
    async fn latest_audit_hash(&self) -> StorageResult<Option<String>>;
}

/// Storage interface for agent runtime checkpointing.
#[async_trait]
pub trait AgentStateStore: Send + Sync {
    async fn upsert_checkpoint(&self, checkpoint: AgentCheckpoint) -> StorageResult<()>;
    async fn get_checkpoint(&self, resonator_id: &str) -> StorageResult<Option<AgentCheckpoint>>;
    async fn list_checkpoints(&self, window: QueryWindow) -> StorageResult<Vec<AgentCheckpoint>>;
}

/// Storage interface for read-model/projection snapshots.
#[async_trait]
pub trait ProjectionStore: Send + Sync {
    async fn upsert_projection(&self, snapshot: ProjectionSnapshot) -> StorageResult<()>;
    async fn get_projection(
        &self,
        namespace: &str,
        key: &str,
    ) -> StorageResult<Option<ProjectionSnapshot>>;
    async fn list_projections(
        &self,
        namespace: &str,
        window: QueryWindow,
    ) -> StorageResult<Vec<ProjectionSnapshot>>;
}

/// Optional semantic memory store for AI-friendly retrieval.
#[async_trait]
pub trait SemanticMemoryStore: Send + Sync {
    async fn upsert_semantic(&self, record: SemanticRecord) -> StorageResult<()>;
    async fn search_semantic(
        &self,
        namespace: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> StorageResult<Vec<SemanticHit>>;
}

/// Unified storage bundle used by MAPLE runtime surfaces.
pub trait MapleStorage:
    CommitmentStore + AuditStore + AgentStateStore + ProjectionStore + SemanticMemoryStore + Send + Sync
{
}

impl<T> MapleStorage for T where
    T: CommitmentStore
        + AuditStore
        + AgentStateStore
        + ProjectionStore
        + SemanticMemoryStore
        + Send
        + Sync
{
}
