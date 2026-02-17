//! Canonical WorldLine Ledger (WLL) interfaces and replay/projection primitives.
//!
//! This crate provides:
//! - append-only reader/writer trait boundaries for commitment/outcome receipts
//! - an in-memory ledger implementation for demos, tests, and local runtime use
//! - deterministic projection builders (`latest_state`, `audit_index`)
//! - replay helpers from genesis and snapshot anchors
//!
//! Compatibility exports from legacy kernel crates remain available under
//! `worldline_ledger::provenance`.

pub mod error;
pub mod memory;
pub mod projection;
pub mod records;
pub mod replay;
pub mod traits;

pub use error::LedgerError;
pub use memory::InMemoryLedger;
pub use projection::{
    AuditIndexEntry, AuditIndexProjection, LatestStateProjection, ProjectionBuilder,
};
pub use records::{
    CommitmentClass, CommitmentProposal, CommitmentReceipt, Decision, EffectSummary,
    EvidenceBundle, OutcomeReceipt, OutcomeRecord, ProofRef, Receipt, ReceiptKind, ReceiptRef,
    SnapshotInput, SnapshotReceipt, StateUpdate,
};
pub use replay::{ReplayEngine, ReplayResult};
pub use traits::{LedgerReader, LedgerWriter};

// Compatibility re-exports.
pub use maple_kernel_fabric as fabric;
pub use maple_kernel_provenance as provenance;
pub use maple_mwl_types as types;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::Value;
    use worldline_types::{CommitmentId, IdentityMaterial, WorldlineId};

    use super::{
        CommitmentClass, CommitmentProposal, Decision, EvidenceBundle, InMemoryLedger,
        OutcomeRecord, ProjectionBuilder, ReplayEngine, StateUpdate,
    };
    use crate::traits::LedgerWriter;

    fn worldline(seed: u8) -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([seed; 32]))
    }

    fn proposal(worldline: &WorldlineId) -> CommitmentProposal {
        CommitmentProposal {
            worldline: worldline.clone(),
            commitment_id: CommitmentId::new(),
            class: CommitmentClass::ExternalIo,
            intent: "lib integration test".into(),
            requested_caps: vec!["cap-demo".into()],
            targets: vec![worldline.clone()],
            evidence: EvidenceBundle::from_references(vec!["obj://demo".into()]),
            nonce: 1,
        }
    }

    #[test]
    fn ledger_api_builds_and_replays_state() {
        let ledger = InMemoryLedger::default();
        let wid = worldline(11);

        let commitment = ledger
            .append_commitment(&proposal(&wid), &Decision::Accepted, [1; 32])
            .unwrap();

        let mut metadata = BTreeMap::new();
        metadata.insert("source".to_string(), "test".to_string());

        let outcome = OutcomeRecord {
            effects: vec![],
            proofs: vec![],
            state_updates: vec![StateUpdate {
                key: "balance".into(),
                value: Value::from(1250),
            }],
            metadata,
        };

        ledger
            .append_outcome(commitment.receipt_hash, &outcome)
            .unwrap();

        let latest = ProjectionBuilder::latest_state(&ledger, &wid).unwrap();
        let replayed = ReplayEngine::replay_from_genesis(&ledger, &wid).unwrap();

        assert_eq!(latest.state, replayed.state);
        assert_eq!(latest.state.get("balance"), Some(&Value::from(1250)));
    }

    #[test]
    fn facade_exports_provenance_index() {
        let index = super::provenance::ProvenanceIndex::new();
        assert_eq!(index.len(), 0);
    }
}
