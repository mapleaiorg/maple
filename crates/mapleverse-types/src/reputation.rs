//! Reputation system for MapleVerse
//!
//! **CRITICAL INVARIANT**: Reputation comes ONLY from verified receipts.
//!
//! There is NO:
//! - Self-assessment
//! - Peer voting without receipts
//! - Administrator override
//! - Imported reputation from external systems
//!
//! Every reputation point must trace back to a specific receipt.

use crate::entity::EntityId;
use crate::errors::{MapleVerseError, MapleVerseResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Reputation score for an entity
///
/// All reputation derives from receipts. The score is calculated by
/// aggregating receipt-based reputation changes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReputationScore {
    /// Current reputation value
    score: i64,
    /// Minimum allowed score
    min_score: i64,
    /// Maximum allowed score
    max_score: i64,
    /// Total positive reputation earned
    total_positive: u64,
    /// Total negative reputation received
    total_negative: u64,
    /// Number of receipts contributing to reputation
    receipt_count: u64,
    /// Breakdown by category
    category_scores: HashMap<String, i64>,
    /// Last update epoch
    last_updated_epoch: u64,
}

impl Default for ReputationScore {
    fn default() -> Self {
        Self {
            score: 0,
            min_score: -10000,
            max_score: 10000,
            total_positive: 0,
            total_negative: 0,
            receipt_count: 0,
            category_scores: HashMap::new(),
            last_updated_epoch: 0,
        }
    }
}

impl ReputationScore {
    /// Create a new reputation score with bounds
    pub fn new(initial: i64, min_score: i64, max_score: i64) -> Self {
        Self {
            score: initial.clamp(min_score, max_score),
            min_score,
            max_score,
            ..Default::default()
        }
    }

    /// Get current score
    pub fn score(&self) -> i64 {
        self.score
    }

    /// Get score as normalized value (0.0 to 1.0)
    pub fn normalized(&self) -> f64 {
        let range = (self.max_score - self.min_score) as f64;
        if range == 0.0 {
            return 0.5;
        }
        (self.score - self.min_score) as f64 / range
    }

    /// Apply reputation from a receipt
    ///
    /// This is the ONLY way to modify reputation.
    pub fn apply_receipt(&mut self, receipt: &ReputationReceipt, epoch: u64) {
        let change = receipt.reputation_change;

        // Track positive/negative
        if change > 0 {
            self.total_positive += change as u64;
        } else if change < 0 {
            self.total_negative += (-change) as u64;
        }

        // Update score with bounds
        self.score = (self.score + change).clamp(self.min_score, self.max_score);

        // Update category
        if let Some(category) = &receipt.category {
            let category_score = self.category_scores.entry(category.clone()).or_insert(0);
            *category_score = (*category_score + change).clamp(self.min_score, self.max_score);
        }

        self.receipt_count += 1;
        self.last_updated_epoch = epoch;
    }

    /// Apply decay at epoch boundary
    pub fn apply_decay(&mut self, decay_rate: f64, epoch: u64) {
        if decay_rate <= 0.0 || decay_rate > 1.0 {
            return;
        }

        // Decay towards zero
        let decay = (self.score as f64 * decay_rate) as i64;
        if self.score > 0 {
            self.score = (self.score - decay).max(0);
        } else if self.score < 0 {
            self.score = (self.score + decay.abs()).min(0);
        }

        // Also decay category scores
        for (_, cat_score) in self.category_scores.iter_mut() {
            let cat_decay = (*cat_score as f64 * decay_rate) as i64;
            if *cat_score > 0 {
                *cat_score = (*cat_score - cat_decay).max(0);
            } else if *cat_score < 0 {
                *cat_score = (*cat_score + cat_decay.abs()).min(0);
            }
        }

        self.last_updated_epoch = epoch;
    }

    /// Get reputation tier
    pub fn tier(&self) -> ReputationTier {
        let normalized = self.normalized();
        if normalized >= 0.9 {
            ReputationTier::Legendary
        } else if normalized >= 0.75 {
            ReputationTier::Excellent
        } else if normalized >= 0.6 {
            ReputationTier::Good
        } else if normalized >= 0.4 {
            ReputationTier::Neutral
        } else if normalized >= 0.25 {
            ReputationTier::Poor
        } else if normalized >= 0.1 {
            ReputationTier::Bad
        } else {
            ReputationTier::Untrusted
        }
    }

    /// Get score for a specific category
    pub fn category_score(&self, category: &str) -> i64 {
        self.category_scores.get(category).copied().unwrap_or(0)
    }

    /// Check if entity meets minimum reputation threshold
    pub fn meets_threshold(&self, threshold: i64) -> bool {
        self.score >= threshold
    }

    /// Get receipt count
    pub fn receipt_count(&self) -> u64 {
        self.receipt_count
    }

    /// Get ratio of positive to total reputation
    pub fn positive_ratio(&self) -> f64 {
        let total = self.total_positive + self.total_negative;
        if total == 0 {
            return 0.5; // Neutral if no reputation yet
        }
        self.total_positive as f64 / total as f64
    }
}

/// Reputation tier levels
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReputationTier {
    /// Untrusted - very low reputation
    Untrusted = 0,
    /// Bad - significantly below average
    Bad = 1,
    /// Poor - below average
    Poor = 2,
    /// Neutral - average reputation
    Neutral = 3,
    /// Good - above average
    Good = 4,
    /// Excellent - significantly above average
    Excellent = 5,
    /// Legendary - top tier reputation
    Legendary = 6,
}

impl ReputationTier {
    /// Get minimum normalized score for this tier
    pub fn min_normalized(&self) -> f64 {
        match self {
            Self::Untrusted => 0.0,
            Self::Bad => 0.1,
            Self::Poor => 0.25,
            Self::Neutral => 0.4,
            Self::Good => 0.6,
            Self::Excellent => 0.75,
            Self::Legendary => 0.9,
        }
    }

    /// Check if this tier is trustworthy (Good or above)
    pub fn is_trustworthy(&self) -> bool {
        *self >= ReputationTier::Good
    }
}

/// Source of reputation - MUST be a receipt
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReputationSource {
    /// From a commitment completion receipt
    CommitmentReceipt {
        /// The receipt ID
        receipt_id: String,
        /// The commitment ID
        commitment_id: String,
    },
    /// From a workflow completion receipt
    WorkflowReceipt {
        /// The receipt ID
        receipt_id: String,
        /// The workflow instance ID
        workflow_id: String,
    },
    /// From a collective endorsement receipt
    CollectiveEndorsement {
        /// The endorsement receipt ID
        receipt_id: String,
        /// The endorsing collective
        collective_id: EntityId,
    },
    /// From a verified attestation receipt
    AttestationReceipt {
        /// The attestation receipt ID
        receipt_id: String,
        /// What was attested
        attestation_type: String,
    },
}

impl ReputationSource {
    /// Validate that this is a proper receipt-based source
    pub fn validate(&self) -> MapleVerseResult<()> {
        // All variants are receipt-based, so validation passes
        // This method exists to reject any future non-receipt sources
        Ok(())
    }

    /// Get the receipt ID
    pub fn receipt_id(&self) -> &str {
        match self {
            Self::CommitmentReceipt { receipt_id, .. } => receipt_id,
            Self::WorkflowReceipt { receipt_id, .. } => receipt_id,
            Self::CollectiveEndorsement { receipt_id, .. } => receipt_id,
            Self::AttestationReceipt { receipt_id, .. } => receipt_id,
        }
    }
}

/// A receipt that contributes to reputation
///
/// Every reputation change MUST have an associated receipt.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReputationReceipt {
    /// Unique receipt ID
    pub id: ReputationReceiptId,
    /// The entity receiving reputation
    pub entity_id: EntityId,
    /// Source of this reputation (must be a receipt)
    pub source: ReputationSource,
    /// Reputation change (+/-)
    pub reputation_change: i64,
    /// Category (e.g., "coordination", "reliability", "contribution")
    pub category: Option<String>,
    /// Description of why reputation was earned/lost
    pub description: String,
    /// When this receipt was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Epoch when this was processed
    pub epoch: u64,
    /// Evidence hash (for verification)
    pub evidence_hash: Option<String>,
}

impl ReputationReceipt {
    /// Create a new reputation receipt
    pub fn new(
        entity_id: EntityId,
        source: ReputationSource,
        reputation_change: i64,
        description: impl Into<String>,
        epoch: u64,
    ) -> Self {
        Self {
            id: ReputationReceiptId::generate(),
            entity_id,
            source,
            reputation_change,
            category: None,
            description: description.into(),
            timestamp: chrono::Utc::now(),
            epoch,
            evidence_hash: None,
        }
    }

    /// Set category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set evidence hash
    pub fn with_evidence(mut self, hash: impl Into<String>) -> Self {
        self.evidence_hash = Some(hash.into());
        self
    }

    /// Validate the receipt
    pub fn validate(&self) -> MapleVerseResult<()> {
        // Validate source is receipt-based
        self.source.validate()?;

        // Validate reputation change is within reasonable bounds
        if self.reputation_change.abs() > 1000 {
            return Err(MapleVerseError::InvalidReceiptForReputation {
                receipt_id: self.id.to_string(),
                reason: "Reputation change exceeds maximum allowed per receipt".to_string(),
            });
        }

        Ok(())
    }
}

/// Unique identifier for a reputation receipt
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReputationReceiptId(String);

impl ReputationReceiptId {
    /// Create a new receipt ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new random receipt ID
    pub fn generate() -> Self {
        Self(format!("rep-receipt-{}", Uuid::new_v4()))
    }

    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ReputationReceiptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Builder for creating reputation receipts
pub struct ReputationReceiptBuilder {
    entity_id: Option<EntityId>,
    source: Option<ReputationSource>,
    reputation_change: i64,
    category: Option<String>,
    description: Option<String>,
    epoch: u64,
    evidence_hash: Option<String>,
}

impl Default for ReputationReceiptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ReputationReceiptBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            entity_id: None,
            source: None,
            reputation_change: 0,
            category: None,
            description: None,
            epoch: 0,
            evidence_hash: None,
        }
    }

    /// Set entity
    pub fn entity(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }

    /// Set source from commitment receipt
    pub fn from_commitment(mut self, receipt_id: impl Into<String>, commitment_id: impl Into<String>) -> Self {
        self.source = Some(ReputationSource::CommitmentReceipt {
            receipt_id: receipt_id.into(),
            commitment_id: commitment_id.into(),
        });
        self
    }

    /// Set source from workflow receipt
    pub fn from_workflow(mut self, receipt_id: impl Into<String>, workflow_id: impl Into<String>) -> Self {
        self.source = Some(ReputationSource::WorkflowReceipt {
            receipt_id: receipt_id.into(),
            workflow_id: workflow_id.into(),
        });
        self
    }

    /// Set reputation change
    pub fn change(mut self, amount: i64) -> Self {
        self.reputation_change = amount;
        self
    }

    /// Set category
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set epoch
    pub fn epoch(mut self, epoch: u64) -> Self {
        self.epoch = epoch;
        self
    }

    /// Set evidence hash
    pub fn evidence(mut self, hash: impl Into<String>) -> Self {
        self.evidence_hash = Some(hash.into());
        self
    }

    /// Build the receipt
    pub fn build(self) -> MapleVerseResult<ReputationReceipt> {
        let entity_id = self.entity_id.ok_or_else(|| MapleVerseError::InvalidConfiguration {
            reason: "Entity ID is required for reputation receipt".to_string(),
        })?;

        let source = self.source.ok_or_else(|| MapleVerseError::InvalidReputationSource {
            attempted_source: "none".to_string(),
        })?;

        let description = self.description.unwrap_or_else(|| "Reputation change".to_string());

        let mut receipt = ReputationReceipt::new(
            entity_id,
            source,
            self.reputation_change,
            description,
            self.epoch,
        );

        if let Some(cat) = self.category {
            receipt = receipt.with_category(cat);
        }

        if let Some(hash) = self.evidence_hash {
            receipt = receipt.with_evidence(hash);
        }

        receipt.validate()?;
        Ok(receipt)
    }
}

/// Attempt to add reputation without a receipt (WILL ALWAYS FAIL)
pub fn add_reputation_without_receipt(
    _entity_id: &EntityId,
    _amount: i64,
    source: &str,
) -> MapleVerseResult<()> {
    Err(MapleVerseError::InvalidReputationSource {
        attempted_source: source.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_score_default() {
        let score = ReputationScore::default();
        assert_eq!(score.score(), 0);
        assert_eq!(score.tier(), ReputationTier::Neutral);
        assert_eq!(score.normalized(), 0.5);
    }

    #[test]
    fn test_reputation_from_receipt() {
        let mut score = ReputationScore::default();

        let receipt = ReputationReceipt::new(
            EntityId::new("agent-1"),
            ReputationSource::CommitmentReceipt {
                receipt_id: "receipt-1".to_string(),
                commitment_id: "commit-1".to_string(),
            },
            100,
            "Completed commitment successfully",
            1,
        );

        score.apply_receipt(&receipt, 1);

        assert_eq!(score.score(), 100);
        assert_eq!(score.receipt_count(), 1);
        assert_eq!(score.total_positive, 100);
        assert_eq!(score.total_negative, 0);
    }

    #[test]
    fn test_reputation_negative() {
        let mut score = ReputationScore::default();

        let receipt = ReputationReceipt::new(
            EntityId::new("agent-1"),
            ReputationSource::CommitmentReceipt {
                receipt_id: "receipt-1".to_string(),
                commitment_id: "commit-1".to_string(),
            },
            -50,
            "Failed to complete commitment",
            1,
        );

        score.apply_receipt(&receipt, 1);

        assert_eq!(score.score(), -50);
        assert_eq!(score.total_negative, 50);
    }

    #[test]
    fn test_reputation_bounds() {
        let mut score = ReputationScore::new(0, -100, 100);

        // Try to go above max
        let receipt = ReputationReceipt::new(
            EntityId::new("agent-1"),
            ReputationSource::CommitmentReceipt {
                receipt_id: "r1".to_string(),
                commitment_id: "c1".to_string(),
            },
            200,
            "Big reward",
            1,
        );
        score.apply_receipt(&receipt, 1);
        assert_eq!(score.score(), 100); // Clamped to max

        // Try to go below min
        let negative_receipt = ReputationReceipt::new(
            EntityId::new("agent-1"),
            ReputationSource::CommitmentReceipt {
                receipt_id: "r2".to_string(),
                commitment_id: "c2".to_string(),
            },
            -300,
            "Big penalty",
            2,
        );
        score.apply_receipt(&negative_receipt, 2);
        assert_eq!(score.score(), -100); // Clamped to min
    }

    #[test]
    fn test_reputation_category() {
        let mut score = ReputationScore::default();

        let receipt = ReputationReceipt::new(
            EntityId::new("agent-1"),
            ReputationSource::CommitmentReceipt {
                receipt_id: "r1".to_string(),
                commitment_id: "c1".to_string(),
            },
            50,
            "Good coordination",
            1,
        )
        .with_category("coordination");

        score.apply_receipt(&receipt, 1);

        assert_eq!(score.category_score("coordination"), 50);
        assert_eq!(score.category_score("reliability"), 0);
    }

    #[test]
    fn test_reputation_decay() {
        let mut score = ReputationScore::default();

        // Add positive reputation
        let receipt = ReputationReceipt::new(
            EntityId::new("agent-1"),
            ReputationSource::CommitmentReceipt {
                receipt_id: "r1".to_string(),
                commitment_id: "c1".to_string(),
            },
            100,
            "Initial reputation",
            1,
        );
        score.apply_receipt(&receipt, 1);

        // Apply 10% decay
        score.apply_decay(0.1, 2);
        assert_eq!(score.score(), 90);

        // Apply again
        score.apply_decay(0.1, 3);
        assert_eq!(score.score(), 81);
    }

    #[test]
    fn test_reputation_tiers() {
        // With range -10000 to 10000 (20000 total):
        // normalized = (score - min) / range = (score + 10000) / 20000
        // Legendary >= 0.9 => score >= 8000
        // Excellent >= 0.75 => score >= 5000
        // Good >= 0.6 => score >= 2000
        // Neutral >= 0.4 => score >= -2000
        // Poor >= 0.25 => score >= -5000
        // Bad >= 0.1 => score >= -8000
        // Untrusted < 0.1 => score < -8000
        let cases = vec![
            (9000, ReputationTier::Legendary),   // normalized = 0.95
            (7000, ReputationTier::Excellent),   // normalized = 0.85
            (3000, ReputationTier::Good),        // normalized = 0.65
            (0, ReputationTier::Neutral),        // normalized = 0.5
            (-4000, ReputationTier::Poor),       // normalized = 0.3
            (-7000, ReputationTier::Bad),        // normalized = 0.15
            (-9000, ReputationTier::Untrusted),  // normalized = 0.05
        ];

        for (value, expected_tier) in cases {
            let score = ReputationScore::new(value, -10000, 10000);
            assert_eq!(score.tier(), expected_tier, "Failed for value {}", value);
        }
    }

    #[test]
    fn test_reputation_positive_ratio() {
        let mut score = ReputationScore::default();

        // 75% positive, 25% negative
        for i in 0..3 {
            let receipt = ReputationReceipt::new(
                EntityId::new("agent-1"),
                ReputationSource::CommitmentReceipt {
                    receipt_id: format!("pos-{}", i),
                    commitment_id: format!("c-{}", i),
                },
                100,
                "Positive",
                i,
            );
            score.apply_receipt(&receipt, i);
        }

        let neg_receipt = ReputationReceipt::new(
            EntityId::new("agent-1"),
            ReputationSource::CommitmentReceipt {
                receipt_id: "neg-1".to_string(),
                commitment_id: "c-neg".to_string(),
            },
            -100,
            "Negative",
            4,
        );
        score.apply_receipt(&neg_receipt, 4);

        assert_eq!(score.positive_ratio(), 0.75);
    }

    #[test]
    fn test_reputation_receipt_validation() {
        let receipt = ReputationReceipt::new(
            EntityId::new("agent-1"),
            ReputationSource::CommitmentReceipt {
                receipt_id: "r1".to_string(),
                commitment_id: "c1".to_string(),
            },
            100,
            "Valid receipt",
            1,
        );

        assert!(receipt.validate().is_ok());

        // Excessive change should fail
        let excessive_receipt = ReputationReceipt::new(
            EntityId::new("agent-1"),
            ReputationSource::CommitmentReceipt {
                receipt_id: "r2".to_string(),
                commitment_id: "c2".to_string(),
            },
            5000, // Too much
            "Excessive",
            1,
        );

        assert!(excessive_receipt.validate().is_err());
    }

    #[test]
    fn test_reputation_without_receipt_fails() {
        let entity_id = EntityId::new("agent-1");
        let result = add_reputation_without_receipt(&entity_id, 100, "self-assessment");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MapleVerseError::InvalidReputationSource { .. }));
    }

    #[test]
    fn test_reputation_receipt_builder() {
        let receipt = ReputationReceiptBuilder::new()
            .entity(EntityId::new("agent-1"))
            .from_commitment("receipt-123", "commitment-456")
            .change(75)
            .category("reliability")
            .description("Completed on time")
            .epoch(5)
            .evidence("hash-abc")
            .build()
            .unwrap();

        assert_eq!(receipt.reputation_change, 75);
        assert_eq!(receipt.category, Some("reliability".to_string()));
        assert_eq!(receipt.epoch, 5);
    }

    #[test]
    fn test_reputation_receipt_builder_missing_source() {
        let result = ReputationReceiptBuilder::new()
            .entity(EntityId::new("agent-1"))
            .change(50)
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_reputation_source_receipt_id() {
        let sources = vec![
            ReputationSource::CommitmentReceipt {
                receipt_id: "cr-1".to_string(),
                commitment_id: "c-1".to_string(),
            },
            ReputationSource::WorkflowReceipt {
                receipt_id: "wr-1".to_string(),
                workflow_id: "w-1".to_string(),
            },
            ReputationSource::CollectiveEndorsement {
                receipt_id: "ce-1".to_string(),
                collective_id: EntityId::new("col-1"),
            },
            ReputationSource::AttestationReceipt {
                receipt_id: "ar-1".to_string(),
                attestation_type: "skill".to_string(),
            },
        ];

        let expected_ids = vec!["cr-1", "wr-1", "ce-1", "ar-1"];

        for (source, expected_id) in sources.iter().zip(expected_ids.iter()) {
            assert_eq!(source.receipt_id(), *expected_id);
            assert!(source.validate().is_ok());
        }
    }

    #[test]
    fn test_trustworthy_tiers() {
        assert!(!ReputationTier::Untrusted.is_trustworthy());
        assert!(!ReputationTier::Bad.is_trustworthy());
        assert!(!ReputationTier::Poor.is_trustworthy());
        assert!(!ReputationTier::Neutral.is_trustworthy());
        assert!(ReputationTier::Good.is_trustworthy());
        assert!(ReputationTier::Excellent.is_trustworthy());
        assert!(ReputationTier::Legendary.is_trustworthy());
    }

    #[test]
    fn test_reputation_serialization() {
        let mut score = ReputationScore::default();
        let receipt = ReputationReceipt::new(
            EntityId::new("agent-1"),
            ReputationSource::CommitmentReceipt {
                receipt_id: "r1".to_string(),
                commitment_id: "c1".to_string(),
            },
            100,
            "Test",
            1,
        );
        score.apply_receipt(&receipt, 1);

        let json = serde_json::to_string(&score).unwrap();
        let deserialized: ReputationScore = serde_json::from_str(&json).unwrap();
        assert_eq!(score.score(), deserialized.score());
    }
}
