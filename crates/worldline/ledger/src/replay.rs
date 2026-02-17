use std::collections::BTreeMap;

use serde_json::Value;
use worldline_types::WorldlineId;

use crate::error::LedgerError;
use crate::records::{Receipt, SnapshotReceipt};
use crate::traits::LedgerReader;

/// Result of replaying a worldline stream into canonical state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayResult {
    pub worldline: WorldlineId,
    pub applied_outcomes: u64,
    pub evaluated_receipts: u64,
    pub state: BTreeMap<String, Value>,
}

/// Deterministic replay helpers for WLL streams.
pub struct ReplayEngine;

impl ReplayEngine {
    pub fn replay_from_genesis<R: LedgerReader>(
        reader: &R,
        worldline: &WorldlineId,
    ) -> Result<ReplayResult, LedgerError> {
        let receipts = reader.read_all(worldline)?;
        Ok(apply_receipts(
            worldline.clone(),
            BTreeMap::new(),
            &receipts,
            0,
        ))
    }

    pub fn replay_from_snapshot<R: LedgerReader>(
        reader: &R,
        snapshot: &SnapshotReceipt,
    ) -> Result<ReplayResult, LedgerError> {
        let receipts = reader.read_all(&snapshot.worldline)?;
        let anchor_index = receipts
            .iter()
            .position(|receipt| receipt.receipt_hash() == snapshot.anchored_receipt_hash)
            .ok_or(LedgerError::MissingSnapshotAnchor)?;

        Ok(apply_receipts(
            snapshot.worldline.clone(),
            snapshot.state.clone(),
            &receipts,
            anchor_index + 1,
        ))
    }

    pub fn verify_snapshot_convergence<R: LedgerReader>(
        reader: &R,
        snapshot: &SnapshotReceipt,
    ) -> Result<bool, LedgerError> {
        let full = Self::replay_from_genesis(reader, &snapshot.worldline)?;
        let tail = Self::replay_from_snapshot(reader, snapshot)?;
        Ok(full.state == tail.state)
    }
}

fn apply_receipts(
    worldline: WorldlineId,
    mut state: BTreeMap<String, Value>,
    receipts: &[Receipt],
    start_index: usize,
) -> ReplayResult {
    let mut applied_outcomes = 0u64;
    let mut evaluated_receipts = 0u64;

    for receipt in receipts.iter().skip(start_index) {
        evaluated_receipts += 1;
        match receipt {
            Receipt::Outcome(outcome) => {
                if outcome.accepted {
                    for update in &outcome.state_updates {
                        state.insert(update.key.clone(), update.value.clone());
                    }
                    applied_outcomes += 1;
                }
            }
            Receipt::Snapshot(snapshot) => {
                state = snapshot.state.clone();
            }
            Receipt::Commitment(_) => {}
        }
    }

    ReplayResult {
        worldline,
        applied_outcomes,
        evaluated_receipts,
        state,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::Value;
    use worldline_types::{CommitmentId, IdentityMaterial, WorldlineId};

    use crate::memory::InMemoryLedger;
    use crate::records::{
        CommitmentClass, CommitmentProposal, Decision, EvidenceBundle, OutcomeRecord,
        SnapshotInput, StateUpdate,
    };
    use crate::traits::LedgerWriter;

    use super::ReplayEngine;

    fn worldline(seed: u8) -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([seed; 32]))
    }

    fn proposal(worldline: &WorldlineId, nonce: u64) -> CommitmentProposal {
        CommitmentProposal {
            worldline: worldline.clone(),
            commitment_id: CommitmentId::new(),
            class: CommitmentClass::ExternalIo,
            intent: "replay test".into(),
            requested_caps: vec!["cap-test".into()],
            targets: vec![worldline.clone()],
            evidence: EvidenceBundle::from_references(vec!["obj://proof".into()]),
            nonce,
        }
    }

    fn outcome(value: i64) -> OutcomeRecord {
        OutcomeRecord {
            effects: vec![],
            proofs: vec![],
            state_updates: vec![StateUpdate {
                key: "balance".into(),
                value: Value::from(value),
            }],
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn replay_from_snapshot_matches_full_replay() {
        let ledger = InMemoryLedger::default();
        let wid = worldline(7);

        let c1 = ledger
            .append_commitment(&proposal(&wid, 1), &Decision::Accepted, [1; 32])
            .unwrap();
        let o1 = ledger
            .append_outcome(c1.receipt_hash, &outcome(10))
            .unwrap();

        let mut snapshot_state = BTreeMap::new();
        snapshot_state.insert("balance".into(), Value::from(10));

        let snapshot = ledger
            .append_snapshot(&SnapshotInput {
                worldline: wid.clone(),
                anchored_receipt_hash: o1.receipt_hash,
                state: snapshot_state,
            })
            .unwrap();

        let c2 = ledger
            .append_commitment(&proposal(&wid, 2), &Decision::Accepted, [2; 32])
            .unwrap();
        ledger
            .append_outcome(c2.receipt_hash, &outcome(25))
            .unwrap();

        let full = ReplayEngine::replay_from_genesis(&ledger, &wid).unwrap();
        let from_snapshot = ReplayEngine::replay_from_snapshot(&ledger, &snapshot).unwrap();

        assert_eq!(full.state, from_snapshot.state);
        assert_eq!(full.state.get("balance"), Some(&Value::from(25)));
        assert!(ReplayEngine::verify_snapshot_convergence(&ledger, &snapshot).unwrap());
    }
}
