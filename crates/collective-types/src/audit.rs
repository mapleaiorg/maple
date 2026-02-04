//! Audit journal types: collective receipts, disputes, and sanctions
//!
//! The audit journal is the collective's accountability record.
//! Every significant action produces a receipt. Disputes and sanctions
//! are tracked for transparency.

use crate::{CollectiveId, ReceiptType};
use chrono::{DateTime, Utc};
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A receipt issued by or about a Collective
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectiveReceipt {
    /// Unique receipt identifier
    pub receipt_id: String,
    /// The collective that issued the receipt
    pub collective_id: CollectiveId,
    /// Type of receipt
    pub receipt_type: ReceiptType,
    /// The actor who triggered the receipt
    pub actor: ResonatorId,
    /// Human-readable description
    pub description: String,
    /// When the receipt was created
    pub timestamp: DateTime<Utc>,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl CollectiveReceipt {
    pub fn new(
        collective_id: CollectiveId,
        receipt_type: ReceiptType,
        actor: ResonatorId,
        description: impl Into<String>,
    ) -> Self {
        Self {
            receipt_id: uuid::Uuid::new_v4().to_string(),
            collective_id,
            receipt_type,
            actor,
            description: description.into(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// A dispute record within a Collective
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisputeRecord {
    /// Unique dispute identifier
    pub dispute_id: String,
    /// Who filed the dispute
    pub complainant: ResonatorId,
    /// Who the dispute is against
    pub respondent: ResonatorId,
    /// Description of the dispute
    pub description: String,
    /// Current status
    pub status: DisputeStatus,
    /// When the dispute was filed
    pub filed_at: DateTime<Utc>,
    /// When the dispute was resolved (if resolved)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<DateTime<Utc>>,
    /// Resolution description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
}

impl DisputeRecord {
    pub fn new(
        complainant: ResonatorId,
        respondent: ResonatorId,
        description: impl Into<String>,
    ) -> Self {
        Self {
            dispute_id: uuid::Uuid::new_v4().to_string(),
            complainant,
            respondent,
            description: description.into(),
            status: DisputeStatus::Filed,
            filed_at: Utc::now(),
            resolved_at: None,
            resolution: None,
        }
    }

    /// Resolve the dispute
    pub fn resolve(&mut self, resolution: impl Into<String>) {
        self.status = DisputeStatus::Resolved;
        self.resolved_at = Some(Utc::now());
        self.resolution = Some(resolution.into());
    }

    /// Escalate the dispute
    pub fn escalate(&mut self) {
        self.status = DisputeStatus::Escalated;
    }

    /// Dismiss the dispute
    pub fn dismiss(&mut self, reason: impl Into<String>) {
        self.status = DisputeStatus::Dismissed;
        self.resolved_at = Some(Utc::now());
        self.resolution = Some(reason.into());
    }

    pub fn is_open(&self) -> bool {
        matches!(
            self.status,
            DisputeStatus::Filed | DisputeStatus::UnderReview
        )
    }
}

/// Status of a dispute
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DisputeStatus {
    /// Dispute has been filed
    #[default]
    Filed,
    /// Dispute is under review
    UnderReview,
    /// Dispute has been resolved
    Resolved,
    /// Dispute has been escalated to a higher authority
    Escalated,
    /// Dispute was dismissed
    Dismissed,
}

/// A sanction record against a member
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SanctionRecord {
    /// Unique sanction identifier
    pub sanction_id: String,
    /// Who is being sanctioned
    pub target: ResonatorId,
    /// Type of sanction
    pub sanction_type: SanctionType,
    /// Reason for the sanction
    pub reason: String,
    /// When the sanction was issued
    pub issued_at: DateTime<Utc>,
    /// When the sanction expires (None = permanent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl SanctionRecord {
    pub fn new(
        target: ResonatorId,
        sanction_type: SanctionType,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            sanction_id: uuid::Uuid::new_v4().to_string(),
            target,
            sanction_type,
            reason: reason.into(),
            issued_at: Utc::now(),
            expires_at: None,
        }
    }

    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Check if the sanction is still active
    pub fn is_active(&self) -> bool {
        match self.expires_at {
            Some(expiry) => Utc::now() < expiry,
            None => true, // No expiry = permanent
        }
    }
}

/// Types of sanctions
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SanctionType {
    /// Formal warning
    Warning,
    /// Temporary suspension from the collective
    Suspension,
    /// Permanent expulsion
    Expulsion,
    /// Financial penalty
    FinancialPenalty(u64),
    /// Custom sanction
    Custom(String),
}

/// The complete audit journal for a Collective
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditJournal {
    /// The collective this journal belongs to
    pub collective_id: CollectiveId,
    /// All receipts
    pub receipts: Vec<CollectiveReceipt>,
    /// All disputes
    pub disputes: Vec<DisputeRecord>,
    /// All sanctions
    pub sanctions: Vec<SanctionRecord>,
}

impl AuditJournal {
    /// Create a new empty audit journal
    pub fn new(collective_id: CollectiveId) -> Self {
        Self {
            collective_id,
            receipts: Vec::new(),
            disputes: Vec::new(),
            sanctions: Vec::new(),
        }
    }

    /// Log a receipt
    pub fn log_receipt(&mut self, receipt: CollectiveReceipt) {
        self.receipts.push(receipt);
    }

    /// File a dispute
    pub fn file_dispute(&mut self, dispute: DisputeRecord) -> String {
        let id = dispute.dispute_id.clone();
        self.disputes.push(dispute);
        id
    }

    /// Issue a sanction
    pub fn issue_sanction(&mut self, sanction: SanctionRecord) -> String {
        let id = sanction.sanction_id.clone();
        self.sanctions.push(sanction);
        id
    }

    /// Get all receipts for a specific actor
    pub fn receipts_for_actor(&self, actor: &ResonatorId) -> Vec<&CollectiveReceipt> {
        self.receipts.iter().filter(|r| r.actor == *actor).collect()
    }

    /// Get all open disputes
    pub fn open_disputes(&self) -> Vec<&DisputeRecord> {
        self.disputes.iter().filter(|d| d.is_open()).collect()
    }

    /// Get all active sanctions for a target
    pub fn active_sanctions_for(&self, target: &ResonatorId) -> Vec<&SanctionRecord> {
        self.sanctions
            .iter()
            .filter(|s| s.target == *target && s.is_active())
            .collect()
    }

    /// Total number of receipts
    pub fn receipt_count(&self) -> usize {
        self.receipts.len()
    }

    /// Get a dispute by ID
    pub fn get_dispute_mut(&mut self, dispute_id: &str) -> Option<&mut DisputeRecord> {
        self.disputes.iter_mut().find(|d| d.dispute_id == dispute_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_journal() -> AuditJournal {
        AuditJournal::new(CollectiveId::new("test-collective"))
    }

    #[test]
    fn test_log_receipt() {
        let mut journal = make_journal();
        let receipt = CollectiveReceipt::new(
            CollectiveId::new("test-collective"),
            ReceiptType::CommitmentFulfilled,
            ResonatorId::new("res-1"),
            "Fulfilled order #123",
        )
        .with_metadata("order_id", "123");

        journal.log_receipt(receipt);
        assert_eq!(journal.receipt_count(), 1);

        let actor_receipts = journal.receipts_for_actor(&ResonatorId::new("res-1"));
        assert_eq!(actor_receipts.len(), 1);
        assert_eq!(actor_receipts[0].metadata.get("order_id").unwrap(), "123");
    }

    #[test]
    fn test_dispute_lifecycle() {
        let mut journal = make_journal();
        let dispute = DisputeRecord::new(
            ResonatorId::new("complainant"),
            ResonatorId::new("respondent"),
            "Failed to deliver on commitment",
        );

        let dispute_id = journal.file_dispute(dispute);
        assert_eq!(journal.open_disputes().len(), 1);

        let dispute = journal.get_dispute_mut(&dispute_id).unwrap();
        assert!(dispute.is_open());

        dispute.resolve("Respondent compensated complainant");
        assert!(!dispute.is_open());
        assert_eq!(journal.open_disputes().len(), 0);
    }

    #[test]
    fn test_sanction() {
        let mut journal = make_journal();
        let sanction = SanctionRecord::new(
            ResonatorId::new("res-bad"),
            SanctionType::Warning,
            "Violated communication policy",
        );

        journal.issue_sanction(sanction);
        let active = journal.active_sanctions_for(&ResonatorId::new("res-bad"));
        assert_eq!(active.len(), 1);

        // No sanctions for other members
        let other = journal.active_sanctions_for(&ResonatorId::new("res-good"));
        assert_eq!(other.len(), 0);
    }

    #[test]
    fn test_sanction_with_expiry() {
        let expired = SanctionRecord::new(
            ResonatorId::new("res-1"),
            SanctionType::Suspension,
            "Temporary ban",
        )
        .with_expiry(Utc::now() - chrono::Duration::hours(1));

        assert!(!expired.is_active());

        let active = SanctionRecord::new(
            ResonatorId::new("res-1"),
            SanctionType::Suspension,
            "Active ban",
        )
        .with_expiry(Utc::now() + chrono::Duration::hours(24));

        assert!(active.is_active());

        let permanent = SanctionRecord::new(
            ResonatorId::new("res-1"),
            SanctionType::Expulsion,
            "Permanent ban",
        );
        assert!(permanent.is_active());
    }

    #[test]
    fn test_dispute_escalate_dismiss() {
        let mut dispute = DisputeRecord::new(
            ResonatorId::new("a"),
            ResonatorId::new("b"),
            "Test dispute",
        );

        dispute.escalate();
        assert_eq!(dispute.status, DisputeStatus::Escalated);
        assert!(!dispute.is_open());

        let mut dispute2 = DisputeRecord::new(
            ResonatorId::new("c"),
            ResonatorId::new("d"),
            "Frivolous dispute",
        );
        dispute2.dismiss("No grounds for dispute");
        assert_eq!(dispute2.status, DisputeStatus::Dismissed);
        assert!(dispute2.resolution.is_some());
    }
}
