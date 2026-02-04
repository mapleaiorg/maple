//! Threshold commitments: collective decision-making
//!
//! Threshold policies define how many and which members must agree
//! before a collective action can proceed. This is NOT just multi-sig—
//! it's receipt-gated collective commitment.

use crate::{ReceiptRequirement, RoleId};
use chrono::{DateTime, Utc};
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};

/// Policy defining how many approvals are needed for collective action
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ThresholdPolicy {
    /// Any single authorized signer (1-of-N)
    SingleSigner,

    /// M-of-N approval required
    MofN {
        /// Minimum approvals needed
        m: u32,
        /// Total eligible signers
        n: u32,
    },

    /// Specific roles must approve
    RoleBased {
        /// All of these roles must have at least one signer
        required_roles: Vec<RoleId>,
    },

    /// Different thresholds based on action value/risk
    RiskTiered {
        /// Tiers from lowest to highest risk
        tiers: Vec<RiskTier>,
    },

    /// Weighted voting with minimum weight threshold
    WeightedVote {
        /// Minimum total weight needed
        threshold_weight: u64,
    },

    /// Time-locked: action proceeds after delay unless vetoed
    TimeLocked {
        /// Delay before action executes (seconds)
        delay_secs: u64,
        /// Threshold to veto during the delay
        veto_threshold: Box<ThresholdPolicy>,
    },
}

impl ThresholdPolicy {
    /// Create an M-of-N policy
    pub fn m_of_n(m: u32, n: u32) -> Self {
        Self::MofN { m, n }
    }

    /// Create a role-based policy
    pub fn role_based(roles: Vec<RoleId>) -> Self {
        Self::RoleBased {
            required_roles: roles,
        }
    }

    /// Create a simple majority policy
    pub fn majority(total: u32) -> Self {
        Self::MofN {
            m: total / 2 + 1,
            n: total,
        }
    }

    /// Create a unanimous policy
    pub fn unanimous(total: u32) -> Self {
        Self::MofN { m: total, n: total }
    }
}

/// A risk tier defining threshold policy for a value range
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskTier {
    /// Maximum value for this tier (actions up to this value use this policy)
    pub max_value: u64,
    /// The threshold policy for this tier
    pub policy: Box<ThresholdPolicy>,
}

impl RiskTier {
    pub fn new(max_value: u64, policy: ThresholdPolicy) -> Self {
        Self {
            max_value,
            policy: Box::new(policy),
        }
    }
}

/// A signature on a threshold commitment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentSignature {
    /// Who signed
    pub signer: ResonatorId,
    /// Role the signer holds (relevant for role-based policies)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<RoleId>,
    /// Voting weight (relevant for weighted voting)
    pub weight: u64,
    /// When the signature was made
    pub signed_at: DateTime<Utc>,
    /// Cryptographic signature data
    pub signature_data: Vec<u8>,
}

impl CommitmentSignature {
    pub fn new(signer: ResonatorId) -> Self {
        Self {
            signer,
            role: None,
            weight: 1,
            signed_at: Utc::now(),
            signature_data: Vec::new(),
        }
    }

    pub fn with_role(mut self, role: RoleId) -> Self {
        self.role = Some(role);
        self
    }

    pub fn with_weight(mut self, weight: u64) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_signature_data(mut self, data: Vec<u8>) -> Self {
        self.signature_data = data;
        self
    }

    /// Check if the signer holds a specific role
    pub fn has_role(&self, role_id: &RoleId) -> bool {
        self.role.as_ref() == Some(role_id)
    }
}

/// Unique identifier for a threshold commitment
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThresholdCommitmentId(pub String);

impl ThresholdCommitmentId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for ThresholdCommitmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// State of a threshold commitment
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThresholdCommitmentState {
    /// Collecting signatures
    #[default]
    Collecting,
    /// Threshold has been satisfied
    Satisfied,
    /// Commitment expired before threshold was met
    Expired,
    /// Commitment was explicitly rejected
    Rejected,
}

/// A commitment that requires threshold approval before execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThresholdCommitment {
    /// Unique identifier
    pub id: ThresholdCommitmentId,
    /// Description of the action being committed to
    pub action_description: String,
    /// Value of the action (used for risk-tiered policies)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<u64>,
    /// The threshold policy to satisfy
    pub threshold: ThresholdPolicy,
    /// Collected signatures so far
    pub signatures: Vec<CommitmentSignature>,
    /// Receipts required upon execution
    pub receipts_required: Vec<ReceiptRequirement>,
    /// Current state
    pub state: ThresholdCommitmentState,
    /// When the commitment was created
    pub created_at: DateTime<Utc>,
    /// Deadline for collecting signatures
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<DateTime<Utc>>,
}

impl ThresholdCommitment {
    /// Create a new threshold commitment
    pub fn new(
        action_description: impl Into<String>,
        threshold: ThresholdPolicy,
    ) -> Self {
        Self {
            id: ThresholdCommitmentId::generate(),
            action_description: action_description.into(),
            value: None,
            threshold,
            signatures: Vec::new(),
            receipts_required: Vec::new(),
            state: ThresholdCommitmentState::Collecting,
            created_at: Utc::now(),
            deadline: None,
        }
    }

    pub fn with_value(mut self, value: u64) -> Self {
        self.value = Some(value);
        self
    }

    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn with_receipt_requirement(mut self, req: ReceiptRequirement) -> Self {
        self.receipts_required.push(req);
        self
    }

    /// Add a signature
    pub fn add_signature(&mut self, signature: CommitmentSignature) {
        // Don't add duplicate signers
        if !self.signatures.iter().any(|s| s.signer == signature.signer) {
            self.signatures.push(signature);
        }
    }

    /// Check if the threshold has been satisfied
    pub fn is_satisfied(&self) -> bool {
        match &self.threshold {
            ThresholdPolicy::SingleSigner => !self.signatures.is_empty(),

            ThresholdPolicy::MofN { m, .. } => self.signatures.len() >= *m as usize,

            ThresholdPolicy::RoleBased { required_roles } => {
                required_roles.iter().all(|role| {
                    self.signatures.iter().any(|sig| sig.has_role(role))
                })
            }

            ThresholdPolicy::RiskTiered { tiers } => {
                let action_value = self.value.unwrap_or(0);
                // Find the applicable tier
                let applicable_tier = tiers
                    .iter()
                    .find(|t| action_value <= t.max_value)
                    .or_else(|| tiers.last());

                match applicable_tier {
                    Some(tier) => {
                        // Create a temporary commitment with the tier's policy
                        // to check satisfaction
                        let temp = ThresholdCommitment {
                            threshold: (*tier.policy).clone(),
                            signatures: self.signatures.clone(),
                            value: self.value,
                            ..self.clone()
                        };
                        temp.is_satisfied()
                    }
                    None => false,
                }
            }

            ThresholdPolicy::WeightedVote { threshold_weight } => {
                let total_weight: u64 = self.signatures.iter().map(|s| s.weight).sum();
                total_weight >= *threshold_weight
            }

            ThresholdPolicy::TimeLocked { delay_secs, .. } => {
                // Satisfied if delay has passed (veto check happens elsewhere)
                let elapsed = Utc::now()
                    .signed_duration_since(self.created_at)
                    .num_seconds();
                elapsed >= *delay_secs as i64
            }
        }
    }

    /// Check if the commitment has expired
    pub fn is_expired(&self) -> bool {
        match self.deadline {
            Some(deadline) => Utc::now() >= deadline,
            None => false,
        }
    }

    /// Number of signatures collected
    pub fn signature_count(&self) -> usize {
        self.signatures.len()
    }

    /// Total weight of collected signatures
    pub fn total_weight(&self) -> u64 {
        self.signatures.iter().map(|s| s.weight).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_signer() {
        let mut commitment =
            ThresholdCommitment::new("Test action", ThresholdPolicy::SingleSigner);

        assert!(!commitment.is_satisfied());

        commitment.add_signature(CommitmentSignature::new(ResonatorId::new("res-1")));
        assert!(commitment.is_satisfied());
    }

    #[test]
    fn test_m_of_n() {
        let mut commitment =
            ThresholdCommitment::new("Test action", ThresholdPolicy::m_of_n(2, 3));

        assert!(!commitment.is_satisfied());

        commitment.add_signature(CommitmentSignature::new(ResonatorId::new("res-1")));
        assert!(!commitment.is_satisfied());

        commitment.add_signature(CommitmentSignature::new(ResonatorId::new("res-2")));
        assert!(commitment.is_satisfied());
    }

    #[test]
    fn test_role_based() {
        let mut commitment = ThresholdCommitment::new(
            "Test action",
            ThresholdPolicy::role_based(vec![
                RoleId::new("admin"),
                RoleId::new("auditor"),
            ]),
        );

        // Only admin signed
        commitment.add_signature(
            CommitmentSignature::new(ResonatorId::new("res-1"))
                .with_role(RoleId::new("admin")),
        );
        assert!(!commitment.is_satisfied());

        // Now auditor signed too
        commitment.add_signature(
            CommitmentSignature::new(ResonatorId::new("res-2"))
                .with_role(RoleId::new("auditor")),
        );
        assert!(commitment.is_satisfied());
    }

    #[test]
    fn test_weighted_vote() {
        let mut commitment = ThresholdCommitment::new(
            "Test action",
            ThresholdPolicy::WeightedVote {
                threshold_weight: 100,
            },
        );

        commitment.add_signature(
            CommitmentSignature::new(ResonatorId::new("res-1")).with_weight(40),
        );
        assert!(!commitment.is_satisfied());
        assert_eq!(commitment.total_weight(), 40);

        commitment.add_signature(
            CommitmentSignature::new(ResonatorId::new("res-2")).with_weight(60),
        );
        assert!(commitment.is_satisfied());
        assert_eq!(commitment.total_weight(), 100);
    }

    #[test]
    fn test_majority() {
        let mut commitment =
            ThresholdCommitment::new("Test action", ThresholdPolicy::majority(5));

        for i in 0..2 {
            commitment.add_signature(CommitmentSignature::new(ResonatorId::new(
                format!("res-{}", i),
            )));
        }
        assert!(!commitment.is_satisfied()); // 2 of 5, need 3

        commitment.add_signature(CommitmentSignature::new(ResonatorId::new("res-2")));
        assert!(commitment.is_satisfied()); // 3 of 5
    }

    #[test]
    fn test_duplicate_signer_ignored() {
        let mut commitment =
            ThresholdCommitment::new("Test action", ThresholdPolicy::m_of_n(2, 3));

        commitment.add_signature(CommitmentSignature::new(ResonatorId::new("res-1")));
        commitment.add_signature(CommitmentSignature::new(ResonatorId::new("res-1")));
        assert_eq!(commitment.signature_count(), 1);
        assert!(!commitment.is_satisfied());
    }

    #[test]
    fn test_risk_tiered() {
        let policy = ThresholdPolicy::RiskTiered {
            tiers: vec![
                RiskTier::new(1000, ThresholdPolicy::SingleSigner),
                RiskTier::new(10_000, ThresholdPolicy::m_of_n(2, 3)),
                RiskTier::new(u64::MAX, ThresholdPolicy::m_of_n(3, 5)),
            ],
        };

        // Low value: single signer sufficient
        let mut low = ThresholdCommitment::new("Low value", policy.clone()).with_value(500);
        low.add_signature(CommitmentSignature::new(ResonatorId::new("res-1")));
        assert!(low.is_satisfied());

        // Medium value: needs 2 of 3
        let mut med = ThresholdCommitment::new("Med value", policy.clone()).with_value(5000);
        med.add_signature(CommitmentSignature::new(ResonatorId::new("res-1")));
        assert!(!med.is_satisfied());
        med.add_signature(CommitmentSignature::new(ResonatorId::new("res-2")));
        assert!(med.is_satisfied());
    }

    #[test]
    fn test_threshold_commitment_id() {
        let id = ThresholdCommitmentId::generate();
        assert!(!id.0.is_empty());
        assert_eq!(
            format!("{}", ThresholdCommitmentId::new("tc-1")),
            "tc-1"
        );
    }
}
