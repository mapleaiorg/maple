use maple_kernel_profiles::{financial_profile, ibank_platform, ProfileType, WorldlineProfile};
use maple_mwl_types::{CommitmentId, TemporalAnchor, WorldlineId};
use tracing::debug;

use crate::ares::FinancialGateExtension;
use crate::error::FinancialError;
use crate::evos::BalanceProjection;
use crate::regulatory::RegulatoryEngine;
use crate::types::{AssetId, FinancialCommitment, ProjectedBalance, SettlementType};

/// iBank integration — connects the kernel financial extensions to the
/// existing iBank infrastructure.
///
/// iBank becomes a Financial-profile worldline using ARES settlement.
/// This struct provides the bridge between the MWL kernel layer and
/// the iBank-specific runtime (crates/ibank/).
///
/// Key mappings:
/// - iBank `amount_minor: u64` → kernel `amount_minor: i64` (signed for debits)
/// - iBank `TransferIntent` → kernel `FinancialCommitment`
/// - iBank `BalanceRecord` → kernel `ProjectedBalance` (recomputed from trajectory)
/// - iBank `RiskPolicyConfig` → kernel `RegulatoryEngine`
/// - iBank `ComplianceDecision` → kernel regulatory check results
pub struct IBankBridge {
    /// ARES financial gate extension
    pub ares: FinancialGateExtension,
    /// EVOS balance projection engine
    pub evos: BalanceProjection,
    /// The iBank platform profile configuration
    profile: WorldlineProfile,
}

impl IBankBridge {
    /// Create a new iBank bridge with iBank-specific defaults.
    ///
    /// Uses the iBank platform configuration from maple-kernel-profiles:
    /// - Financial profile with enhanced strictness
    /// - iBank-specific AML thresholds
    /// - Conservative risk tolerance
    pub fn new() -> Self {
        // Use the iBank platform's financial profile
        let platform = ibank_platform();
        let profile = platform
            .active_profiles
            .iter()
            .find(|p| p.profile_type == ProfileType::Financial)
            .cloned()
            .unwrap_or_else(financial_profile);

        // Create regulatory engine with iBank-tuned settings
        let mut regulatory = RegulatoryEngine::new();

        // iBank-specific: stricter AML thresholds
        // Maps to iBank's pure_ai_max_amount_minor: 1_000_000 ($10K)
        // and hard_limit_amount_minor: 25_000_000 ($250K)
        regulatory.aml_mut().enhanced_due_diligence_threshold = 1_000_000;
        regulatory.aml_mut().block_threshold = 25_000_000;

        // iBank-specific: Basel III capital ratio
        regulatory.capital_mut().minimum_ratio = 0.08;

        let ares = FinancialGateExtension::with_regulatory(regulatory);

        Self {
            ares,
            evos: BalanceProjection::new(),
            profile,
        }
    }

    /// Get the financial profile used by this bridge.
    pub fn profile(&self) -> &WorldlineProfile {
        &self.profile
    }

    /// Convert an iBank-style transfer request to a FinancialCommitment.
    ///
    /// Maps iBank's `TransferIntent` fields to the kernel commitment model:
    /// - `origin_actor` → `declaring_identity` (as WorldlineId)
    /// - `counterparty_actor` → `counterparty` (as WorldlineId)
    /// - `amount_minor` → `amount_minor` (u64 → i64)
    /// - `currency` → `asset`
    /// - `transaction_type` → `settlement_type`
    /// - `decision_receipt_id` → explicit commitment-boundary linkage
    pub fn create_financial_commitment(
        declaring_identity: WorldlineId,
        counterparty: WorldlineId,
        asset: AssetId,
        amount_minor: u64,
        settlement_type: SettlementType,
        decision_receipt_id: impl Into<String>,
    ) -> FinancialCommitment {
        FinancialCommitment {
            commitment_id: CommitmentId::new(),
            asset,
            amount_minor: amount_minor as i64,
            settlement_type,
            counterparty,
            declaring_identity,
            decision_receipt_id: decision_receipt_id.into(),
            created_at: TemporalAnchor::now(0),
        }
    }

    /// Run the full iBank pre-check pipeline on a financial commitment.
    ///
    /// This mirrors iBank's orchestration flow:
    /// 1. ARES collateral check
    /// 2. ARES regulatory check (AML, sanctions, capital, position limits)
    ///
    /// After pre-check passes, the commitment can be submitted to the
    /// standard Commitment Gate for the 7-stage pipeline.
    pub fn pre_check(&self, commitment: &FinancialCommitment) -> Result<(), FinancialError> {
        debug!(
            asset = %commitment.asset,
            amount = commitment.amount_minor,
            "Running iBank pre-check pipeline"
        );

        self.ares.pre_check(commitment)
    }

    /// Get the projected balance for a worldline+asset.
    ///
    /// Per I.ME-FIN-1: balance is computed from trajectory, not stored.
    /// This replaces iBank's `BalanceRecord.available_minor` with a
    /// trajectory-based projection.
    pub fn projected_balance(
        &self,
        worldline: &WorldlineId,
        asset: &AssetId,
    ) -> Result<ProjectedBalance, FinancialError> {
        self.evos.project(worldline, asset)
    }

    /// Map iBank transaction type string to settlement type.
    ///
    /// iBank uses string-based transaction types like "transfer", "payment", etc.
    /// This maps them to the kernel's typed SettlementType enum.
    pub fn map_settlement_type(transaction_type: &str) -> SettlementType {
        match transaction_type.to_lowercase().as_str() {
            "dvp" | "delivery_vs_payment" => SettlementType::DvP,
            "pvp" | "payment_vs_payment" | "fx" => SettlementType::PvP,
            _ => SettlementType::FreeOfPayment,
        }
    }
}

impl Default for IBankBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CollateralRecord, SettlementEvent};
    use maple_mwl_types::IdentityMaterial;

    fn wid_a() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn wid_b() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]))
    }

    fn usd() -> AssetId {
        AssetId::new("USD")
    }

    #[test]
    fn ibank_bridge_uses_financial_profile() {
        let bridge = IBankBridge::new();
        assert_eq!(bridge.profile().profile_type, ProfileType::Financial);
    }

    #[test]
    fn ibank_bridge_has_strict_aml_thresholds() {
        let _initial = IBankBridge::new();
        // Should block at $250K (25M minor units)
        let commitment = IBankBridge::create_financial_commitment(
            wid_a(),
            wid_b(),
            usd(),
            30_000_000, // $300K
            SettlementType::FreeOfPayment,
            "dec-rcpt-ibank-aml-1",
        );

        // Register collateral so we don't fail on collateral check
        // Need mutable access, so create fresh
        let mut bridge = IBankBridge::new();
        bridge.ares.register_collateral(CollateralRecord {
            worldline: wid_a(),
            asset: usd(),
            available_minor: 100_000_000,
            locked_minor: 0,
        });

        let result = bridge.pre_check(&commitment);
        assert!(matches!(result, Err(FinancialError::AmlViolation { .. })));
    }

    #[test]
    fn ibank_bridge_allows_small_transfers() {
        let mut bridge = IBankBridge::new();
        bridge.ares.register_collateral(CollateralRecord {
            worldline: wid_a(),
            asset: usd(),
            available_minor: 100_000,
            locked_minor: 0,
        });

        let commitment = IBankBridge::create_financial_commitment(
            wid_a(),
            wid_b(),
            usd(),
            50_000, // $500
            SettlementType::FreeOfPayment,
            "dec-rcpt-ibank-small-1",
        );

        assert!(bridge.pre_check(&commitment).is_ok());
    }

    #[test]
    fn create_financial_commitment_maps_correctly() {
        let fc = IBankBridge::create_financial_commitment(
            wid_a(),
            wid_b(),
            usd(),
            100_000,
            SettlementType::DvP,
            "dec-rcpt-ibank-map-1",
        );

        assert_eq!(fc.amount_minor, 100_000);
        assert_eq!(fc.asset, usd());
        assert_eq!(fc.settlement_type, SettlementType::DvP);
        assert_eq!(fc.declaring_identity, wid_a());
        assert_eq!(fc.counterparty, wid_b());
        assert_eq!(fc.decision_receipt_id, "dec-rcpt-ibank-map-1");
    }

    #[test]
    fn map_settlement_type_dvp() {
        assert_eq!(IBankBridge::map_settlement_type("dvp"), SettlementType::DvP);
        assert_eq!(
            IBankBridge::map_settlement_type("delivery_vs_payment"),
            SettlementType::DvP
        );
    }

    #[test]
    fn map_settlement_type_pvp() {
        assert_eq!(IBankBridge::map_settlement_type("pvp"), SettlementType::PvP);
        assert_eq!(IBankBridge::map_settlement_type("fx"), SettlementType::PvP);
    }

    #[test]
    fn map_settlement_type_free_of_payment() {
        assert_eq!(
            IBankBridge::map_settlement_type("transfer"),
            SettlementType::FreeOfPayment
        );
        assert_eq!(
            IBankBridge::map_settlement_type("payment"),
            SettlementType::FreeOfPayment
        );
    }

    #[test]
    fn projected_balance_from_trajectory() {
        let mut bridge = IBankBridge::new();

        // Record some settlements
        bridge.evos.record_for_worldline(
            wid_a(),
            SettlementEvent {
                settlement_id: "s1".into(),
                commitment_id: CommitmentId::new(),
                asset: usd(),
                amount_minor: 100_000,
                counterparty: wid_b(),
                settled_at: TemporalAnchor::now(0),
                settlement_type: SettlementType::FreeOfPayment,
            },
        );
        bridge.evos.record_for_worldline(
            wid_a(),
            SettlementEvent {
                settlement_id: "s2".into(),
                commitment_id: CommitmentId::new(),
                asset: usd(),
                amount_minor: -30_000,
                counterparty: wid_b(),
                settled_at: TemporalAnchor::now(0),
                settlement_type: SettlementType::FreeOfPayment,
            },
        );

        let balance = bridge.projected_balance(&wid_a(), &usd()).unwrap();
        assert_eq!(balance.balance_minor, 70_000);
        assert_eq!(balance.trajectory_length, 2);
    }

    #[test]
    fn projected_balance_fails_for_no_trajectory() {
        let bridge = IBankBridge::new();
        assert!(matches!(
            bridge.projected_balance(&wid_a(), &usd()),
            Err(FinancialError::EmptyTrajectory(_))
        ));
    }
}
