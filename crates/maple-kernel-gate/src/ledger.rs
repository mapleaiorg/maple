use maple_mwl_types::{
    CommitmentId, CommitmentStatus, DenialReason, FailureReason, PolicyDecisionCard,
    TemporalAnchor, WorldlineId,
};
use serde::{Deserialize, Serialize};

use crate::declaration::CommitmentDeclaration;
use crate::error::LedgerError;

/// Lifecycle events tracked for each commitment.
///
/// Per Whitepaper §6.6: Denied commitments, failed executions, and expired
/// obligations are all recorded. Accountability is preserved regardless of outcome.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LifecycleEvent {
    Declared(TemporalAnchor),
    Approved(TemporalAnchor),
    Denied {
        at: TemporalAnchor,
        reason: DenialReason,
    },
    ExecutionStarted(TemporalAnchor),
    Fulfilled(TemporalAnchor),
    Failed {
        at: TemporalAnchor,
        reason: FailureReason,
    },
    Expired(TemporalAnchor),
    Revoked {
        at: TemporalAnchor,
        by: WorldlineId,
        reason: String,
    },
}

/// A single entry in the Commitment Ledger.
///
/// Per I.CG-1 (Decision Immutability): PolicyDecisionCards are immutable once recorded.
/// The declaration and decision fields are set at creation and never modified.
/// Only lifecycle events can be appended.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub commitment_id: CommitmentId,
    pub declaration: CommitmentDeclaration,
    pub decision: PolicyDecisionCard,
    pub lifecycle: Vec<LifecycleEvent>,
    pub created_at: TemporalAnchor,
}

/// Filter for querying the ledger.
#[derive(Clone, Debug, Default)]
pub struct LedgerFilter {
    pub worldline_id: Option<WorldlineId>,
    pub status: Option<CommitmentStatus>,
    pub time_range: Option<(TemporalAnchor, TemporalAnchor)>,
}

impl LedgerFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_worldline(mut self, wid: WorldlineId) -> Self {
        self.worldline_id = Some(wid);
        self
    }

    pub fn with_status(mut self, status: CommitmentStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_time_range(mut self, from: TemporalAnchor, to: TemporalAnchor) -> Self {
        self.time_range = Some((from, to));
        self
    }

    /// Check if a ledger entry matches this filter.
    pub fn matches(&self, entry: &LedgerEntry) -> bool {
        if let Some(ref wid) = self.worldline_id {
            if entry.declaration.declaring_identity != *wid {
                return false;
            }
        }

        if let Some((ref from, ref to)) = self.time_range {
            if entry.created_at < *from || entry.created_at > *to {
                return false;
            }
        }

        if let Some(ref status) = self.status {
            let entry_status = derive_status(entry);
            if entry_status != *status {
                return false;
            }
        }

        true
    }
}

/// Derive the current status of a commitment from its lifecycle events.
fn derive_status(entry: &LedgerEntry) -> CommitmentStatus {
    // Most recent event determines status
    if let Some(last) = entry.lifecycle.last() {
        match last {
            LifecycleEvent::Declared(_) => CommitmentStatus::Pending,
            LifecycleEvent::Approved(_) => CommitmentStatus::Approved,
            LifecycleEvent::Denied { reason, .. } => CommitmentStatus::Denied(reason.clone()),
            LifecycleEvent::ExecutionStarted(_) => CommitmentStatus::Active,
            LifecycleEvent::Fulfilled(_) => CommitmentStatus::Fulfilled,
            LifecycleEvent::Failed { reason, .. } => CommitmentStatus::Failed(reason.clone()),
            LifecycleEvent::Expired(_) => CommitmentStatus::Expired,
            LifecycleEvent::Revoked { by, reason, .. } => CommitmentStatus::Revoked {
                by: by.clone(),
                reason: reason.clone(),
            },
        }
    } else {
        CommitmentStatus::Pending
    }
}

/// Commitment Ledger — append-only, immutable record of ALL commitments.
///
/// Per I.AAS-3 (Ledger Immutability): This ledger is append-only.
/// No delete or modify operations exist.
///
/// Per Whitepaper §6.6: "The ledger records denied Commitments, failed executions,
/// expired obligations. Accountability is preserved regardless of outcome."
pub struct CommitmentLedger {
    entries: Vec<LedgerEntry>,
}

impl CommitmentLedger {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Append a new ledger entry. APPEND-ONLY — no modification or deletion.
    ///
    /// Returns error if a commitment with the same ID already exists.
    pub fn append(&mut self, entry: LedgerEntry) -> Result<(), LedgerError> {
        if self
            .entries
            .iter()
            .any(|e| e.commitment_id == entry.commitment_id)
        {
            return Err(LedgerError::DuplicateEntry(entry.commitment_id));
        }

        self.entries.push(entry);
        Ok(())
    }

    /// Record a lifecycle event for an existing commitment.
    ///
    /// This only appends to the lifecycle vec — it does NOT modify the
    /// declaration or decision fields (I.CG-1: Decision Immutability).
    pub fn record_lifecycle(
        &mut self,
        cid: &CommitmentId,
        event: LifecycleEvent,
    ) -> Result<(), LedgerError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.commitment_id == *cid)
            .ok_or_else(|| LedgerError::NotFound(cid.clone()))?;

        entry.lifecycle.push(event);
        Ok(())
    }

    /// Query ledger entries matching a filter.
    pub fn query(&self, filter: &LedgerFilter) -> Vec<&LedgerEntry> {
        self.entries.iter().filter(|e| filter.matches(e)).collect()
    }

    /// Get full history for a specific commitment.
    pub fn history(&self, cid: &CommitmentId) -> Option<&LedgerEntry> {
        self.entries.iter().find(|e| e.commitment_id == *cid)
    }

    /// Number of entries in the ledger.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the ledger is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for CommitmentLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declaration::CommitmentDeclaration;
    use maple_mwl_types::{
        AdjudicationDecision, CommitmentScope, EffectDomain, IdentityMaterial, RiskClass, RiskLevel,
    };

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn test_scope() -> CommitmentScope {
        CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![test_worldline()],
            constraints: vec![],
        }
    }

    fn test_decision(approved: bool) -> PolicyDecisionCard {
        PolicyDecisionCard {
            decision_id: uuid::Uuid::new_v4().to_string(),
            decision: if approved {
                AdjudicationDecision::Approve
            } else {
                AdjudicationDecision::Deny
            },
            rationale: if approved {
                "Approved".into()
            } else {
                "Denied".into()
            },
            risk: RiskLevel {
                class: RiskClass::Low,
                score: Some(0.1),
                factors: vec![],
            },
            conditions: vec![],
            policy_refs: vec![],
            decided_at: TemporalAnchor::now(0),
            version: 1,
        }
    }

    fn create_entry(approved: bool) -> LedgerEntry {
        let decl = CommitmentDeclaration::builder(test_worldline(), test_scope()).build();
        let decision = test_decision(approved);
        let lifecycle = if approved {
            vec![
                LifecycleEvent::Declared(TemporalAnchor::now(0)),
                LifecycleEvent::Approved(TemporalAnchor::now(0)),
            ]
        } else {
            vec![
                LifecycleEvent::Declared(TemporalAnchor::now(0)),
                LifecycleEvent::Denied {
                    at: TemporalAnchor::now(0),
                    reason: DenialReason {
                        code: "TEST".into(),
                        message: "test denial".into(),
                        policy_refs: vec![],
                    },
                },
            ]
        };

        LedgerEntry {
            commitment_id: decl.id.clone(),
            declaration: decl,
            decision,
            lifecycle,
            created_at: TemporalAnchor::now(0),
        }
    }

    #[test]
    fn append_creates_entry() {
        let mut ledger = CommitmentLedger::new();
        let entry = create_entry(true);
        let cid = entry.commitment_id.clone();
        ledger.append(entry).unwrap();

        assert_eq!(ledger.len(), 1);
        assert!(ledger.history(&cid).is_some());
    }

    #[test]
    fn duplicate_entry_rejected() {
        let mut ledger = CommitmentLedger::new();
        let entry = create_entry(true);
        let dup = entry.clone();
        ledger.append(entry).unwrap();
        assert!(ledger.append(dup).is_err());
    }

    #[test]
    fn record_lifecycle_appends() {
        let mut ledger = CommitmentLedger::new();
        let entry = create_entry(true);
        let cid = entry.commitment_id.clone();
        ledger.append(entry).unwrap();

        let before = ledger.history(&cid).unwrap().lifecycle.len();
        ledger
            .record_lifecycle(
                &cid,
                LifecycleEvent::ExecutionStarted(TemporalAnchor::now(0)),
            )
            .unwrap();
        let after = ledger.history(&cid).unwrap().lifecycle.len();
        assert_eq!(after, before + 1);
    }

    #[test]
    fn record_lifecycle_nonexistent_fails() {
        let mut ledger = CommitmentLedger::new();
        let fake_cid = CommitmentId::new();
        assert!(ledger
            .record_lifecycle(&fake_cid, LifecycleEvent::Fulfilled(TemporalAnchor::now(0)))
            .is_err());
    }

    #[test]
    fn denied_commitments_are_first_class() {
        let mut ledger = CommitmentLedger::new();
        let denied_entry = create_entry(false);
        let cid = denied_entry.commitment_id.clone();
        ledger.append(denied_entry).unwrap();

        // Denied entry exists and is queryable
        let entry = ledger.history(&cid).unwrap();
        assert_eq!(entry.decision.decision, AdjudicationDecision::Deny);
        assert!(matches!(
            entry.lifecycle.last().unwrap(),
            LifecycleEvent::Denied { .. }
        ));
    }

    #[test]
    fn query_by_worldline() {
        let mut ledger = CommitmentLedger::new();
        ledger.append(create_entry(true)).unwrap();
        ledger.append(create_entry(true)).unwrap();

        let filter = LedgerFilter::new().with_worldline(test_worldline());
        let results = ledger.query(&filter);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn query_by_status() {
        let mut ledger = CommitmentLedger::new();
        ledger.append(create_entry(true)).unwrap();
        ledger.append(create_entry(false)).unwrap();

        let filter = LedgerFilter::new().with_status(CommitmentStatus::Approved);
        let approved = ledger.query(&filter);
        assert_eq!(approved.len(), 1);
    }

    #[test]
    fn ledger_is_append_only_no_delete_or_modify() {
        // This test documents the invariant: there are NO delete or modify methods
        // on CommitmentLedger. The only mutations are:
        // - append(): add new entry
        // - record_lifecycle(): append lifecycle event to existing entry
        //
        // This satisfies I.AAS-3 (Ledger Immutability).
        let mut ledger = CommitmentLedger::new();
        let entry = create_entry(true);
        let cid = entry.commitment_id.clone();
        ledger.append(entry).unwrap();

        // The decision field of a ledger entry is immutable after creation.
        // record_lifecycle only appends to the lifecycle vec.
        let original_decision_id = ledger.history(&cid).unwrap().decision.decision_id.clone();

        ledger
            .record_lifecycle(&cid, LifecycleEvent::Fulfilled(TemporalAnchor::now(0)))
            .unwrap();

        // Decision is unchanged
        assert_eq!(
            ledger.history(&cid).unwrap().decision.decision_id,
            original_decision_id
        );
    }
}
