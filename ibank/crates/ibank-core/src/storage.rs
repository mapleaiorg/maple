use crate::error::IBankError;
use crate::ledger::{AppendOnlyLedger, AuditEvent, LedgerEntry, LedgerEntryKind};
use crate::types::{CommitmentRecord, ConsequenceRecord};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

/// Ledger persistence backend configuration.
#[derive(Debug, Clone)]
pub enum LedgerStorageConfig {
    /// Keep all commitment/audit/outcome entries in process memory only.
    Memory,
    /// Persist all entries in PostgreSQL and hydrate ledger state on startup.
    Postgres {
        database_url: String,
        max_connections: u32,
    },
}

impl LedgerStorageConfig {
    pub fn memory() -> Self {
        Self::Memory
    }

    pub fn postgres(database_url: impl Into<String>, max_connections: u32) -> Self {
        Self::Postgres {
            database_url: database_url.into(),
            max_connections,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Postgres { .. } => "postgres",
        }
    }
}

impl Default for LedgerStorageConfig {
    fn default() -> Self {
        Self::Memory
    }
}

#[derive(Debug, Clone)]
enum LedgerStorageBackend {
    Memory,
    Postgres(PostgresLedgerStore),
}

/// Runtime ledger wrapper that keeps an in-memory authoritative chain while
/// optionally mirroring each entry to PostgreSQL.
///
/// Invariant handling:
/// - Entry hash/index is computed against in-memory chain first.
/// - Entry is persisted before it is committed in-memory.
/// - On startup, PostgreSQL entries are hydrated and hash-verified.
#[derive(Debug, Clone)]
pub struct PersistentLedger {
    ledger: AppendOnlyLedger,
    backend: LedgerStorageBackend,
}

impl PersistentLedger {
    /// Build an in-memory persistent ledger from already persisted entries.
    pub fn from_entries(entries: Vec<LedgerEntry>) -> Result<Self, IBankError> {
        Ok(Self {
            ledger: AppendOnlyLedger::from_entries(entries)?,
            backend: LedgerStorageBackend::Memory,
        })
    }

    pub async fn bootstrap(config: LedgerStorageConfig) -> Result<Self, IBankError> {
        match config {
            LedgerStorageConfig::Memory => Ok(Self {
                ledger: AppendOnlyLedger::new(),
                backend: LedgerStorageBackend::Memory,
            }),
            LedgerStorageConfig::Postgres {
                database_url,
                max_connections,
            } => {
                let store = PostgresLedgerStore::connect(&database_url, max_connections).await?;
                store.ensure_schema().await?;
                let entries = store.load_entries().await?;
                let ledger = AppendOnlyLedger::from_entries(entries)?;
                Ok(Self {
                    ledger,
                    backend: LedgerStorageBackend::Postgres(store),
                })
            }
        }
    }

    pub fn backend_label(&self) -> &'static str {
        match self.backend {
            LedgerStorageBackend::Memory => "memory",
            LedgerStorageBackend::Postgres(_) => "postgres",
        }
    }

    pub fn entries(&self) -> &[LedgerEntry] {
        self.ledger.entries()
    }

    pub fn as_append_only(&self) -> &AppendOnlyLedger {
        &self.ledger
    }

    pub fn verify_chain(&self) -> bool {
        self.ledger.verify_chain()
    }

    pub fn commitment_exists(&self, commitment_id: &str) -> bool {
        self.ledger.commitment_exists(commitment_id)
    }

    pub fn find_entry(&self, entry_id: &str) -> Option<&LedgerEntry> {
        self.ledger.find_entry(entry_id)
    }

    pub async fn append_commitment_record(
        &mut self,
        trace_id: &str,
        record: &CommitmentRecord,
    ) -> Result<LedgerEntry, IBankError> {
        let payload =
            serde_json::to_value(record).map_err(|e| IBankError::Serialization(e.to_string()))?;
        self.append(
            trace_id,
            LedgerEntryKind::Commitment,
            Some(record.commitment.commitment_id.to_string()),
            payload,
        )
        .await
    }

    pub async fn append_audit(
        &mut self,
        trace_id: &str,
        commitment_id: Option<String>,
        event: AuditEvent,
    ) -> Result<LedgerEntry, IBankError> {
        let payload =
            serde_json::to_value(event).map_err(|e| IBankError::Serialization(e.to_string()))?;
        self.append(trace_id, LedgerEntryKind::Audit, commitment_id, payload)
            .await
    }

    pub async fn append_outcome(
        &mut self,
        trace_id: &str,
        commitment_id: Option<String>,
        outcome: &ConsequenceRecord,
    ) -> Result<LedgerEntry, IBankError> {
        let payload =
            serde_json::to_value(outcome).map_err(|e| IBankError::Serialization(e.to_string()))?;
        self.append(trace_id, LedgerEntryKind::Outcome, commitment_id, payload)
            .await
    }

    async fn append(
        &mut self,
        trace_id: &str,
        kind: LedgerEntryKind,
        commitment_id: Option<String>,
        payload: serde_json::Value,
    ) -> Result<LedgerEntry, IBankError> {
        let entry = self
            .ledger
            .build_entry(trace_id, kind, commitment_id, payload)?;

        if let LedgerStorageBackend::Postgres(store) = &self.backend {
            store.insert_entry(&entry).await?;
        }

        self.ledger.commit_entry(entry.clone())?;
        Ok(entry)
    }
}

#[derive(Debug, Clone)]
struct PostgresLedgerStore {
    pool: PgPool,
}

impl PostgresLedgerStore {
    async fn connect(database_url: &str, max_connections: u32) -> Result<Self, IBankError> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections.max(1))
            .connect(database_url)
            .await
            .map_err(|e| IBankError::Ledger(format!("postgres connect failed: {e}")))?;

        Ok(Self { pool })
    }

    async fn ensure_schema(&self) -> Result<(), IBankError> {
        // Single append-only table for commitments/audit/outcomes.
        // The application controls deterministic index/hash generation.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS ibank_ledger_entries (
                ledger_index BIGINT PRIMARY KEY,
                entry_id TEXT NOT NULL UNIQUE,
                trace_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                commitment_id TEXT NULL,
                entry_timestamp TIMESTAMPTZ NOT NULL,
                payload JSONB NOT NULL,
                previous_hash TEXT NULL,
                entry_hash TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| IBankError::Ledger(format!("postgres schema create failed: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_ibank_ledger_trace_id ON ibank_ledger_entries (trace_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| IBankError::Ledger(format!("postgres index create failed: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_ibank_ledger_commitment_id ON ibank_ledger_entries (commitment_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| IBankError::Ledger(format!("postgres index create failed: {e}")))?;

        Ok(())
    }

    async fn load_entries(&self) -> Result<Vec<LedgerEntry>, IBankError> {
        let rows = sqlx::query(
            r#"
            SELECT
                ledger_index,
                entry_id,
                trace_id,
                kind,
                commitment_id,
                entry_timestamp,
                payload,
                previous_hash,
                entry_hash
            FROM ibank_ledger_entries
            ORDER BY ledger_index ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| IBankError::Ledger(format!("postgres load failed: {e}")))?;

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let kind_str: String = row
                .try_get("kind")
                .map_err(|e| IBankError::Ledger(format!("postgres decode kind failed: {e}")))?;
            let kind = parse_kind(&kind_str)?;

            let index: i64 = row.try_get("ledger_index").map_err(|e| {
                IBankError::Ledger(format!("postgres decode ledger_index failed: {e}"))
            })?;

            entries.push(LedgerEntry {
                entry_id: row.try_get("entry_id").map_err(|e| {
                    IBankError::Ledger(format!("postgres decode entry_id failed: {e}"))
                })?,
                index: index.try_into().map_err(|_| {
                    IBankError::Ledger("negative ledger index in storage".to_string())
                })?,
                trace_id: row.try_get("trace_id").map_err(|e| {
                    IBankError::Ledger(format!("postgres decode trace_id failed: {e}"))
                })?,
                kind,
                commitment_id: row.try_get("commitment_id").map_err(|e| {
                    IBankError::Ledger(format!("postgres decode commitment_id failed: {e}"))
                })?,
                timestamp: row.try_get("entry_timestamp").map_err(|e| {
                    IBankError::Ledger(format!("postgres decode entry_timestamp failed: {e}"))
                })?,
                payload: row.try_get("payload").map_err(|e| {
                    IBankError::Ledger(format!("postgres decode payload failed: {e}"))
                })?,
                previous_hash: row.try_get("previous_hash").map_err(|e| {
                    IBankError::Ledger(format!("postgres decode previous_hash failed: {e}"))
                })?,
                entry_hash: row.try_get("entry_hash").map_err(|e| {
                    IBankError::Ledger(format!("postgres decode entry_hash failed: {e}"))
                })?,
            });
        }

        Ok(entries)
    }

    async fn insert_entry(&self, entry: &LedgerEntry) -> Result<(), IBankError> {
        let index: i64 = entry.index.try_into().map_err(|_| {
            IBankError::Ledger("ledger index exceeds postgres BIGINT range".to_string())
        })?;
        sqlx::query(
            r#"
            INSERT INTO ibank_ledger_entries (
                ledger_index,
                entry_id,
                trace_id,
                kind,
                commitment_id,
                entry_timestamp,
                payload,
                previous_hash,
                entry_hash
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(index)
        .bind(&entry.entry_id)
        .bind(&entry.trace_id)
        .bind(kind_to_str(&entry.kind))
        .bind(&entry.commitment_id)
        .bind(entry.timestamp)
        .bind(&entry.payload)
        .bind(&entry.previous_hash)
        .bind(&entry.entry_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| IBankError::Ledger(format!("postgres insert failed: {e}")))?;

        Ok(())
    }
}

fn kind_to_str(kind: &LedgerEntryKind) -> &'static str {
    match kind {
        LedgerEntryKind::Commitment => "commitment",
        LedgerEntryKind::Audit => "audit",
        LedgerEntryKind::Outcome => "outcome",
    }
}

fn parse_kind(value: &str) -> Result<LedgerEntryKind, IBankError> {
    match value {
        "commitment" => Ok(LedgerEntryKind::Commitment),
        "audit" => Ok(LedgerEntryKind::Audit),
        "outcome" => Ok(LedgerEntryKind::Outcome),
        other => Err(IBankError::Ledger(format!(
            "unknown ledger kind '{other}' in postgres"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{AppendOnlyLedger, AuditEvent};
    use crate::types::ConsequenceRecord;
    use chrono::Utc;

    #[tokio::test]
    async fn memory_backend_appends_and_verifies_hash_chain() {
        let mut ledger = PersistentLedger::bootstrap(LedgerStorageConfig::memory())
            .await
            .unwrap();

        ledger
            .append_audit("trace-a", None, AuditEvent::new("stage", "detail"))
            .await
            .unwrap();
        ledger
            .append_outcome(
                "trace-a",
                None,
                &ConsequenceRecord {
                    success: false,
                    detail: "failed".to_string(),
                    route: None,
                    occurred_at: Utc::now(),
                },
            )
            .await
            .unwrap();

        assert_eq!(ledger.entries().len(), 2);
        assert!(ledger.verify_chain());
    }

    #[test]
    fn kind_string_roundtrip() {
        let kinds = [
            LedgerEntryKind::Commitment,
            LedgerEntryKind::Audit,
            LedgerEntryKind::Outcome,
        ];

        for kind in kinds {
            let parsed = parse_kind(kind_to_str(&kind)).unwrap();
            assert_eq!(kind, parsed);
        }
    }

    #[test]
    fn from_entries_rehydrates_verified_chain() {
        let mut base = AppendOnlyLedger::new();
        let first = base
            .append_audit("trace-a", None, AuditEvent::new("prepared", "ok"))
            .unwrap();
        base.append_outcome(
            "trace-a",
            None,
            &ConsequenceRecord {
                success: true,
                detail: "done".to_string(),
                route: None,
                occurred_at: Utc::now(),
            },
        )
        .unwrap();

        let rehydrated = PersistentLedger::from_entries(base.entries().to_vec()).unwrap();
        assert_eq!(rehydrated.entries().len(), 2);
        assert_eq!(rehydrated.entries()[0].entry_id, first.entry_id);
        assert!(rehydrated.verify_chain());
    }
}
