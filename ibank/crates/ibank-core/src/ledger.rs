use crate::error::IBankError;
use crate::types::{CommitmentRecord, ConsequenceRecord};
use chrono::{DateTime, Utc};
use rcf_commitment::RcfCommitment;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Ledger entry types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LedgerEntryKind {
    Commitment,
    Audit,
    Outcome,
}

/// Hash-chained ledger entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub entry_id: String,
    pub index: u64,
    pub trace_id: String,
    pub kind: LedgerEntryKind,
    pub commitment_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub payload: Value,
    pub previous_hash: Option<String>,
    pub entry_hash: String,
}

/// Audit payload persisted in append-only log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub stage: String,
    pub detail: String,
}

impl AuditEvent {
    pub fn new(stage: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            detail: detail.into(),
        }
    }
}

/// Append-only ledger with hash-chain proofs.
///
/// Design choice: no in-place mutation APIs are exposed. Every state transition becomes
/// an additional record, which preserves full historical accountability.
#[derive(Debug, Default, Clone)]
pub struct AppendOnlyLedger {
    entries: Vec<LedgerEntry>,
}

impl AppendOnlyLedger {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Rebuild a ledger from persisted entries and verify hash-chain integrity.
    pub fn from_entries(entries: Vec<LedgerEntry>) -> Result<Self, IBankError> {
        let ledger = Self { entries };

        for (expected_index, entry) in ledger.entries.iter().enumerate() {
            if entry.index != expected_index as u64 {
                return Err(IBankError::Ledger(format!(
                    "ledger index gap detected at position {} (found {})",
                    expected_index, entry.index
                )));
            }
        }

        if !ledger.verify_chain() {
            return Err(IBankError::Ledger(
                "persisted ledger hash-chain verification failed".to_string(),
            ));
        }

        Ok(ledger)
    }

    pub fn entries(&self) -> &[LedgerEntry] {
        &self.entries
    }

    pub fn commitment_exists(&self, commitment_id: &str) -> bool {
        self.entries.iter().any(|entry| {
            entry.kind == LedgerEntryKind::Commitment
                && entry
                    .commitment_id
                    .as_ref()
                    .map(|id| id == commitment_id)
                    .unwrap_or(false)
        })
    }

    pub fn find_entry(&self, entry_id: &str) -> Option<&LedgerEntry> {
        self.entries.iter().find(|entry| entry.entry_id == entry_id)
    }

    pub fn append_commitment(
        &mut self,
        trace_id: &str,
        commitment: &RcfCommitment,
    ) -> Result<LedgerEntry, IBankError> {
        let payload = serde_json::to_value(commitment)
            .map_err(|e| IBankError::Serialization(e.to_string()))?;
        self.append(
            trace_id,
            LedgerEntryKind::Commitment,
            Some(commitment.commitment_id.to_string()),
            payload,
        )
    }

    /// Append a commitment plus iBank platform context.
    ///
    /// This path is used by the iBank Core Engine so commitment entries carry
    /// deterministic risk/compliance snapshots required by downstream audits.
    pub fn append_commitment_record(
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
    }

    pub fn append_audit(
        &mut self,
        trace_id: &str,
        commitment_id: Option<String>,
        event: AuditEvent,
    ) -> Result<LedgerEntry, IBankError> {
        let payload =
            serde_json::to_value(event).map_err(|e| IBankError::Serialization(e.to_string()))?;
        self.append(trace_id, LedgerEntryKind::Audit, commitment_id, payload)
    }

    pub fn append_outcome(
        &mut self,
        trace_id: &str,
        commitment_id: Option<String>,
        outcome: &ConsequenceRecord,
    ) -> Result<LedgerEntry, IBankError> {
        let payload =
            serde_json::to_value(outcome).map_err(|e| IBankError::Serialization(e.to_string()))?;
        self.append(trace_id, LedgerEntryKind::Outcome, commitment_id, payload)
    }

    pub fn verify_chain(&self) -> bool {
        let mut previous_hash: Option<String> = None;
        for entry in &self.entries {
            let expected_hash = compute_entry_hash(
                entry.index,
                &entry.trace_id,
                &entry.kind,
                entry.commitment_id.as_deref(),
                entry.timestamp,
                &entry.payload,
                previous_hash.as_deref(),
            );
            if entry.entry_hash != expected_hash {
                return false;
            }
            if entry.previous_hash != previous_hash {
                return false;
            }
            previous_hash = Some(entry.entry_hash.clone());
        }
        true
    }

    fn append(
        &mut self,
        trace_id: &str,
        kind: LedgerEntryKind,
        commitment_id: Option<String>,
        payload: Value,
    ) -> Result<LedgerEntry, IBankError> {
        let entry = self.build_entry(trace_id, kind, commitment_id, payload)?;
        self.commit_entry(entry.clone())?;
        Ok(entry)
    }

    /// Build the next deterministic entry without mutating the in-memory chain.
    pub fn build_entry(
        &self,
        trace_id: &str,
        kind: LedgerEntryKind,
        commitment_id: Option<String>,
        payload: Value,
    ) -> Result<LedgerEntry, IBankError> {
        let index = self.entries.len() as u64;
        let timestamp = Utc::now();
        let previous_hash = self.entries.last().map(|entry| entry.entry_hash.clone());
        let entry_hash = compute_entry_hash(
            index,
            trace_id,
            &kind,
            commitment_id.as_deref(),
            timestamp,
            &payload,
            previous_hash.as_deref(),
        );

        Ok(LedgerEntry {
            entry_id: Uuid::new_v4().to_string(),
            index,
            trace_id: trace_id.to_string(),
            kind,
            commitment_id,
            timestamp,
            payload,
            previous_hash,
            entry_hash,
        })
    }

    /// Commit a pre-built entry after external durability succeeds.
    pub fn commit_entry(&mut self, entry: LedgerEntry) -> Result<(), IBankError> {
        let expected_index = self.entries.len() as u64;
        if entry.index != expected_index {
            return Err(IBankError::Ledger(format!(
                "commit index mismatch: expected {}, got {}",
                expected_index, entry.index
            )));
        }

        let expected_previous_hash = self.entries.last().map(|e| e.entry_hash.clone());
        if entry.previous_hash != expected_previous_hash {
            return Err(IBankError::Ledger(
                "commit previous hash mismatch".to_string(),
            ));
        }

        let expected_hash = compute_entry_hash(
            entry.index,
            &entry.trace_id,
            &entry.kind,
            entry.commitment_id.as_deref(),
            entry.timestamp,
            &entry.payload,
            entry.previous_hash.as_deref(),
        );

        if entry.entry_hash != expected_hash {
            return Err(IBankError::Ledger(
                "commit hash mismatch for ledger entry".to_string(),
            ));
        }

        self.entries.push(entry);
        Ok(())
    }
}

fn compute_entry_hash(
    index: u64,
    trace_id: &str,
    kind: &LedgerEntryKind,
    commitment_id: Option<&str>,
    timestamp: DateTime<Utc>,
    payload: &Value,
    previous_hash: Option<&str>,
) -> String {
    let material = serde_json::json!({
        "index": index,
        "trace_id": trace_id,
        "kind": kind,
        "commitment_id": commitment_id,
        "timestamp": timestamp,
        "payload": payload,
        "previous_hash": previous_hash,
    });

    let bytes = serde_json::to_vec(&material).unwrap_or_default();
    blake3::hash(&bytes).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcf_commitment::{CommitmentBuilder, IntendedOutcome};
    use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint};

    #[test]
    fn verifies_hash_chain() {
        let mut ledger = AppendOnlyLedger::new();
        let commitment = CommitmentBuilder::new(IdentityRef::new("bank-a"), EffectDomain::Finance)
            .with_outcome(IntendedOutcome::new("settle transfer"))
            .with_scope(ScopeConstraint::global())
            .build()
            .unwrap();

        ledger
            .append_commitment("trace-1", &commitment)
            .expect("commitment appended");
        ledger
            .append_audit(
                "trace-1",
                Some(commitment.commitment_id.to_string()),
                AuditEvent::new("risk_checked", "score=10"),
            )
            .expect("audit appended");

        assert!(ledger.verify_chain());
    }

    #[test]
    fn detects_tampered_entries() {
        let mut ledger = AppendOnlyLedger::new();
        let commitment = CommitmentBuilder::new(IdentityRef::new("bank-a"), EffectDomain::Finance)
            .with_outcome(IntendedOutcome::new("settle transfer"))
            .with_scope(ScopeConstraint::global())
            .build()
            .unwrap();

        ledger
            .append_commitment("trace-2", &commitment)
            .expect("commitment appended");

        // Clone and tamper outside of append APIs to validate proof behavior.
        let mut tampered = ledger.clone();
        tampered.entries[0].payload = serde_json::json!({"tampered": true});

        assert!(!tampered.verify_chain());
    }
}
