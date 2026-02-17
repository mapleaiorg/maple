use maple_waf_context_graph::{ContentHash, GovernanceTier};
use maple_waf_evidence::EquivalenceTier;
use serde::{Deserialize, Serialize};

/// An upgrade proposal submitted to the swap gate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpgradeProposal {
    /// Hash of the artifact to install.
    pub artifact_hash: ContentHash,
    /// Hash of the evidence bundle.
    pub evidence_hash: ContentHash,
    /// Hash of the delta that produced the artifact.
    pub delta_hash: ContentHash,
    /// Governance tier required.
    pub governance_tier: GovernanceTier,
    /// Equivalence tier to verify.
    pub equivalence_tier: EquivalenceTier,
    /// Description of the upgrade.
    pub description: String,
}

impl UpgradeProposal {
    pub fn new(
        artifact_hash: ContentHash,
        evidence_hash: ContentHash,
        delta_hash: ContentHash,
    ) -> Self {
        Self {
            artifact_hash,
            evidence_hash,
            delta_hash,
            governance_tier: GovernanceTier::Tier0,
            equivalence_tier: EquivalenceTier::E0,
            description: String::new(),
        }
    }

    pub fn with_governance_tier(mut self, tier: GovernanceTier) -> Self {
        self.governance_tier = tier;
        self
    }

    pub fn with_equivalence_tier(mut self, tier: EquivalenceTier) -> Self {
        self.equivalence_tier = tier;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }
}

/// Result of a swap gate operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SwapResult {
    /// Successfully swapped.
    Swapped {
        artifact_hash: ContentHash,
        snapshot_hash: ContentHash,
    },
    /// Denied by governance.
    Denied(String),
    /// Rolled back after degradation.
    RolledBack {
        reason: String,
        restored_snapshot: ContentHash,
    },
}

impl SwapResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Swapped { .. })
    }
}

/// A snapshot of the current system state for rollback.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub hash: ContentHash,
    pub timestamp_ms: u64,
    pub description: String,
    /// Serialized state (simulated as bytes).
    pub state: Vec<u8>,
}

impl Snapshot {
    pub fn new(state: Vec<u8>, description: impl Into<String>) -> Self {
        let hash = ContentHash::hash(&state);
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_millis() as u64;
        Self {
            hash,
            timestamp_ms,
            description: description.into(),
            state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_proposal_builder() {
        let p = UpgradeProposal::new(
            ContentHash::hash(b"art"),
            ContentHash::hash(b"evi"),
            ContentHash::hash(b"delta"),
        )
        .with_governance_tier(GovernanceTier::Tier2)
        .with_equivalence_tier(EquivalenceTier::E1)
        .with_description("optimize allocator");
        assert_eq!(p.governance_tier, GovernanceTier::Tier2);
        assert_eq!(p.equivalence_tier, EquivalenceTier::E1);
        assert_eq!(p.description, "optimize allocator");
    }

    #[test]
    fn swap_result_success() {
        let r = SwapResult::Swapped {
            artifact_hash: ContentHash::hash(b"a"),
            snapshot_hash: ContentHash::hash(b"s"),
        };
        assert!(r.is_success());
    }

    #[test]
    fn swap_result_denied() {
        let r = SwapResult::Denied("insufficient tier".into());
        assert!(!r.is_success());
    }

    #[test]
    fn swap_result_rollback() {
        let r = SwapResult::RolledBack {
            reason: "degradation".into(),
            restored_snapshot: ContentHash::hash(b"snap"),
        };
        assert!(!r.is_success());
    }

    #[test]
    fn snapshot_content_addressed() {
        let s = Snapshot::new(vec![1, 2, 3], "test snapshot");
        let expected = ContentHash::hash(&[1, 2, 3]);
        assert_eq!(s.hash, expected);
    }

    #[test]
    fn proposal_serde() {
        let p = UpgradeProposal::new(
            ContentHash::hash(b"a"),
            ContentHash::hash(b"e"),
            ContentHash::hash(b"d"),
        );
        let json = serde_json::to_string(&p).unwrap();
        let restored: UpgradeProposal = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.artifact_hash, p.artifact_hash);
    }
}
