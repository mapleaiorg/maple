use thiserror::Error;

use crate::types::AssetId;

/// Errors from the financial extension (ARES).
#[derive(Error, Debug)]
pub enum FinancialError {
    // --- Collateral errors ---
    #[error("insufficient collateral: required {required} minor units of {asset}, available {available}")]
    InsufficientCollateral {
        asset: AssetId,
        required: i64,
        available: i64,
    },

    #[error("collateral asset not recognized: {0}")]
    UnrecognizedAsset(AssetId),

    // --- DvP / Settlement errors ---
    #[error("DvP atomicity violation: {message}")]
    DvPViolation { message: String },

    #[error("partial settlement rejected (I.CEP-FIN-1): {message}")]
    PartialSettlement { message: String },

    #[error("settlement leg mismatch: expected {expected} legs, got {actual}")]
    LegMismatch { expected: usize, actual: usize },

    #[error("settlement counterparty not found: {0}")]
    CounterpartyNotFound(String),

    #[error("missing commitment decision receipt link for financial settlement")]
    MissingDecisionReceiptLink,

    #[error("invalid commitment decision receipt link: {receipt_id}")]
    InvalidDecisionReceiptLink { receipt_id: String },

    #[error("settlement does not align with commitment {commitment_id}: {message}")]
    SettlementCommitmentMismatch {
        commitment_id: String,
        message: String,
    },

    #[error(
        "settlement legs do not match commitment parties: expected {declaring_identity} <-> {counterparty}"
    )]
    SettlementPartiesMismatch {
        declaring_identity: String,
        counterparty: String,
    },

    // --- Regulatory errors ---
    #[error("regulatory violation: {policy} — {message}")]
    RegulatoryViolation { policy: String, message: String },

    #[error("AML screening failed: {reason}")]
    AmlViolation { reason: String },

    #[error("sanctions hit: party {party} matches sanctions list")]
    SanctionsHit { party: String },

    #[error("capital adequacy breach: ratio {ratio:.4} below minimum {minimum:.4}")]
    CapitalAdequacy { ratio: f64, minimum: f64 },

    #[error("position limit exceeded: {asset} position {current} would exceed limit {limit}")]
    PositionLimitExceeded {
        asset: AssetId,
        current: i64,
        limit: i64,
    },

    #[error("circuit breaker active: {reason}")]
    CircuitBreakerActive { reason: String },

    // --- Balance projection errors ---
    #[error("balance projection failed: {reason}")]
    ProjectionFailed { reason: String },

    #[error("trajectory contains no settlements for asset {0}")]
    EmptyTrajectory(AssetId),

    // --- Liquidity errors ---
    #[error("insufficient liquidity: {message}")]
    InsufficientLiquidity { message: String },
}

/// Result of a financial check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FinancialCheckResult {
    /// Check passed
    Passed,
    /// Check passed with warnings
    PassedWithWarnings(Vec<String>),
    /// Check blocked the operation
    Blocked { policy: String, reason: String },
}

impl FinancialCheckResult {
    pub fn is_passed(&self) -> bool {
        matches!(
            self,
            FinancialCheckResult::Passed | FinancialCheckResult::PassedWithWarnings(_)
        )
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, FinancialCheckResult::Blocked { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_result_predicates() {
        assert!(FinancialCheckResult::Passed.is_passed());
        assert!(!FinancialCheckResult::Passed.is_blocked());

        let warnings = FinancialCheckResult::PassedWithWarnings(vec!["test".into()]);
        assert!(warnings.is_passed());

        let blocked = FinancialCheckResult::Blocked {
            policy: "AML".into(),
            reason: "high risk".into(),
        };
        assert!(blocked.is_blocked());
        assert!(!blocked.is_passed());
    }

    #[test]
    fn error_display() {
        let err = FinancialError::InsufficientCollateral {
            asset: AssetId("USD".into()),
            required: 10000,
            available: 5000,
        };
        let s = err.to_string();
        assert!(s.contains("10000"));
        assert!(s.contains("5000"));
        assert!(s.contains("USD"));
    }

    #[test]
    fn dvp_violation_display() {
        let err = FinancialError::DvPViolation {
            message: "leg 2 failed".into(),
        };
        assert!(err.to_string().contains("leg 2 failed"));
    }

    #[test]
    fn partial_settlement_references_invariant() {
        let err = FinancialError::PartialSettlement {
            message: "only 1 of 2 legs settled".into(),
        };
        assert!(err.to_string().contains("I.CEP-FIN-1"));
    }
}
