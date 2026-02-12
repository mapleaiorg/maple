use std::collections::HashMap;

use maple_mwl_types::WorldlineId;
use tracing::{debug, info, warn};

use crate::error::FinancialError;
use crate::regulatory::RegulatoryEngine;
use crate::types::{
    AssetId, AtomicSettlement, CollateralRecord, FinancialCommitment, SettledLeg, SettlementLeg,
};
use maple_mwl_types::TemporalAnchor;

/// ARES — the Commitment Gate specialized for financial operations.
///
/// ARES extends the Gate pipeline with three financial-specific pre-checks:
/// 1. **Collateral sufficiency** — does the declaring identity have enough?
/// 2. **DvP/PvP atomicity** — all legs settle or none do (I.CEP-FIN-1)
/// 3. **Regulatory compliance** — AML, sanctions, capital adequacy, position limits
///
/// ARES does NOT replace the Gate — it runs before or alongside the standard
/// 7-stage pipeline to add financial-specific validation.
pub struct FinancialGateExtension {
    /// Collateral balances for each worldline+asset
    collateral: HashMap<(WorldlineId, AssetId), CollateralRecord>,
    /// Regulatory engine for compliance checks
    regulatory: RegulatoryEngine,
}

impl FinancialGateExtension {
    /// Create a new financial gate extension.
    pub fn new() -> Self {
        Self {
            collateral: HashMap::new(),
            regulatory: RegulatoryEngine::new(),
        }
    }

    /// Create with a custom regulatory engine.
    pub fn with_regulatory(regulatory: RegulatoryEngine) -> Self {
        Self {
            collateral: HashMap::new(),
            regulatory,
        }
    }

    /// Register collateral for a worldline.
    pub fn register_collateral(&mut self, record: CollateralRecord) {
        let key = (record.worldline.clone(), record.asset.clone());
        self.collateral.insert(key, record);
    }

    /// Get the regulatory engine (for adding policies).
    pub fn regulatory_mut(&mut self) -> &mut RegulatoryEngine {
        &mut self.regulatory
    }

    /// Check collateral sufficiency before settlement.
    ///
    /// Verifies that the declaring identity has enough unlocked collateral
    /// of the specified asset to cover the commitment amount.
    pub fn collateral_check(
        &self,
        commitment: &FinancialCommitment,
    ) -> Result<(), FinancialError> {
        let key = (
            commitment.declaring_identity.clone(),
            commitment.asset.clone(),
        );

        let record = self.collateral.get(&key).ok_or_else(|| {
            FinancialError::InsufficientCollateral {
                asset: commitment.asset.clone(),
                required: commitment.amount_minor,
                available: 0,
            }
        })?;

        let available = record.available_minor - record.locked_minor;
        if available < commitment.amount_minor {
            warn!(
                asset = %commitment.asset,
                required = commitment.amount_minor,
                available = available,
                "Collateral check failed"
            );
            return Err(FinancialError::InsufficientCollateral {
                asset: commitment.asset.clone(),
                required: commitment.amount_minor,
                available,
            });
        }

        debug!(
            asset = %commitment.asset,
            amount = commitment.amount_minor,
            available = available,
            "Collateral check passed"
        );
        Ok(())
    }

    /// DvP atomicity: both legs settle or neither does.
    ///
    /// Per I.CEP-FIN-1: "DvP/PvP required. Partial settlement = violation."
    ///
    /// This function simulates atomic settlement of all legs. In a real system,
    /// this would coordinate with external settlement rails. Here we validate
    /// that all legs are valid and would settle atomically.
    pub fn dvp_atomicity(
        &self,
        legs: &[SettlementLeg],
    ) -> Result<AtomicSettlement, FinancialError> {
        if legs.is_empty() {
            return Err(FinancialError::DvPViolation {
                message: "No settlement legs provided".into(),
            });
        }

        // For DvP, we need exactly 2 legs (delivery + payment)
        // For PvP, we need exactly 2 legs (payment + payment)
        // Validate all legs have positive amounts
        for (i, leg) in legs.iter().enumerate() {
            if leg.amount_minor <= 0 {
                return Err(FinancialError::DvPViolation {
                    message: format!("Leg {} has non-positive amount: {}", i, leg.amount_minor),
                });
            }
        }

        // Validate that legs form a valid pair:
        // Leg 0: A -> B (asset X)
        // Leg 1: B -> A (asset Y) [or same asset for PvP]
        if legs.len() >= 2 {
            let leg_a = &legs[0];
            let leg_b = &legs[1];

            // The two legs should be between the same parties (reversed)
            if leg_a.from != leg_b.to || leg_a.to != leg_b.from {
                return Err(FinancialError::DvPViolation {
                    message: "Settlement legs are not between the same counterparties".into(),
                });
            }
        }

        // Simulate atomic settlement — all legs succeed
        let settled_legs: Vec<SettledLeg> = legs
            .iter()
            .map(|leg| SettledLeg {
                leg: leg.clone(),
                settled: true,
                reference: Some(uuid::Uuid::new_v4().to_string()),
            })
            .collect();

        let settlement = AtomicSettlement {
            settlement_id: uuid::Uuid::new_v4().to_string(),
            legs: settled_legs,
            settled_at: TemporalAnchor::now(0),
            atomic: true,
        };

        info!(
            settlement_id = %settlement.settlement_id,
            legs = legs.len(),
            "DvP atomic settlement completed"
        );

        Ok(settlement)
    }

    /// Validate that a settlement is truly atomic — no partial settlement.
    ///
    /// Per I.CEP-FIN-1: partial settlement is a violation.
    pub fn validate_atomicity(settlement: &AtomicSettlement) -> Result<(), FinancialError> {
        let all_settled = settlement.legs.iter().all(|l| l.settled);
        let none_settled = settlement.legs.iter().all(|l| !l.settled);

        if !all_settled && !none_settled {
            // Partial settlement — violation!
            let settled_count = settlement.legs.iter().filter(|l| l.settled).count();
            let total = settlement.legs.len();
            return Err(FinancialError::PartialSettlement {
                message: format!(
                    "Only {} of {} legs settled — I.CEP-FIN-1 violation",
                    settled_count, total
                ),
            });
        }

        if !settlement.atomic {
            return Err(FinancialError::DvPViolation {
                message: "Settlement not marked as atomic".into(),
            });
        }

        Ok(())
    }

    /// Regulatory compliance check — delegates to the regulatory engine.
    pub fn regulatory_check(
        &self,
        commitment: &FinancialCommitment,
    ) -> Result<(), FinancialError> {
        self.regulatory.check_compliance(commitment)
    }

    /// Run all financial pre-checks for a commitment.
    ///
    /// This is the main entry point for ARES validation:
    /// 1. Collateral check
    /// 2. Regulatory check
    /// (DvP atomicity is checked during settlement execution)
    pub fn pre_check(
        &self,
        commitment: &FinancialCommitment,
    ) -> Result<(), FinancialError> {
        self.collateral_check(commitment)?;
        self.regulatory_check(commitment)?;
        Ok(())
    }
}

impl Default for FinancialGateExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SettlementType;
    use maple_mwl_types::{CommitmentId, IdentityMaterial};

    fn wid_a() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn wid_b() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]))
    }

    fn usd() -> AssetId {
        AssetId::new("USD")
    }

    fn btc() -> AssetId {
        AssetId::new("BTC")
    }

    fn test_commitment(amount: i64) -> FinancialCommitment {
        FinancialCommitment {
            commitment_id: CommitmentId::new(),
            asset: usd(),
            amount_minor: amount,
            settlement_type: SettlementType::DvP,
            counterparty: wid_b(),
            declaring_identity: wid_a(),
            created_at: TemporalAnchor::now(0),
        }
    }

    #[test]
    fn collateral_check_passes_when_sufficient() {
        let mut ext = FinancialGateExtension::new();
        ext.register_collateral(CollateralRecord {
            worldline: wid_a(),
            asset: usd(),
            available_minor: 100_000,
            locked_minor: 0,
        });

        let commitment = test_commitment(50_000);
        assert!(ext.collateral_check(&commitment).is_ok());
    }

    #[test]
    fn collateral_check_fails_when_insufficient() {
        let mut ext = FinancialGateExtension::new();
        ext.register_collateral(CollateralRecord {
            worldline: wid_a(),
            asset: usd(),
            available_minor: 30_000,
            locked_minor: 0,
        });

        let commitment = test_commitment(50_000);
        assert!(matches!(
            ext.collateral_check(&commitment),
            Err(FinancialError::InsufficientCollateral { .. })
        ));
    }

    #[test]
    fn collateral_check_accounts_for_locked() {
        let mut ext = FinancialGateExtension::new();
        ext.register_collateral(CollateralRecord {
            worldline: wid_a(),
            asset: usd(),
            available_minor: 100_000,
            locked_minor: 60_000,
        });

        // Only 40k available after lock
        let commitment = test_commitment(50_000);
        assert!(matches!(
            ext.collateral_check(&commitment),
            Err(FinancialError::InsufficientCollateral { .. })
        ));
    }

    #[test]
    fn collateral_check_fails_when_no_record() {
        let ext = FinancialGateExtension::new();
        let commitment = test_commitment(50_000);
        assert!(matches!(
            ext.collateral_check(&commitment),
            Err(FinancialError::InsufficientCollateral { .. })
        ));
    }

    #[test]
    fn dvp_atomicity_succeeds_for_valid_pair() {
        let ext = FinancialGateExtension::new();
        let legs = vec![
            SettlementLeg {
                from: wid_a(),
                to: wid_b(),
                asset: usd(),
                amount_minor: 100_000,
            },
            SettlementLeg {
                from: wid_b(),
                to: wid_a(),
                asset: btc(),
                amount_minor: 1_000_000,
            },
        ];

        let result = ext.dvp_atomicity(&legs).unwrap();
        assert!(result.atomic);
        assert_eq!(result.legs.len(), 2);
        assert!(result.legs.iter().all(|l| l.settled));
    }

    #[test]
    fn dvp_atomicity_rejects_empty_legs() {
        let ext = FinancialGateExtension::new();
        assert!(matches!(
            ext.dvp_atomicity(&[]),
            Err(FinancialError::DvPViolation { .. })
        ));
    }

    #[test]
    fn dvp_atomicity_rejects_zero_amount() {
        let ext = FinancialGateExtension::new();
        let legs = vec![SettlementLeg {
            from: wid_a(),
            to: wid_b(),
            asset: usd(),
            amount_minor: 0,
        }];

        assert!(matches!(
            ext.dvp_atomicity(&legs),
            Err(FinancialError::DvPViolation { .. })
        ));
    }

    #[test]
    fn dvp_atomicity_rejects_mismatched_counterparties() {
        let ext = FinancialGateExtension::new();
        let wid_c = WorldlineId::derive(&IdentityMaterial::GenesisHash([3u8; 32]));
        let legs = vec![
            SettlementLeg {
                from: wid_a(),
                to: wid_b(),
                asset: usd(),
                amount_minor: 100_000,
            },
            SettlementLeg {
                from: wid_c, // Wrong! Should be wid_b
                to: wid_a(),
                asset: btc(),
                amount_minor: 1_000_000,
            },
        ];

        assert!(matches!(
            ext.dvp_atomicity(&legs),
            Err(FinancialError::DvPViolation { .. })
        ));
    }

    #[test]
    fn validate_atomicity_passes_for_all_settled() {
        let settlement = AtomicSettlement {
            settlement_id: "test".into(),
            legs: vec![
                SettledLeg {
                    leg: SettlementLeg {
                        from: wid_a(),
                        to: wid_b(),
                        asset: usd(),
                        amount_minor: 100_000,
                    },
                    settled: true,
                    reference: Some("ref1".into()),
                },
                SettledLeg {
                    leg: SettlementLeg {
                        from: wid_b(),
                        to: wid_a(),
                        asset: btc(),
                        amount_minor: 1_000_000,
                    },
                    settled: true,
                    reference: Some("ref2".into()),
                },
            ],
            settled_at: TemporalAnchor::now(0),
            atomic: true,
        };

        assert!(FinancialGateExtension::validate_atomicity(&settlement).is_ok());
    }

    #[test]
    fn validate_atomicity_rejects_partial_settlement() {
        let settlement = AtomicSettlement {
            settlement_id: "test".into(),
            legs: vec![
                SettledLeg {
                    leg: SettlementLeg {
                        from: wid_a(),
                        to: wid_b(),
                        asset: usd(),
                        amount_minor: 100_000,
                    },
                    settled: true,
                    reference: Some("ref1".into()),
                },
                SettledLeg {
                    leg: SettlementLeg {
                        from: wid_b(),
                        to: wid_a(),
                        asset: btc(),
                        amount_minor: 1_000_000,
                    },
                    settled: false, // This leg failed!
                    reference: None,
                },
            ],
            settled_at: TemporalAnchor::now(0),
            atomic: true,
        };

        assert!(matches!(
            FinancialGateExtension::validate_atomicity(&settlement),
            Err(FinancialError::PartialSettlement { .. })
        ));
    }

    #[test]
    fn validate_atomicity_passes_for_all_failed() {
        // All legs failing is NOT a partial settlement — it's a clean rollback
        let settlement = AtomicSettlement {
            settlement_id: "test".into(),
            legs: vec![
                SettledLeg {
                    leg: SettlementLeg {
                        from: wid_a(),
                        to: wid_b(),
                        asset: usd(),
                        amount_minor: 100_000,
                    },
                    settled: false,
                    reference: None,
                },
                SettledLeg {
                    leg: SettlementLeg {
                        from: wid_b(),
                        to: wid_a(),
                        asset: btc(),
                        amount_minor: 1_000_000,
                    },
                    settled: false,
                    reference: None,
                },
            ],
            settled_at: TemporalAnchor::now(0),
            atomic: true,
        };

        assert!(FinancialGateExtension::validate_atomicity(&settlement).is_ok());
    }

    #[test]
    fn pre_check_runs_collateral_and_regulatory() {
        let mut ext = FinancialGateExtension::new();
        ext.register_collateral(CollateralRecord {
            worldline: wid_a(),
            asset: usd(),
            available_minor: 100_000,
            locked_minor: 0,
        });

        let commitment = test_commitment(50_000);
        // No regulatory policies registered, so should pass
        assert!(ext.pre_check(&commitment).is_ok());
    }

    #[test]
    fn single_leg_settlement_succeeds() {
        let ext = FinancialGateExtension::new();
        let legs = vec![SettlementLeg {
            from: wid_a(),
            to: wid_b(),
            asset: usd(),
            amount_minor: 100_000,
        }];

        let result = ext.dvp_atomicity(&legs).unwrap();
        assert!(result.atomic);
        assert_eq!(result.legs.len(), 1);
    }
}
