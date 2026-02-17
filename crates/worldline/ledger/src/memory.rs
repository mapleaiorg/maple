use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::RwLock;

use crate::error::LedgerError;
use crate::records::{
    CommitmentProposal, CommitmentReceipt, Decision, OutcomeReceipt, OutcomeRecord, Receipt,
    ReceiptRef, SnapshotInput, SnapshotReceipt,
};
use crate::traits::{LedgerReader, LedgerWriter};

/// In-memory WLL implementation used for tests, local demos, and embedding.
pub struct InMemoryLedger {
    node_id: u16,
    inner: RwLock<LedgerState>,
}

#[derive(Default)]
struct LedgerState {
    streams: HashMap<worldline_types::WorldlineId, Vec<Receipt>>,
    hash_index: HashMap<[u8; 32], (worldline_types::WorldlineId, usize)>,
}

impl InMemoryLedger {
    pub fn new(node_id: u16) -> Self {
        Self {
            node_id,
            inner: RwLock::new(LedgerState::default()),
        }
    }

    /// Validate hash chain, sequence monotonicity, and receipt attribution for one stream.
    pub fn validate_stream(
        &self,
        worldline: &worldline_types::WorldlineId,
    ) -> Result<(), LedgerError> {
        let receipts = self.read_all(worldline)?;
        let mut seen_receipt_hashes = HashSet::new();
        let mut commitment_hashes = HashSet::new();

        for (index, receipt) in receipts.iter().enumerate() {
            let expected_seq = (index + 1) as u64;
            if receipt.seq() != expected_seq {
                return Err(LedgerError::IntegrityViolation {
                    seq: receipt.seq(),
                    reason: format!("expected seq {}, found {}", expected_seq, receipt.seq()),
                });
            }

            let expected_prev = if index == 0 {
                None
            } else {
                Some(receipts[index - 1].receipt_hash())
            };
            if receipt.prev_hash() != expected_prev {
                return Err(LedgerError::IntegrityViolation {
                    seq: receipt.seq(),
                    reason: "previous hash link mismatch".into(),
                });
            }

            let computed_hash = recompute_receipt_hash(receipt)?;
            if computed_hash != receipt.receipt_hash() {
                return Err(LedgerError::IntegrityViolation {
                    seq: receipt.seq(),
                    reason: "receipt hash mismatch".into(),
                });
            }

            seen_receipt_hashes.insert(receipt.receipt_hash());

            match receipt {
                Receipt::Commitment(commitment) => {
                    commitment_hashes.insert(commitment.receipt_hash);
                }
                Receipt::Outcome(outcome) => {
                    if !commitment_hashes.contains(&outcome.commitment_receipt_hash) {
                        return Err(LedgerError::IntegrityViolation {
                            seq: receipt.seq(),
                            reason: "outcome does not reference a commitment receipt".into(),
                        });
                    }
                }
                Receipt::Snapshot(snapshot) => {
                    if !seen_receipt_hashes.contains(&snapshot.anchored_receipt_hash) {
                        return Err(LedgerError::IntegrityViolation {
                            seq: receipt.seq(),
                            reason: "snapshot anchor missing in stream".into(),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn append_receipt(
        &self,
        state: &mut LedgerState,
        worldline: &worldline_types::WorldlineId,
        mut receipt: Receipt,
    ) -> Result<Receipt, LedgerError> {
        let stream = state.streams.entry(worldline.clone()).or_default();
        let expected_seq = (stream.len() + 1) as u64;
        if receipt.seq() != expected_seq {
            return Err(LedgerError::IntegrityViolation {
                seq: receipt.seq(),
                reason: format!(
                    "append attempted out of order; expected seq {}",
                    expected_seq
                ),
            });
        }

        let expected_prev = stream.last().map(Receipt::receipt_hash);
        if receipt.prev_hash() != expected_prev {
            return Err(LedgerError::IntegrityViolation {
                seq: receipt.seq(),
                reason: "append attempted with mismatched previous hash".into(),
            });
        }

        let receipt_hash = recompute_receipt_hash(&receipt)?;
        if state.hash_index.contains_key(&receipt_hash) {
            return Err(LedgerError::HashCollision);
        }

        receipt.set_receipt_hash(receipt_hash);
        stream.push(receipt.clone());
        state
            .hash_index
            .insert(receipt_hash, (worldline.clone(), stream.len() - 1));

        Ok(receipt)
    }

    fn stream_position(
        state: &LedgerState,
        worldline: &worldline_types::WorldlineId,
        node_id: u16,
    ) -> (u64, Option<[u8; 32]>, worldline_types::TemporalAnchor) {
        let last = state.streams.get(worldline).and_then(|s| s.last());
        let seq = state
            .streams
            .get(worldline)
            .map(|s| (s.len() + 1) as u64)
            .unwrap_or(1);
        let prev_hash = last.map(Receipt::receipt_hash);
        let timestamp = next_anchor(last, node_id);
        (seq, prev_hash, timestamp)
    }

    fn find_commitment_by_hash(
        state: &LedgerState,
        receipt_hash: [u8; 32],
    ) -> Result<CommitmentReceipt, LedgerError> {
        let (worldline, index) = state
            .hash_index
            .get(&receipt_hash)
            .cloned()
            .ok_or(LedgerError::MissingCommitmentReceipt)?;

        let receipt = state
            .streams
            .get(&worldline)
            .and_then(|stream| stream.get(index))
            .ok_or(LedgerError::MissingCommitmentReceipt)?;

        receipt
            .as_commitment()
            .cloned()
            .ok_or(LedgerError::MissingCommitmentReceipt)
    }
}

impl Default for InMemoryLedger {
    fn default() -> Self {
        Self::new(0)
    }
}

impl LedgerWriter for InMemoryLedger {
    fn append_commitment(
        &self,
        proposal: &CommitmentProposal,
        decision: &Decision,
        policy_hash: [u8; 32],
    ) -> Result<CommitmentReceipt, LedgerError> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| LedgerError::IntegrityViolation {
                seq: 0,
                reason: "ledger write lock poisoned".into(),
            })?;

        let proposal_hash = hash_json(proposal)?;
        let (seq, prev_hash, timestamp) =
            Self::stream_position(&state, &proposal.worldline, self.node_id);

        let commitment = CommitmentReceipt {
            worldline: proposal.worldline.clone(),
            seq,
            receipt_hash: [0; 32],
            prev_hash,
            timestamp,
            proposal_hash,
            commitment_id: proposal.commitment_id.clone(),
            class: proposal.class.clone(),
            intent: proposal.intent.clone(),
            requested_caps: proposal.requested_caps.clone(),
            evidence: proposal.evidence.clone(),
            decision: decision.clone(),
            policy_hash,
        };

        let receipt = self.append_receipt(
            &mut state,
            &proposal.worldline,
            Receipt::Commitment(commitment),
        )?;

        match receipt {
            Receipt::Commitment(commitment) => Ok(commitment),
            _ => unreachable!("append_commitment always returns a commitment receipt"),
        }
    }

    fn append_outcome(
        &self,
        commitment_receipt_hash: [u8; 32],
        outcome: &OutcomeRecord,
    ) -> Result<OutcomeReceipt, LedgerError> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| LedgerError::IntegrityViolation {
                seq: 0,
                reason: "ledger write lock poisoned".into(),
            })?;

        let commitment = Self::find_commitment_by_hash(&state, commitment_receipt_hash)?;
        if !commitment.decision.is_accepted() {
            return Err(LedgerError::CommitmentNotAccepted);
        }

        let (seq, prev_hash, timestamp) =
            Self::stream_position(&state, &commitment.worldline, self.node_id);

        let outcome_receipt = OutcomeReceipt {
            worldline: commitment.worldline.clone(),
            seq,
            receipt_hash: [0; 32],
            prev_hash,
            timestamp,
            commitment_receipt_hash,
            outcome_hash: outcome.outcome_hash(),
            accepted: true,
            effects: outcome.effects.clone(),
            proofs: outcome.proofs.clone(),
            state_updates: outcome.state_updates.clone(),
            metadata: outcome.metadata.clone(),
        };

        let receipt = self.append_receipt(
            &mut state,
            &commitment.worldline,
            Receipt::Outcome(outcome_receipt),
        )?;

        match receipt {
            Receipt::Outcome(outcome) => Ok(outcome),
            _ => unreachable!("append_outcome always returns an outcome receipt"),
        }
    }

    fn append_rejection_outcome(
        &self,
        commitment_receipt_hash: [u8; 32],
        reason: &str,
    ) -> Result<OutcomeReceipt, LedgerError> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| LedgerError::IntegrityViolation {
                seq: 0,
                reason: "ledger write lock poisoned".into(),
            })?;

        let commitment = Self::find_commitment_by_hash(&state, commitment_receipt_hash)?;
        if !commitment.decision.is_rejected() {
            return Err(LedgerError::CommitmentNotRejected);
        }

        let (seq, prev_hash, timestamp) =
            Self::stream_position(&state, &commitment.worldline, self.node_id);

        let mut metadata = BTreeMap::new();
        metadata.insert("rejection_reason".to_string(), reason.to_string());

        let rejection = OutcomeReceipt {
            worldline: commitment.worldline.clone(),
            seq,
            receipt_hash: [0; 32],
            prev_hash,
            timestamp,
            commitment_receipt_hash,
            outcome_hash: hash_json(&metadata)?,
            accepted: false,
            effects: vec![],
            proofs: vec![],
            state_updates: vec![],
            metadata,
        };

        let receipt = self.append_receipt(
            &mut state,
            &commitment.worldline,
            Receipt::Outcome(rejection),
        )?;

        match receipt {
            Receipt::Outcome(outcome) => Ok(outcome),
            _ => unreachable!("append_rejection_outcome always returns an outcome receipt"),
        }
    }

    fn append_snapshot(&self, snapshot: &SnapshotInput) -> Result<SnapshotReceipt, LedgerError> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| LedgerError::IntegrityViolation {
                seq: 0,
                reason: "ledger write lock poisoned".into(),
            })?;

        let anchor = state
            .hash_index
            .get(&snapshot.anchored_receipt_hash)
            .cloned()
            .ok_or(LedgerError::MissingSnapshotAnchor)?;

        if anchor.0 != snapshot.worldline {
            return Err(LedgerError::MissingSnapshotAnchor);
        }

        let (seq, prev_hash, timestamp) =
            Self::stream_position(&state, &snapshot.worldline, self.node_id);
        let state_hash = hash_json(&snapshot.state)?;

        let snapshot_receipt = SnapshotReceipt {
            worldline: snapshot.worldline.clone(),
            seq,
            receipt_hash: [0; 32],
            prev_hash,
            timestamp,
            anchored_receipt_hash: snapshot.anchored_receipt_hash,
            state_hash,
            state: snapshot.state.clone(),
        };

        let receipt = self.append_receipt(
            &mut state,
            &snapshot.worldline,
            Receipt::Snapshot(snapshot_receipt),
        )?;

        match receipt {
            Receipt::Snapshot(snapshot) => Ok(snapshot),
            _ => unreachable!("append_snapshot always returns a snapshot receipt"),
        }
    }
}

impl LedgerReader for InMemoryLedger {
    fn head(
        &self,
        worldline: &worldline_types::WorldlineId,
    ) -> Result<Option<ReceiptRef>, LedgerError> {
        let state = self
            .inner
            .read()
            .map_err(|_| LedgerError::IntegrityViolation {
                seq: 0,
                reason: "ledger read lock poisoned".into(),
            })?;

        Ok(state
            .streams
            .get(worldline)
            .and_then(|stream| stream.last())
            .map(ReceiptRef::from))
    }

    fn read_range(
        &self,
        worldline: &worldline_types::WorldlineId,
        from_seq: u64,
        to_seq: u64,
    ) -> Result<Vec<Receipt>, LedgerError> {
        if from_seq == 0 || to_seq == 0 || from_seq > to_seq {
            return Err(LedgerError::InvalidRange {
                from: from_seq,
                to: to_seq,
            });
        }

        let state = self
            .inner
            .read()
            .map_err(|_| LedgerError::IntegrityViolation {
                seq: 0,
                reason: "ledger read lock poisoned".into(),
            })?;

        let Some(stream) = state.streams.get(worldline) else {
            return Ok(vec![]);
        };

        let start = (from_seq - 1) as usize;
        if start >= stream.len() {
            return Ok(vec![]);
        }

        let end_exclusive = to_seq.min(stream.len() as u64) as usize;
        Ok(stream[start..end_exclusive].to_vec())
    }

    fn read_all(
        &self,
        worldline: &worldline_types::WorldlineId,
    ) -> Result<Vec<Receipt>, LedgerError> {
        let state = self
            .inner
            .read()
            .map_err(|_| LedgerError::IntegrityViolation {
                seq: 0,
                reason: "ledger read lock poisoned".into(),
            })?;

        Ok(state.streams.get(worldline).cloned().unwrap_or_default())
    }

    fn get_by_hash(&self, hash: [u8; 32]) -> Result<Option<Receipt>, LedgerError> {
        let state = self
            .inner
            .read()
            .map_err(|_| LedgerError::IntegrityViolation {
                seq: 0,
                reason: "ledger read lock poisoned".into(),
            })?;

        let Some((worldline, index)) = state.hash_index.get(&hash) else {
            return Ok(None);
        };

        Ok(state
            .streams
            .get(worldline)
            .and_then(|stream| stream.get(*index))
            .cloned())
    }

    fn worldlines(&self) -> Result<Vec<worldline_types::WorldlineId>, LedgerError> {
        let state = self
            .inner
            .read()
            .map_err(|_| LedgerError::IntegrityViolation {
                seq: 0,
                reason: "ledger read lock poisoned".into(),
            })?;

        let mut ids: Vec<_> = state.streams.keys().cloned().collect();
        ids.sort_by_key(|wid| wid.short_id());
        Ok(ids)
    }
}

fn hash_json<T: serde::Serialize>(value: &T) -> Result<[u8; 32], LedgerError> {
    let encoded =
        serde_json::to_vec(value).map_err(|error| LedgerError::Serialization(error.to_string()))?;
    Ok(*blake3::hash(&encoded).as_bytes())
}

fn recompute_receipt_hash(receipt: &Receipt) -> Result<[u8; 32], LedgerError> {
    let mut canonical = receipt.clone();
    canonical.set_receipt_hash([0; 32]);

    let encoded = serde_json::to_vec(&canonical)
        .map_err(|error| LedgerError::Serialization(error.to_string()))?;

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"worldline-ledger-receipt-v1:");
    hasher.update(&encoded);
    Ok(*hasher.finalize().as_bytes())
}

fn next_anchor(last: Option<&Receipt>, node_id: u16) -> worldline_types::TemporalAnchor {
    let now = worldline_types::TemporalAnchor::now(node_id);
    match last {
        None => now,
        Some(previous) => {
            let prev = previous.timestamp();
            if now.physical_ms > prev.physical_ms {
                worldline_types::TemporalAnchor::new(now.physical_ms, 0, node_id)
            } else {
                worldline_types::TemporalAnchor::new(
                    prev.physical_ms,
                    prev.logical.saturating_add(1),
                    node_id,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use worldline_types::{CommitmentId, IdentityMaterial, WorldlineId};

    fn worldline(seed: u8) -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([seed; 32]))
    }

    fn commitment(worldline: &WorldlineId) -> CommitmentProposal {
        CommitmentProposal {
            worldline: worldline.clone(),
            commitment_id: CommitmentId::new(),
            class: crate::records::CommitmentClass::ExternalIo,
            intent: "synchronize state".into(),
            requested_caps: vec!["cap-sync".into()],
            targets: vec![worldline.clone()],
            evidence: crate::records::EvidenceBundle::from_references(
                vec!["obj://evidence".into()],
            ),
            nonce: 1,
        }
    }

    fn accepted_outcome(key: &str, value: i64) -> OutcomeRecord {
        OutcomeRecord {
            effects: vec![crate::records::EffectSummary {
                kind: "test-effect".into(),
                target: "test-target".into(),
                description: "state update".into(),
            }],
            proofs: vec![],
            state_updates: vec![crate::records::StateUpdate {
                key: key.into(),
                value: Value::from(value),
            }],
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn append_commitment_and_outcome_create_hash_chain() {
        let ledger = InMemoryLedger::default();
        let wid = worldline(1);

        let commitment = ledger
            .append_commitment(&commitment(&wid), &Decision::Accepted, [2; 32])
            .unwrap();

        let outcome = ledger
            .append_outcome(commitment.receipt_hash, &accepted_outcome("balance", 100))
            .unwrap();

        assert_eq!(commitment.seq, 1);
        assert_eq!(outcome.seq, 2);
        assert_eq!(outcome.prev_hash, Some(commitment.receipt_hash));
        ledger.validate_stream(&wid).unwrap();
    }

    #[test]
    fn outcome_without_commitment_is_rejected() {
        let ledger = InMemoryLedger::default();
        let error = ledger
            .append_outcome([7; 32], &accepted_outcome("balance", 1))
            .unwrap_err();
        assert_eq!(error, LedgerError::MissingCommitmentReceipt);
    }

    #[test]
    fn accepted_outcome_requires_accepted_commitment() {
        let ledger = InMemoryLedger::default();
        let wid = worldline(2);

        let rejected = ledger
            .append_commitment(
                &commitment(&wid),
                &Decision::Rejected {
                    reason: "policy denied".into(),
                },
                [5; 32],
            )
            .unwrap();

        let error = ledger
            .append_outcome(rejected.receipt_hash, &accepted_outcome("x", 10))
            .unwrap_err();
        assert_eq!(error, LedgerError::CommitmentNotAccepted);

        let rejection_outcome = ledger
            .append_rejection_outcome(rejected.receipt_hash, "denied by policy")
            .unwrap();
        assert!(!rejection_outcome.accepted);
    }

    #[test]
    fn snapshot_requires_existing_anchor() {
        let ledger = InMemoryLedger::default();
        let wid = worldline(3);

        let mut state = BTreeMap::new();
        state.insert("balance".into(), Value::from(10));

        let error = ledger
            .append_snapshot(&SnapshotInput {
                worldline: wid,
                anchored_receipt_hash: [9; 32],
                state,
            })
            .unwrap_err();

        assert_eq!(error, LedgerError::MissingSnapshotAnchor);
    }

    #[test]
    fn validate_stream_detects_tampering() {
        let ledger = InMemoryLedger::default();
        let wid = worldline(4);

        let commitment = ledger
            .append_commitment(&commitment(&wid), &Decision::Accepted, [1; 32])
            .unwrap();
        ledger
            .append_outcome(commitment.receipt_hash, &accepted_outcome("n", 1))
            .unwrap();

        {
            let mut guard = ledger.inner.write().unwrap();
            let stream = guard.streams.get_mut(&wid).unwrap();
            if let Receipt::Outcome(outcome) = &mut stream[1] {
                outcome.state_updates[0].value = Value::from(999);
            }
        }

        let error = ledger.validate_stream(&wid).unwrap_err();
        assert!(matches!(
            error,
            LedgerError::IntegrityViolation { reason, .. } if reason == "receipt hash mismatch"
        ));
    }

    #[test]
    fn read_range_is_inclusive_and_validated() {
        let ledger = InMemoryLedger::default();
        let wid = worldline(5);

        let commitment = ledger
            .append_commitment(&commitment(&wid), &Decision::Accepted, [3; 32])
            .unwrap();
        ledger
            .append_outcome(commitment.receipt_hash, &accepted_outcome("n", 2))
            .unwrap();

        let range = ledger.read_range(&wid, 1, 2).unwrap();
        assert_eq!(range.len(), 2);

        let error = ledger.read_range(&wid, 3, 2).unwrap_err();
        assert_eq!(error, LedgerError::InvalidRange { from: 3, to: 2 });
    }
}
