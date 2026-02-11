use serde::{Deserialize, Serialize};

use crate::temporal::TemporalAnchor;
use crate::worldline_id::WorldlineId;

/// Commitment status lifecycle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CommitmentStatus {
    /// Declared but not yet adjudicated
    Pending,
    /// Approved by AAS, awaiting execution
    Approved,
    /// Currently being executed
    Active,
    /// Successfully completed
    Fulfilled,
    /// Failed with explicit reason
    Failed(FailureReason),
    /// Revoked before completion
    Revoked { by: WorldlineId, reason: String },
    /// Expired without execution
    Expired,
    /// Denied by AAS during adjudication
    Denied(DenialReason),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FailureReason {
    pub code: String,
    pub message: String,
    /// 0.0 to 1.0 — how much of the commitment was completed before failure
    pub partial_completion: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenialReason {
    pub code: String,
    pub message: String,
    pub policy_refs: Vec<String>,
}

/// Reversibility levels for commitments.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Reversibility {
    FullyReversible,
    Conditional { conditions: Vec<String> },
    TimeWindow { window_ms: u64 },
    Irreversible,
}

/// Commitment scope — what is being committed to.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentScope {
    pub effect_domain: EffectDomain,
    pub targets: Vec<WorldlineId>,
    pub constraints: Vec<String>,
}

/// Effect domains — what kind of world-state change.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectDomain {
    Communication,
    Financial,
    Infrastructure,
    Governance,
    DataMutation,
    Custom(String),
}

/// Temporal bounds for commitments.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemporalBounds {
    pub starts: TemporalAnchor,
    pub expires: Option<TemporalAnchor>,
    pub review_at: Option<TemporalAnchor>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_commitment_status_variants_serialize() {
        let wid = WorldlineId::ephemeral();
        let statuses: Vec<CommitmentStatus> = vec![
            CommitmentStatus::Pending,
            CommitmentStatus::Approved,
            CommitmentStatus::Active,
            CommitmentStatus::Fulfilled,
            CommitmentStatus::Failed(FailureReason {
                code: "TIMEOUT".into(),
                message: "Operation timed out".into(),
                partial_completion: Some(0.75),
            }),
            CommitmentStatus::Revoked {
                by: wid,
                reason: "No longer needed".into(),
            },
            CommitmentStatus::Expired,
            CommitmentStatus::Denied(DenialReason {
                code: "POLICY_VIOLATION".into(),
                message: "Exceeds risk threshold".into(),
                policy_refs: vec!["POL-001".into()],
            }),
        ];

        for status in &statuses {
            let json = serde_json::to_string(status).unwrap();
            let restored: CommitmentStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*status, restored);
        }
    }

    #[test]
    fn failure_reason_includes_partial_completion() {
        let reason = FailureReason {
            code: "ERR".into(),
            message: "partial".into(),
            partial_completion: Some(0.5),
        };
        assert_eq!(reason.partial_completion, Some(0.5));

        let json = serde_json::to_string(&reason).unwrap();
        let restored: FailureReason = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.partial_completion, Some(0.5));
    }

    #[test]
    fn reversibility_variants_serialize() {
        let variants: Vec<Reversibility> = vec![
            Reversibility::FullyReversible,
            Reversibility::Conditional {
                conditions: vec!["within 24h".into()],
            },
            Reversibility::TimeWindow { window_ms: 3600000 },
            Reversibility::Irreversible,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let restored: Reversibility = serde_json::from_str(&json).unwrap();
            assert_eq!(*v, restored);
        }
    }

    #[test]
    fn effect_domain_variants() {
        let domains = vec![
            EffectDomain::Communication,
            EffectDomain::Financial,
            EffectDomain::Infrastructure,
            EffectDomain::Governance,
            EffectDomain::DataMutation,
            EffectDomain::Custom("testing".into()),
        ];
        for d in &domains {
            let json = serde_json::to_string(d).unwrap();
            let restored: EffectDomain = serde_json::from_str(&json).unwrap();
            assert_eq!(*d, restored);
        }
    }
}
