//! Threshold Engine — manages collective decision-making
//!
//! The threshold engine coordinates multi-party approval for collective
//! actions. It manages the lifecycle of threshold commitments from
//! creation through signature collection to satisfaction or expiry.

use chrono::{DateTime, Utc};
use collective_types::{
    AuditJournal, CollectiveError, CollectiveId, CollectiveReceipt, CollectiveResult,
    CommitmentSignature, ReceiptRequirement, ReceiptType, ThresholdCommitment,
    ThresholdCommitmentId, ThresholdCommitmentState, ThresholdPolicy,
};
use resonator_types::ResonatorId;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Result of adding a signature
#[derive(Clone, Debug)]
pub enum SignatureResult {
    /// Signature accepted, but threshold not yet met
    Accepted { signatures_so_far: usize },
    /// Signature accepted AND threshold is now satisfied
    ThresholdMet,
    /// Commitment already satisfied
    AlreadySatisfied,
    /// Commitment expired
    Expired,
    /// Commitment rejected
    Rejected,
}

/// The Threshold Engine — coordinates collective approval
pub struct ThresholdEngine {
    /// Active threshold commitments
    commitments: HashMap<ThresholdCommitmentId, ThresholdCommitment>,
    /// Collective ID for receipts
    collective_id: CollectiveId,
}

impl ThresholdEngine {
    pub fn new(collective_id: CollectiveId) -> Self {
        Self {
            commitments: HashMap::new(),
            collective_id,
        }
    }

    /// Create a new threshold commitment
    pub fn create_commitment(
        &mut self,
        action_description: impl Into<String>,
        threshold: ThresholdPolicy,
        value: Option<u64>,
        deadline: Option<DateTime<Utc>>,
        receipt_requirements: Vec<ReceiptRequirement>,
        journal: &mut AuditJournal,
    ) -> ThresholdCommitmentId {
        let mut commitment = ThresholdCommitment::new(action_description, threshold);

        if let Some(v) = value {
            commitment = commitment.with_value(v);
        }
        if let Some(d) = deadline {
            commitment = commitment.with_deadline(d);
        }
        for req in receipt_requirements {
            commitment = commitment.with_receipt_requirement(req);
        }

        let id = commitment.id.clone();
        self.commitments.insert(id.clone(), commitment);

        info!(commitment_id = %id, "Threshold commitment created");

        journal.log_receipt(CollectiveReceipt::new(
            self.collective_id.clone(),
            ReceiptType::Custom("threshold_created".into()),
            ResonatorId::new("system"),
            format!("Threshold commitment created: {}", id),
        ));

        id
    }

    /// Add a signature to a threshold commitment
    pub fn add_signature(
        &mut self,
        commitment_id: &ThresholdCommitmentId,
        signature: CommitmentSignature,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<SignatureResult> {
        let commitment = self.commitments.get_mut(commitment_id).ok_or_else(|| {
            CollectiveError::PolicyViolation(format!(
                "Threshold commitment not found: {}",
                commitment_id
            ))
        })?;

        // Check state
        match commitment.state {
            ThresholdCommitmentState::Satisfied => {
                return Ok(SignatureResult::AlreadySatisfied);
            }
            ThresholdCommitmentState::Expired => {
                return Ok(SignatureResult::Expired);
            }
            ThresholdCommitmentState::Rejected => {
                return Ok(SignatureResult::Rejected);
            }
            ThresholdCommitmentState::Collecting => {
                // Check deadline
                if commitment.is_expired() {
                    commitment.state = ThresholdCommitmentState::Expired;
                    warn!(commitment_id = %commitment_id, "Threshold commitment expired");
                    return Ok(SignatureResult::Expired);
                }
            }
        }

        let signer = signature.signer.clone();
        commitment.add_signature(signature);

        debug!(
            commitment_id = %commitment_id,
            signer = %signer,
            signatures = commitment.signature_count(),
            "Signature added to threshold commitment"
        );

        journal.log_receipt(CollectiveReceipt::new(
            self.collective_id.clone(),
            ReceiptType::Custom("signature_added".into()),
            signer,
            format!("Signed threshold commitment: {}", commitment_id),
        ));

        // Check if threshold is now met
        if commitment.is_satisfied() {
            commitment.state = ThresholdCommitmentState::Satisfied;

            info!(commitment_id = %commitment_id, "Threshold met!");

            journal.log_receipt(CollectiveReceipt::new(
                self.collective_id.clone(),
                ReceiptType::Custom("threshold_met".into()),
                ResonatorId::new("system"),
                format!("Threshold commitment satisfied: {}", commitment_id),
            ));

            Ok(SignatureResult::ThresholdMet)
        } else {
            Ok(SignatureResult::Accepted {
                signatures_so_far: commitment.signature_count(),
            })
        }
    }

    /// Reject a threshold commitment
    pub fn reject_commitment(
        &mut self,
        commitment_id: &ThresholdCommitmentId,
        reason: &str,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        let commitment = self.commitments.get_mut(commitment_id).ok_or_else(|| {
            CollectiveError::PolicyViolation(format!(
                "Threshold commitment not found: {}",
                commitment_id
            ))
        })?;

        commitment.state = ThresholdCommitmentState::Rejected;

        warn!(commitment_id = %commitment_id, reason = reason, "Threshold commitment rejected");

        journal.log_receipt(CollectiveReceipt::new(
            self.collective_id.clone(),
            ReceiptType::Custom("threshold_rejected".into()),
            ResonatorId::new("system"),
            format!(
                "Threshold commitment rejected: {} ({})",
                commitment_id, reason
            ),
        ));

        Ok(())
    }

    /// Check and expire any past-deadline commitments
    pub fn expire_stale_commitments(
        &mut self,
        journal: &mut AuditJournal,
    ) -> Vec<ThresholdCommitmentId> {
        let mut expired = Vec::new();

        for (id, commitment) in self.commitments.iter_mut() {
            if commitment.state == ThresholdCommitmentState::Collecting && commitment.is_expired() {
                commitment.state = ThresholdCommitmentState::Expired;
                expired.push(id.clone());

                journal.log_receipt(CollectiveReceipt::new(
                    self.collective_id.clone(),
                    ReceiptType::Custom("threshold_expired".into()),
                    ResonatorId::new("system"),
                    format!("Threshold commitment expired: {}", id),
                ));
            }
        }

        if !expired.is_empty() {
            info!(count = expired.len(), "Expired stale threshold commitments");
        }

        expired
    }

    // --- Query methods ---

    /// Get a threshold commitment by ID
    pub fn get_commitment(&self, id: &ThresholdCommitmentId) -> Option<&ThresholdCommitment> {
        self.commitments.get(id)
    }

    /// Check if a commitment is satisfied
    pub fn is_satisfied(&self, id: &ThresholdCommitmentId) -> bool {
        self.commitments
            .get(id)
            .map(|c| c.state == ThresholdCommitmentState::Satisfied)
            .unwrap_or(false)
    }

    /// Get all active (collecting) commitments
    pub fn active_commitments(&self) -> Vec<&ThresholdCommitment> {
        self.commitments
            .values()
            .filter(|c| c.state == ThresholdCommitmentState::Collecting)
            .collect()
    }

    /// Get all satisfied commitments
    pub fn satisfied_commitments(&self) -> Vec<&ThresholdCommitment> {
        self.commitments
            .values()
            .filter(|c| c.state == ThresholdCommitmentState::Satisfied)
            .collect()
    }

    /// Number of active commitments
    pub fn active_count(&self) -> usize {
        self.commitments
            .values()
            .filter(|c| c.state == ThresholdCommitmentState::Collecting)
            .count()
    }

    /// Clean up completed/expired/rejected commitments
    pub fn cleanup_completed(&mut self) -> usize {
        let before = self.commitments.len();
        self.commitments.retain(|_, c| {
            c.state == ThresholdCommitmentState::Collecting
                || c.state == ThresholdCommitmentState::Satisfied
        });
        before - self.commitments.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use collective_types::RoleId;

    fn setup() -> (ThresholdEngine, AuditJournal) {
        let id = CollectiveId::new("test");
        (ThresholdEngine::new(id.clone()), AuditJournal::new(id))
    }

    #[test]
    fn test_single_signer_commitment() {
        let (mut engine, mut journal) = setup();

        let id = engine.create_commitment(
            "Approve transfer",
            ThresholdPolicy::SingleSigner,
            None,
            None,
            vec![],
            &mut journal,
        );

        assert_eq!(engine.active_count(), 1);

        let sig = CommitmentSignature::new(ResonatorId::new("signer-1"));
        let result = engine.add_signature(&id, sig, &mut journal).unwrap();

        assert!(matches!(result, SignatureResult::ThresholdMet));
        assert!(engine.is_satisfied(&id));
    }

    #[test]
    fn test_m_of_n_commitment() {
        let (mut engine, mut journal) = setup();

        let id = engine.create_commitment(
            "Budget approval",
            ThresholdPolicy::m_of_n(2, 3),
            Some(50_000),
            None,
            vec![],
            &mut journal,
        );

        // First signer
        let sig1 = CommitmentSignature::new(ResonatorId::new("signer-1"));
        let result = engine.add_signature(&id, sig1, &mut journal).unwrap();
        assert!(matches!(
            result,
            SignatureResult::Accepted {
                signatures_so_far: 1
            }
        ));
        assert!(!engine.is_satisfied(&id));

        // Second signer
        let sig2 = CommitmentSignature::new(ResonatorId::new("signer-2"));
        let result = engine.add_signature(&id, sig2, &mut journal).unwrap();
        assert!(matches!(result, SignatureResult::ThresholdMet));
        assert!(engine.is_satisfied(&id));
    }

    #[test]
    fn test_role_based_commitment() {
        let (mut engine, mut journal) = setup();

        let id = engine.create_commitment(
            "Policy change",
            ThresholdPolicy::role_based(vec![RoleId::new("admin"), RoleId::new("compliance")]),
            None,
            None,
            vec![],
            &mut journal,
        );

        // Admin signs
        let sig1 =
            CommitmentSignature::new(ResonatorId::new("admin-1")).with_role(RoleId::new("admin"));
        let result = engine.add_signature(&id, sig1, &mut journal).unwrap();
        assert!(matches!(result, SignatureResult::Accepted { .. }));

        // Compliance signs
        let sig2 = CommitmentSignature::new(ResonatorId::new("compliance-1"))
            .with_role(RoleId::new("compliance"));
        let result = engine.add_signature(&id, sig2, &mut journal).unwrap();
        assert!(matches!(result, SignatureResult::ThresholdMet));
    }

    #[test]
    fn test_expired_commitment() {
        let (mut engine, mut journal) = setup();

        let id = engine.create_commitment(
            "Expired action",
            ThresholdPolicy::m_of_n(2, 3),
            None,
            Some(Utc::now() - chrono::Duration::hours(1)), // Already expired
            vec![],
            &mut journal,
        );

        let sig = CommitmentSignature::new(ResonatorId::new("signer-1"));
        let result = engine.add_signature(&id, sig, &mut journal).unwrap();
        assert!(matches!(result, SignatureResult::Expired));
    }

    #[test]
    fn test_reject_commitment() {
        let (mut engine, mut journal) = setup();

        let id = engine.create_commitment(
            "Bad action",
            ThresholdPolicy::SingleSigner,
            None,
            None,
            vec![],
            &mut journal,
        );

        engine
            .reject_commitment(&id, "Policy violation", &mut journal)
            .unwrap();

        let sig = CommitmentSignature::new(ResonatorId::new("signer-1"));
        let result = engine.add_signature(&id, sig, &mut journal).unwrap();
        assert!(matches!(result, SignatureResult::Rejected));
    }

    #[test]
    fn test_expire_stale() {
        let (mut engine, mut journal) = setup();

        // Create one expired and one active
        engine.create_commitment(
            "Expired",
            ThresholdPolicy::SingleSigner,
            None,
            Some(Utc::now() - chrono::Duration::hours(1)),
            vec![],
            &mut journal,
        );

        engine.create_commitment(
            "Active",
            ThresholdPolicy::SingleSigner,
            None,
            None,
            vec![],
            &mut journal,
        );

        let expired = engine.expire_stale_commitments(&mut journal);
        assert_eq!(expired.len(), 1);
        assert_eq!(engine.active_count(), 1);
    }

    #[test]
    fn test_cleanup() {
        let (mut engine, mut journal) = setup();

        // Create and satisfy one
        let id = engine.create_commitment(
            "Done",
            ThresholdPolicy::SingleSigner,
            None,
            None,
            vec![],
            &mut journal,
        );
        engine
            .add_signature(
                &id,
                CommitmentSignature::new(ResonatorId::new("s")),
                &mut journal,
            )
            .unwrap();

        // Create and reject one
        let id2 = engine.create_commitment(
            "Rejected",
            ThresholdPolicy::SingleSigner,
            None,
            None,
            vec![],
            &mut journal,
        );
        engine.reject_commitment(&id2, "bad", &mut journal).unwrap();

        // Active one
        engine.create_commitment(
            "Active",
            ThresholdPolicy::SingleSigner,
            None,
            None,
            vec![],
            &mut journal,
        );

        let cleaned = engine.cleanup_completed();
        assert_eq!(cleaned, 1); // Only rejected gets cleaned
        assert_eq!(engine.commitments.len(), 2); // Satisfied + Active remain
    }
}
