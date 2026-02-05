//! PostgreSQL adapter for MAPLE storage.
//!
//! This adapter is designed as the transactional source-of-truth backend.
//! Semantic search is currently implemented with deterministic in-Rust cosine
//! scoring over stored embeddings. `pgvector` integration can be added later
//! without changing trait surfaces.

use crate::model::{
    AgentCheckpoint, AuditAppend, AuditRecord, CommitmentRecord, ProjectionSnapshot, SemanticHit,
    SemanticRecord,
};
use crate::traits::{
    AgentStateStore, AuditStore, CommitmentStore, ProjectionStore, QueryWindow, SemanticMemoryStore,
};
use crate::{StorageError, StorageResult};
use aas_types::{CommitmentOutcome, Decision, LifecycleStatus, PolicyDecisionCard};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rcf_commitment::{CommitmentId, RcfCommitment};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Acquire, Row};
use std::cmp::Ordering;
use uuid::Uuid;

/// PostgreSQL-backed storage adapter.
#[derive(Clone)]
pub struct PostgresMapleStorage {
    pool: PgPool,
}

impl PostgresMapleStorage {
    /// Connect to PostgreSQL and initialize required schema.
    pub async fn connect(database_url: &str) -> StorageResult<Self> {
        Self::connect_with_options(database_url, 10, 5).await
    }

    /// Connect with explicit pool parameters.
    pub async fn connect_with_options(
        database_url: &str,
        max_connections: u32,
        connect_timeout_secs: u64,
    ) -> StorageResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(std::time::Duration::from_secs(connect_timeout_secs))
            .connect(database_url)
            .await
            .map_err(|e| StorageError::Backend(format!("failed to connect postgres: {e}")))?;
        let store = Self { pool };
        store.init_schema().await?;
        Ok(store)
    }

    /// Create adapter from an existing pool.
    pub async fn from_pool(pool: PgPool) -> StorageResult<Self> {
        let store = Self { pool };
        store.init_schema().await?;
        Ok(store)
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    async fn init_schema(&self) -> StorageResult<()> {
        let ddl = [
            r#"
            CREATE TABLE IF NOT EXISTS maple_commitments (
                commitment_id TEXT PRIMARY KEY,
                commitment JSONB NOT NULL,
                decision JSONB NOT NULL,
                lifecycle_status TEXT NOT NULL,
                execution_started_at TIMESTAMPTZ,
                execution_completed_at TIMESTAMPTZ,
                outcome JSONB,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS maple_audit_events (
                event_id TEXT PRIMARY KEY,
                sequence BIGINT NOT NULL UNIQUE,
                timestamp TIMESTAMPTZ NOT NULL,
                actor TEXT NOT NULL,
                stage TEXT NOT NULL,
                success BOOLEAN NOT NULL,
                message TEXT NOT NULL,
                commitment_id TEXT,
                payload JSONB NOT NULL,
                previous_hash TEXT,
                hash TEXT NOT NULL
            )
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS maple_agent_checkpoints (
                resonator_id TEXT PRIMARY KEY,
                profile_name TEXT NOT NULL,
                state TEXT NOT NULL,
                active_commitments JSONB NOT NULL,
                last_audit_event_id TEXT,
                metadata JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS maple_projections (
                namespace TEXT NOT NULL,
                key TEXT NOT NULL,
                schema_version TEXT NOT NULL,
                data JSONB NOT NULL,
                snapshot_hash TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                PRIMARY KEY (namespace, key)
            )
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS maple_semantic_records (
                namespace TEXT NOT NULL,
                record_id TEXT NOT NULL,
                embedding JSONB NOT NULL,
                content TEXT NOT NULL,
                metadata JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                PRIMARY KEY (namespace, record_id)
            )
            "#,
        ];

        for stmt in ddl {
            sqlx::query(stmt)
                .execute(&self.pool)
                .await
                .map_err(|e| StorageError::Backend(format!("schema init failed: {e}")))?;
        }
        // Backward-compatible columns for deployments created before lifecycle timestamps.
        sqlx::query(
            "ALTER TABLE maple_commitments ADD COLUMN IF NOT EXISTS execution_started_at TIMESTAMPTZ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(format!("schema migration failed: {e}")))?;
        sqlx::query(
            "ALTER TABLE maple_commitments ADD COLUMN IF NOT EXISTS execution_completed_at TIMESTAMPTZ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(format!("schema migration failed: {e}")))?;
        Ok(())
    }
}

#[async_trait]
impl CommitmentStore for PostgresMapleStorage {
    async fn create_commitment(
        &self,
        commitment: RcfCommitment,
        decision: PolicyDecisionCard,
        declared_at: DateTime<Utc>,
    ) -> StorageResult<()> {
        let lifecycle_status = decision_to_lifecycle(decision.decision);
        let commitment_json = serde_json::to_value(&commitment)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let decision_json = serde_json::to_value(&decision)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO maple_commitments
                (commitment_id, commitment, decision, lifecycle_status, execution_started_at, execution_completed_at, outcome, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NULL, NULL, NULL, $5, $5)
            "#,
        )
        .bind(commitment.commitment_id.0.clone())
        .bind(commitment_json)
        .bind(decision_json)
        .bind(lifecycle_status_to_str(lifecycle_status))
        .bind(declared_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_conflict)?;

        Ok(())
    }

    async fn transition_lifecycle(
        &self,
        commitment_id: &CommitmentId,
        expected_from: LifecycleStatus,
        to: LifecycleStatus,
        updated_at: DateTime<Utc>,
    ) -> StorageResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE maple_commitments
               SET lifecycle_status = $1,
                   execution_started_at = CASE
                        WHEN $1 = 'Executing'::TEXT AND execution_started_at IS NULL THEN $2
                        ELSE execution_started_at
                   END,
                   updated_at = $2
             WHERE commitment_id = $3
               AND lifecycle_status = $4
            "#,
        )
        .bind(lifecycle_status_to_str(to))
        .bind(updated_at)
        .bind(commitment_id.0.clone())
        .bind(lifecycle_status_to_str(expected_from))
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        if result.rows_affected() == 0 {
            let exists = self.get_commitment(commitment_id).await?.is_some();
            if exists {
                return Err(StorageError::InvariantViolation(format!(
                    "invalid lifecycle transition for commitment {}",
                    commitment_id
                )));
            }
            return Err(StorageError::NotFound(format!(
                "commitment {} not found",
                commitment_id
            )));
        }

        Ok(())
    }

    async fn set_outcome(
        &self,
        commitment_id: &CommitmentId,
        outcome: CommitmentOutcome,
        final_status: LifecycleStatus,
    ) -> StorageResult<()> {
        let outcome_json = serde_json::to_value(&outcome)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        let result = sqlx::query(
            r#"
            UPDATE maple_commitments
               SET outcome = $1,
                   lifecycle_status = $2,
                   execution_completed_at = $3,
                   updated_at = $3
             WHERE commitment_id = $4
            "#,
        )
        .bind(outcome_json)
        .bind(lifecycle_status_to_str(final_status))
        .bind(Utc::now())
        .bind(commitment_id.0.clone())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "commitment {} not found",
                commitment_id
            )));
        }

        Ok(())
    }

    async fn get_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> StorageResult<Option<CommitmentRecord>> {
        let row = sqlx::query(
            r#"
            SELECT commitment_id, commitment, decision, lifecycle_status, outcome, created_at, updated_at
              , execution_started_at, execution_completed_at
              FROM maple_commitments
             WHERE commitment_id = $1
            "#,
        )
        .bind(commitment_id.0.clone())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        row.map(commitment_row_to_record).transpose()
    }

    async fn list_commitments(&self, window: QueryWindow) -> StorageResult<Vec<CommitmentRecord>> {
        let rows = if window.limit == 0 {
            sqlx::query(
                r#"
                SELECT commitment_id, commitment, decision, lifecycle_status, outcome, created_at, updated_at
                  , execution_started_at, execution_completed_at
                  FROM maple_commitments
                 ORDER BY updated_at DESC
                 OFFSET $1
                "#,
            )
            .bind(to_i64(window.offset)?)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        } else {
            sqlx::query(
                r#"
                SELECT commitment_id, commitment, decision, lifecycle_status, outcome, created_at, updated_at
                  , execution_started_at, execution_completed_at
                  FROM maple_commitments
                 ORDER BY updated_at DESC
                 LIMIT $1 OFFSET $2
                "#,
            )
            .bind(to_i64(window.limit)?)
            .bind(to_i64(window.offset)?)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        };

        rows.into_iter().map(commitment_row_to_record).collect()
    }
}

#[async_trait]
impl AuditStore for PostgresMapleStorage {
    async fn append_audit(&self, event: AuditAppend) -> StorageResult<AuditRecord> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let conn = tx
            .acquire()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        sqlx::query("LOCK TABLE maple_audit_events IN EXCLUSIVE MODE")
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let last = sqlx::query(
            "SELECT sequence, hash FROM maple_audit_events ORDER BY sequence DESC LIMIT 1",
        )
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        let (sequence, previous_hash) = if let Some(row) = last {
            let seq: i64 = row
                .try_get("sequence")
                .map_err(|e| StorageError::Backend(e.to_string()))?;
            let prev: String = row
                .try_get("hash")
                .map_err(|e| StorageError::Backend(e.to_string()))?;
            (seq + 1, Some(prev))
        } else {
            (1_i64, None)
        };

        let hash = compute_audit_hash(&event, previous_hash.as_deref(), sequence as u64)?;
        let event_id = format!("audit-{}", Uuid::new_v4());

        sqlx::query(
            r#"
            INSERT INTO maple_audit_events
                (event_id, sequence, timestamp, actor, stage, success, message, commitment_id, payload, previous_hash, hash)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(event_id.clone())
        .bind(sequence)
        .bind(event.timestamp)
        .bind(event.actor.clone())
        .bind(event.stage.clone())
        .bind(event.success)
        .bind(event.message.clone())
        .bind(event.commitment_id.as_ref().map(|id| id.0.clone()))
        .bind(event.payload.clone())
        .bind(previous_hash.clone())
        .bind(hash.clone())
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(AuditRecord {
            event_id,
            sequence: sequence as u64,
            timestamp: event.timestamp,
            actor: event.actor,
            stage: event.stage,
            success: event.success,
            message: event.message,
            commitment_id: event.commitment_id,
            payload: event.payload,
            previous_hash,
            hash,
        })
    }

    async fn list_audit(&self, window: QueryWindow) -> StorageResult<Vec<AuditRecord>> {
        let rows = if window.limit == 0 {
            sqlx::query(
                r#"
                SELECT event_id, sequence, timestamp, actor, stage, success, message, commitment_id, payload, previous_hash, hash
                  FROM maple_audit_events
                 ORDER BY sequence DESC
                 OFFSET $1
                "#,
            )
            .bind(to_i64(window.offset)?)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        } else {
            sqlx::query(
                r#"
                SELECT event_id, sequence, timestamp, actor, stage, success, message, commitment_id, payload, previous_hash, hash
                  FROM maple_audit_events
                 ORDER BY sequence DESC
                 LIMIT $1 OFFSET $2
                "#,
            )
            .bind(to_i64(window.limit)?)
            .bind(to_i64(window.offset)?)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        };

        rows.into_iter().map(audit_row_to_record).collect()
    }

    async fn latest_audit_hash(&self) -> StorageResult<Option<String>> {
        let row = sqlx::query("SELECT hash FROM maple_audit_events ORDER BY sequence DESC LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(row
            .map(|r| r.try_get::<String, _>("hash"))
            .transpose()
            .map_err(|e| StorageError::Backend(e.to_string()))?)
    }
}

#[async_trait]
impl AgentStateStore for PostgresMapleStorage {
    async fn upsert_checkpoint(&self, checkpoint: AgentCheckpoint) -> StorageResult<()> {
        let active = serde_json::to_value(&checkpoint.active_commitments)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO maple_agent_checkpoints
                (resonator_id, profile_name, state, active_commitments, last_audit_event_id, metadata, updated_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (resonator_id) DO UPDATE SET
                profile_name = EXCLUDED.profile_name,
                state = EXCLUDED.state,
                active_commitments = EXCLUDED.active_commitments,
                last_audit_event_id = EXCLUDED.last_audit_event_id,
                metadata = EXCLUDED.metadata,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(checkpoint.resonator_id)
        .bind(checkpoint.profile_name)
        .bind(checkpoint.state)
        .bind(active)
        .bind(checkpoint.last_audit_event_id)
        .bind(checkpoint.metadata)
        .bind(checkpoint.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(())
    }

    async fn get_checkpoint(&self, resonator_id: &str) -> StorageResult<Option<AgentCheckpoint>> {
        let row = sqlx::query(
            r#"
            SELECT resonator_id, profile_name, state, active_commitments, last_audit_event_id, metadata, updated_at
              FROM maple_agent_checkpoints
             WHERE resonator_id = $1
            "#,
        )
        .bind(resonator_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        row.map(checkpoint_row_to_record).transpose()
    }

    async fn list_checkpoints(&self, window: QueryWindow) -> StorageResult<Vec<AgentCheckpoint>> {
        let rows = if window.limit == 0 {
            sqlx::query(
                r#"
                SELECT resonator_id, profile_name, state, active_commitments, last_audit_event_id, metadata, updated_at
                  FROM maple_agent_checkpoints
                 ORDER BY updated_at DESC
                 OFFSET $1
                "#,
            )
            .bind(to_i64(window.offset)?)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        } else {
            sqlx::query(
                r#"
                SELECT resonator_id, profile_name, state, active_commitments, last_audit_event_id, metadata, updated_at
                  FROM maple_agent_checkpoints
                 ORDER BY updated_at DESC
                 LIMIT $1 OFFSET $2
                "#,
            )
            .bind(to_i64(window.limit)?)
            .bind(to_i64(window.offset)?)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        };

        rows.into_iter().map(checkpoint_row_to_record).collect()
    }
}

#[async_trait]
impl ProjectionStore for PostgresMapleStorage {
    async fn upsert_projection(&self, snapshot: ProjectionSnapshot) -> StorageResult<()> {
        sqlx::query(
            r#"
            INSERT INTO maple_projections (namespace, key, schema_version, data, snapshot_hash, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (namespace, key) DO UPDATE SET
                schema_version = EXCLUDED.schema_version,
                data = EXCLUDED.data,
                snapshot_hash = EXCLUDED.snapshot_hash,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(snapshot.namespace)
        .bind(snapshot.key)
        .bind(snapshot.schema_version)
        .bind(snapshot.data)
        .bind(snapshot.snapshot_hash)
        .bind(snapshot.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(())
    }

    async fn get_projection(
        &self,
        namespace: &str,
        key: &str,
    ) -> StorageResult<Option<ProjectionSnapshot>> {
        let row = sqlx::query(
            r#"
            SELECT namespace, key, schema_version, data, snapshot_hash, created_at
              FROM maple_projections
             WHERE namespace = $1 AND key = $2
            "#,
        )
        .bind(namespace)
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        row.map(projection_row_to_record).transpose()
    }

    async fn list_projections(
        &self,
        namespace: &str,
        window: QueryWindow,
    ) -> StorageResult<Vec<ProjectionSnapshot>> {
        let rows = if window.limit == 0 {
            sqlx::query(
                r#"
                SELECT namespace, key, schema_version, data, snapshot_hash, created_at
                  FROM maple_projections
                 WHERE namespace = $1
                 ORDER BY created_at DESC
                 OFFSET $2
                "#,
            )
            .bind(namespace)
            .bind(to_i64(window.offset)?)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        } else {
            sqlx::query(
                r#"
                SELECT namespace, key, schema_version, data, snapshot_hash, created_at
                  FROM maple_projections
                 WHERE namespace = $1
                 ORDER BY created_at DESC
                 LIMIT $2 OFFSET $3
                "#,
            )
            .bind(namespace)
            .bind(to_i64(window.limit)?)
            .bind(to_i64(window.offset)?)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        };

        rows.into_iter().map(projection_row_to_record).collect()
    }
}

#[async_trait]
impl SemanticMemoryStore for PostgresMapleStorage {
    async fn upsert_semantic(&self, record: SemanticRecord) -> StorageResult<()> {
        let embedding = serde_json::to_value(&record.embedding)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        sqlx::query(
            r#"
            INSERT INTO maple_semantic_records (namespace, record_id, embedding, content, metadata, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (namespace, record_id) DO UPDATE SET
                embedding = EXCLUDED.embedding,
                content = EXCLUDED.content,
                metadata = EXCLUDED.metadata,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(record.namespace)
        .bind(record.record_id)
        .bind(embedding)
        .bind(record.content)
        .bind(record.metadata)
        .bind(record.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(())
    }

    async fn search_semantic(
        &self,
        namespace: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> StorageResult<Vec<SemanticHit>> {
        if query_embedding.is_empty() {
            return Err(StorageError::InvalidInput(
                "query embedding must not be empty".to_string(),
            ));
        }

        let rows = sqlx::query(
            r#"
            SELECT namespace, record_id, embedding, content, metadata, created_at
              FROM maple_semantic_records
             WHERE namespace = $1
            "#,
        )
        .bind(namespace)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        let mut hits = rows
            .into_iter()
            .filter_map(|row| semantic_row_to_record(row).ok())
            .filter_map(|record| {
                cosine_similarity(query_embedding, &record.embedding)
                    .map(|score| SemanticHit { record, score })
            })
            .collect::<Vec<_>>();

        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        if limit > 0 {
            hits.truncate(limit);
        }
        Ok(hits)
    }
}

fn commitment_row_to_record(row: sqlx::postgres::PgRow) -> StorageResult<CommitmentRecord> {
    let commitment_json: serde_json::Value = row
        .try_get("commitment")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let decision_json: serde_json::Value = row
        .try_get("decision")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let outcome_json: Option<serde_json::Value> = row
        .try_get("outcome")
        .map_err(|e| StorageError::Backend(e.to_string()))?;

    let commitment: RcfCommitment = serde_json::from_value(commitment_json)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let decision: PolicyDecisionCard = serde_json::from_value(decision_json)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let outcome: Option<CommitmentOutcome> = outcome_json
        .map(|v| serde_json::from_value(v).map_err(|e| StorageError::Serialization(e.to_string())))
        .transpose()?;

    let lifecycle: String = row
        .try_get("lifecycle_status")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    Ok(CommitmentRecord {
        commitment_id: CommitmentId::new(
            row.try_get::<String, _>("commitment_id")
                .map_err(|e| StorageError::Backend(e.to_string()))?,
        ),
        commitment,
        decision,
        lifecycle_status: parse_lifecycle_status(&lifecycle)?,
        execution_started_at: row
            .try_get("execution_started_at")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        execution_completed_at: row
            .try_get("execution_completed_at")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        outcome,
        created_at: row
            .try_get("created_at")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        updated_at: row
            .try_get("updated_at")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
    })
}

fn audit_row_to_record(row: sqlx::postgres::PgRow) -> StorageResult<AuditRecord> {
    let commitment_id: Option<String> = row
        .try_get("commitment_id")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    Ok(AuditRecord {
        event_id: row
            .try_get("event_id")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        sequence: row
            .try_get::<i64, _>("sequence")
            .map_err(|e| StorageError::Backend(e.to_string()))? as u64,
        timestamp: row
            .try_get("timestamp")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        actor: row
            .try_get("actor")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        stage: row
            .try_get("stage")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        success: row
            .try_get("success")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        message: row
            .try_get("message")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        commitment_id: commitment_id.map(CommitmentId::new),
        payload: row
            .try_get("payload")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        previous_hash: row
            .try_get("previous_hash")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        hash: row
            .try_get("hash")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
    })
}

fn checkpoint_row_to_record(row: sqlx::postgres::PgRow) -> StorageResult<AgentCheckpoint> {
    let active_json: serde_json::Value = row
        .try_get("active_commitments")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let active_commitments: Vec<String> = serde_json::from_value(active_json)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;

    Ok(AgentCheckpoint {
        resonator_id: row
            .try_get("resonator_id")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        profile_name: row
            .try_get("profile_name")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        state: row
            .try_get("state")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        active_commitments,
        last_audit_event_id: row
            .try_get("last_audit_event_id")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        metadata: row
            .try_get("metadata")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        updated_at: row
            .try_get("updated_at")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
    })
}

fn projection_row_to_record(row: sqlx::postgres::PgRow) -> StorageResult<ProjectionSnapshot> {
    Ok(ProjectionSnapshot {
        namespace: row
            .try_get("namespace")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        key: row
            .try_get("key")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        schema_version: row
            .try_get("schema_version")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        data: row
            .try_get("data")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        snapshot_hash: row
            .try_get("snapshot_hash")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        created_at: row
            .try_get("created_at")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
    })
}

fn semantic_row_to_record(row: sqlx::postgres::PgRow) -> StorageResult<SemanticRecord> {
    let embedding_json: serde_json::Value = row
        .try_get("embedding")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let embedding: Vec<f32> = serde_json::from_value(embedding_json)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;

    Ok(SemanticRecord {
        namespace: row
            .try_get("namespace")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        record_id: row
            .try_get("record_id")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        embedding,
        content: row
            .try_get("content")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        metadata: row
            .try_get("metadata")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
        created_at: row
            .try_get("created_at")
            .map_err(|e| StorageError::Backend(e.to_string()))?,
    })
}

fn decision_to_lifecycle(decision: Decision) -> LifecycleStatus {
    match decision {
        Decision::Approved => LifecycleStatus::Approved,
        Decision::Denied => LifecycleStatus::Denied,
        Decision::PendingHumanReview | Decision::PendingAdditionalInfo => LifecycleStatus::Pending,
    }
}

fn lifecycle_status_to_str(status: LifecycleStatus) -> &'static str {
    match status {
        LifecycleStatus::Pending => "pending",
        LifecycleStatus::Approved => "approved",
        LifecycleStatus::Denied => "denied",
        LifecycleStatus::Executing => "executing",
        LifecycleStatus::Completed => "completed",
        LifecycleStatus::Failed => "failed",
        LifecycleStatus::Expired => "expired",
    }
}

fn parse_lifecycle_status(raw: &str) -> StorageResult<LifecycleStatus> {
    match raw {
        "pending" => Ok(LifecycleStatus::Pending),
        "approved" => Ok(LifecycleStatus::Approved),
        "denied" => Ok(LifecycleStatus::Denied),
        "executing" => Ok(LifecycleStatus::Executing),
        "completed" => Ok(LifecycleStatus::Completed),
        "failed" => Ok(LifecycleStatus::Failed),
        "expired" => Ok(LifecycleStatus::Expired),
        _ => Err(StorageError::Serialization(format!(
            "unknown lifecycle status `{raw}`"
        ))),
    }
}

fn map_sqlx_conflict(err: sqlx::Error) -> StorageError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.code().as_deref() == Some("23505") {
            return StorageError::Conflict(db_err.message().to_string());
        }
    }
    StorageError::Backend(err.to_string())
}

fn to_i64(value: usize) -> StorageResult<i64> {
    i64::try_from(value)
        .map_err(|_| StorageError::InvalidInput("window value too large".to_string()))
}

fn compute_audit_hash(
    event: &AuditAppend,
    previous_hash: Option<&str>,
    sequence: u64,
) -> StorageResult<String> {
    let serializable = serde_json::json!({
        "previous_hash": previous_hash,
        "sequence": sequence,
        "timestamp": event.timestamp,
        "actor": event.actor,
        "stage": event.stage,
        "success": event.success,
        "message": event.message,
        "commitment_id": event.commitment_id.as_ref().map(|id| id.0.clone()),
        "payload": event.payload,
    });
    let serialized = serde_json::to_vec(&serializable)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    Ok(blake3::hash(&serialized).to_hex().to_string())
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }
    let (mut dot, mut norm_a, mut norm_b) = (0.0_f32, 0.0_f32, 0.0_f32);
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return None;
    }
    Some(dot / (norm_a.sqrt() * norm_b.sqrt()))
}
