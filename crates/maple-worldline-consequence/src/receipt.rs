//! Cryptographic execution receipt proving that a self-modification was applied.
//!
//! Follows the resonator `ConsequenceReceipt` pattern: deterministic SHA-256
//! hash of receipt fields provides tamper-evident proof of execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use maple_worldline_commitment::types::SelfCommitmentId;
use maple_worldline_intent::types::{IntentId, SubstrateTier};

use crate::types::SelfConsequenceId;

/// Cryptographic receipt proving execution of a self-modification.
///
/// The `execution_hash` is a deterministic SHA-256 of key fields, providing
/// tamper-evident proof that the receipt has not been modified since issuance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    /// Unique receipt identifier.
    pub receipt_id: String,
    /// The consequence this receipt is for.
    pub consequence_id: SelfConsequenceId,
    /// The commitment that authorized execution.
    pub commitment_id: SelfCommitmentId,
    /// The intent that originated the regeneration.
    pub intent_id: IntentId,
    /// SHA-256 hash of the execution payload.
    pub execution_hash: String,
    /// When the receipt was issued.
    pub issued_at: DateTime<Utc>,
    /// Human-readable summary of what was executed.
    pub summary: String,
    /// Number of tests that passed during execution.
    pub tests_passed: usize,
    /// Governance tier of the executed modification.
    pub governance_tier: SubstrateTier,
}

impl ExecutionReceipt {
    /// Create a new execution receipt with auto-computed hash.
    pub fn new(
        consequence_id: SelfConsequenceId,
        commitment_id: SelfCommitmentId,
        intent_id: IntentId,
        governance_tier: SubstrateTier,
        tests_passed: usize,
        summary: impl Into<String>,
    ) -> Self {
        let mut receipt = Self {
            receipt_id: format!("receipt-{}", uuid::Uuid::new_v4()),
            consequence_id,
            commitment_id,
            intent_id,
            execution_hash: String::new(),
            issued_at: Utc::now(),
            summary: summary.into(),
            tests_passed,
            governance_tier,
        };
        receipt.execution_hash = receipt.compute_hash();
        receipt
    }

    /// Compute deterministic SHA-256 hash of the receipt fields.
    pub fn compute_hash(&self) -> String {
        let payload = serde_json::json!({
            "consequence_id": self.consequence_id.0,
            "commitment_id": self.commitment_id.0,
            "intent_id": self.intent_id.to_string(),
            "issued_at": self.issued_at.to_rfc3339(),
            "summary": self.summary,
            "tests_passed": self.tests_passed,
            "governance_tier": format!("{:?}", self.governance_tier),
        });

        let mut hasher = Sha256::new();
        hasher.update(payload.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify that the receipt hash is valid (not tampered).
    pub fn verify(&self) -> bool {
        self.execution_hash == self.compute_hash()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_receipt() -> ExecutionReceipt {
        ExecutionReceipt::new(
            SelfConsequenceId::new(),
            SelfCommitmentId::new(),
            IntentId::new(),
            SubstrateTier::Tier0,
            5,
            "Applied configuration optimization",
        )
    }

    #[test]
    fn receipt_generation() {
        let receipt = make_receipt();
        assert!(receipt.receipt_id.starts_with("receipt-"));
        assert!(!receipt.execution_hash.is_empty());
        assert_eq!(receipt.tests_passed, 5);
        assert_eq!(receipt.governance_tier, SubstrateTier::Tier0);
    }

    #[test]
    fn receipt_verification_succeeds() {
        let receipt = make_receipt();
        assert!(receipt.verify());
    }

    #[test]
    fn receipt_tamper_detection() {
        let mut receipt = make_receipt();
        assert!(receipt.verify());

        // Tamper with the summary
        receipt.summary = "Tampered summary".to_string();
        assert!(!receipt.verify());
    }

    #[test]
    fn receipt_hash_deterministic() {
        let receipt = make_receipt();
        let hash1 = receipt.compute_hash();
        let hash2 = receipt.compute_hash();
        assert_eq!(hash1, hash2);
    }
}
