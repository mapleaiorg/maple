use std::collections::{BTreeMap, HashMap};

use serde_json::Value;
use worldline_types::{CommitmentId, TemporalAnchor, WorldlineId};

use crate::error::LedgerError;
use crate::records::{Receipt, ReceiptKind, ReceiptRef};
use crate::traits::LedgerReader;

/// Read model: latest worldline state reconstructed from receipts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LatestStateProjection {
    pub worldline: WorldlineId,
    pub head: Option<ReceiptRef>,
    pub latest_commitment: Option<CommitmentId>,
    pub trajectory_length: u64,
    pub last_updated: Option<TemporalAnchor>,
    pub state: BTreeMap<String, Value>,
}

/// Read model row for compliance/audit workflows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditIndexEntry {
    pub seq: u64,
    pub receipt_hash: [u8; 32],
    pub kind: ReceiptKind,
    pub timestamp: TemporalAnchor,
    pub commitment_id: Option<CommitmentId>,
    pub accepted: Option<bool>,
    pub summary: String,
}

/// Read model: immutable sequence of receipt summaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditIndexProjection {
    pub worldline: WorldlineId,
    pub entries: Vec<AuditIndexEntry>,
}

/// Deterministic projection builders.
pub struct ProjectionBuilder;

impl ProjectionBuilder {
    pub fn latest_state<R: LedgerReader>(
        reader: &R,
        worldline: &WorldlineId,
    ) -> Result<LatestStateProjection, LedgerError> {
        let receipts = reader.read_all(worldline)?;
        let mut state = BTreeMap::new();
        let mut latest_commitment = None;
        let mut last_updated = None;

        for receipt in &receipts {
            match receipt {
                Receipt::Commitment(commitment) => {
                    latest_commitment = Some(commitment.commitment_id.clone());
                }
                Receipt::Outcome(outcome) => {
                    if outcome.accepted {
                        for update in &outcome.state_updates {
                            state.insert(update.key.clone(), update.value.clone());
                        }
                    }
                }
                Receipt::Snapshot(snapshot) => {
                    state = snapshot.state.clone();
                }
            }
            last_updated = Some(receipt.timestamp());
        }

        Ok(LatestStateProjection {
            worldline: worldline.clone(),
            head: receipts.last().map(ReceiptRef::from),
            latest_commitment,
            trajectory_length: receipts.len() as u64,
            last_updated,
            state,
        })
    }

    pub fn audit_index<R: LedgerReader>(
        reader: &R,
        worldline: &WorldlineId,
    ) -> Result<AuditIndexProjection, LedgerError> {
        let receipts = reader.read_all(worldline)?;
        let mut commitment_by_hash = HashMap::new();

        for receipt in &receipts {
            if let Receipt::Commitment(commitment) = receipt {
                commitment_by_hash
                    .insert(commitment.receipt_hash, commitment.commitment_id.clone());
            }
        }

        let entries = receipts
            .iter()
            .map(|receipt| match receipt {
                Receipt::Commitment(commitment) => AuditIndexEntry {
                    seq: commitment.seq,
                    receipt_hash: commitment.receipt_hash,
                    kind: ReceiptKind::Commitment,
                    timestamp: commitment.timestamp,
                    commitment_id: Some(commitment.commitment_id.clone()),
                    accepted: Some(commitment.decision.is_accepted()),
                    summary: commitment.intent.clone(),
                },
                Receipt::Outcome(outcome) => AuditIndexEntry {
                    seq: outcome.seq,
                    receipt_hash: outcome.receipt_hash,
                    kind: ReceiptKind::Outcome,
                    timestamp: outcome.timestamp,
                    commitment_id: commitment_by_hash
                        .get(&outcome.commitment_receipt_hash)
                        .cloned(),
                    accepted: Some(outcome.accepted),
                    summary: if outcome.accepted {
                        format!(
                            "{} effect(s), {} proof(s)",
                            outcome.effects.len(),
                            outcome.proofs.len()
                        )
                    } else {
                        "rejected outcome".into()
                    },
                },
                Receipt::Snapshot(snapshot) => AuditIndexEntry {
                    seq: snapshot.seq,
                    receipt_hash: snapshot.receipt_hash,
                    kind: ReceiptKind::Snapshot,
                    timestamp: snapshot.timestamp,
                    commitment_id: None,
                    accepted: None,
                    summary: format!(
                        "snapshot anchored at {}",
                        short_hash(snapshot.anchored_receipt_hash)
                    ),
                },
            })
            .collect();

        Ok(AuditIndexProjection {
            worldline: worldline.clone(),
            entries,
        })
    }
}

fn short_hash(hash: [u8; 32]) -> String {
    hash[..6].iter().map(|b| format!("{b:02x}")).collect()
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

    use super::*;

    fn worldline(seed: u8) -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([seed; 32]))
    }

    fn proposal(worldline: &WorldlineId) -> CommitmentProposal {
        CommitmentProposal {
            worldline: worldline.clone(),
            commitment_id: CommitmentId::new(),
            class: CommitmentClass::ExternalIo,
            intent: "projection test".into(),
            requested_caps: vec!["cap-test".into()],
            targets: vec![worldline.clone()],
            evidence: EvidenceBundle::from_references(vec!["obj://proof".into()]),
            nonce: 9,
        }
    }

    fn outcome(key: &str, value: i64) -> OutcomeRecord {
        OutcomeRecord {
            effects: vec![],
            proofs: vec![],
            state_updates: vec![StateUpdate {
                key: key.to_string(),
                value: Value::from(value),
            }],
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn latest_state_projection_is_deterministic() {
        let ledger = InMemoryLedger::default();
        let wid = worldline(1);

        let commitment = ledger
            .append_commitment(&proposal(&wid), &Decision::Accepted, [1; 32])
            .unwrap();
        let outcome_receipt = ledger
            .append_outcome(commitment.receipt_hash, &outcome("balance", 40))
            .unwrap();

        let mut snap_state = BTreeMap::new();
        snap_state.insert("balance".into(), Value::from(40));
        ledger
            .append_snapshot(&SnapshotInput {
                worldline: wid.clone(),
                anchored_receipt_hash: outcome_receipt.receipt_hash,
                state: snap_state,
            })
            .unwrap();

        let first = ProjectionBuilder::latest_state(&ledger, &wid).unwrap();
        let second = ProjectionBuilder::latest_state(&ledger, &wid).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.state.get("balance"), Some(&Value::from(40)));
    }

    #[test]
    fn audit_index_projection_contains_all_receipts() {
        let ledger = InMemoryLedger::default();
        let wid = worldline(2);

        let commitment = ledger
            .append_commitment(&proposal(&wid), &Decision::Accepted, [2; 32])
            .unwrap();
        ledger
            .append_outcome(commitment.receipt_hash, &outcome("x", 1))
            .unwrap();

        let projection = ProjectionBuilder::audit_index(&ledger, &wid).unwrap();
        assert_eq!(projection.entries.len(), 2);
        assert_eq!(projection.entries[0].kind, ReceiptKind::Commitment);
        assert_eq!(projection.entries[1].kind, ReceiptKind::Outcome);
    }
}
