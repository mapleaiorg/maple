use std::collections::HashSet;

use tracing::{debug, warn};

use crate::error::FinancialError;
use crate::types::{AssetId, FinancialCommitment};

/// Regulatory Engine — compliance checks for financial operations.
///
/// Implements governance policies as required by the financial extension:
/// - AML (Anti-Money Laundering) screening
/// - Sanctions list checking
/// - Capital adequacy requirements
/// - Position limits
/// - Circuit breaker under stress
pub struct RegulatoryEngine {
    /// AML screening config
    aml: AmlConfig,
    /// Sanctions list
    sanctions: SanctionsList,
    /// Capital adequacy config
    capital: CapitalConfig,
    /// Position limits per asset
    position_limits: PositionLimits,
    /// Circuit breaker state
    circuit_breaker: CircuitBreakerState,
}

/// AML screening configuration.
#[derive(Clone, Debug)]
pub struct AmlConfig {
    /// Amount threshold (minor units) above which enhanced due diligence is required
    pub enhanced_due_diligence_threshold: i64,
    /// Amount threshold (minor units) above which the transaction is blocked
    pub block_threshold: i64,
    /// Whether AML screening is enabled
    pub enabled: bool,
}

impl Default for AmlConfig {
    fn default() -> Self {
        Self {
            enhanced_due_diligence_threshold: 1_000_000, // $10,000
            block_threshold: 25_000_000,                  // $250,000
            enabled: true,
        }
    }
}

/// Sanctions list for compliance checking.
#[derive(Clone, Debug, Default)]
pub struct SanctionsList {
    /// Set of sanctioned party identifiers (worldline ID strings)
    pub sanctioned_parties: HashSet<String>,
}

impl SanctionsList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a party to the sanctions list.
    pub fn add_party(&mut self, party: impl Into<String>) {
        self.sanctioned_parties.insert(party.into());
    }

    /// Check if a party is sanctioned.
    pub fn is_sanctioned(&self, party: &str) -> bool {
        self.sanctioned_parties.contains(party)
    }
}

/// Capital adequacy configuration.
#[derive(Clone, Debug)]
pub struct CapitalConfig {
    /// Minimum capital ratio (e.g., 0.08 = 8% like Basel III)
    pub minimum_ratio: f64,
    /// Current capital available (minor units)
    pub current_capital: i64,
    /// Total risk-weighted assets (minor units)
    pub risk_weighted_assets: i64,
    /// Whether capital adequacy checking is enabled
    pub enabled: bool,
}

impl Default for CapitalConfig {
    fn default() -> Self {
        Self {
            minimum_ratio: 0.08,
            current_capital: 100_000_000, // $1M
            risk_weighted_assets: 500_000_000, // $5M
            enabled: true,
        }
    }
}

impl CapitalConfig {
    /// Current capital ratio.
    pub fn ratio(&self) -> f64 {
        if self.risk_weighted_assets == 0 {
            return 1.0;
        }
        self.current_capital as f64 / self.risk_weighted_assets as f64
    }
}

/// Position limits per asset.
#[derive(Clone, Debug, Default)]
pub struct PositionLimits {
    /// Maximum position per asset (minor units)
    limits: std::collections::HashMap<AssetId, i64>,
    /// Current positions per asset (minor units)
    positions: std::collections::HashMap<AssetId, i64>,
}

impl PositionLimits {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a position limit for an asset.
    pub fn set_limit(&mut self, asset: AssetId, limit: i64) {
        self.limits.insert(asset, limit);
    }

    /// Update the current position for an asset.
    pub fn set_position(&mut self, asset: AssetId, position: i64) {
        self.positions.insert(asset, position);
    }

    /// Check if adding amount to the current position would exceed the limit.
    pub fn would_exceed(&self, asset: &AssetId, additional: i64) -> Option<(i64, i64)> {
        if let Some(&limit) = self.limits.get(asset) {
            let current = self.positions.get(asset).copied().unwrap_or(0);
            let new_position = current + additional;
            if new_position > limit {
                return Some((new_position, limit));
            }
        }
        None
    }
}

/// Circuit breaker state.
#[derive(Clone, Debug)]
pub struct CircuitBreakerState {
    /// Whether the circuit breaker is currently active
    pub active: bool,
    /// Reason for activation
    pub reason: Option<String>,
    /// Whether circuit breaker checking is enabled
    pub enabled: bool,
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self {
            active: false,
            reason: None,
            enabled: true,
        }
    }
}

impl RegulatoryEngine {
    /// Create a new regulatory engine with default configurations.
    pub fn new() -> Self {
        Self {
            aml: AmlConfig::default(),
            sanctions: SanctionsList::new(),
            capital: CapitalConfig::default(),
            position_limits: PositionLimits::new(),
            circuit_breaker: CircuitBreakerState::default(),
        }
    }

    /// Get mutable access to AML config.
    pub fn aml_mut(&mut self) -> &mut AmlConfig {
        &mut self.aml
    }

    /// Get mutable access to sanctions list.
    pub fn sanctions_mut(&mut self) -> &mut SanctionsList {
        &mut self.sanctions
    }

    /// Get mutable access to capital config.
    pub fn capital_mut(&mut self) -> &mut CapitalConfig {
        &mut self.capital
    }

    /// Get mutable access to position limits.
    pub fn position_limits_mut(&mut self) -> &mut PositionLimits {
        &mut self.position_limits
    }

    /// Activate the circuit breaker.
    pub fn activate_circuit_breaker(&mut self, reason: impl Into<String>) {
        self.circuit_breaker.active = true;
        self.circuit_breaker.reason = Some(reason.into());
        warn!(
            reason = self.circuit_breaker.reason.as_deref().unwrap_or("unknown"),
            "Circuit breaker activated"
        );
    }

    /// Deactivate the circuit breaker.
    pub fn deactivate_circuit_breaker(&mut self) {
        self.circuit_breaker.active = false;
        self.circuit_breaker.reason = None;
        debug!("Circuit breaker deactivated");
    }

    /// Is the circuit breaker active?
    pub fn is_circuit_breaker_active(&self) -> bool {
        self.circuit_breaker.enabled && self.circuit_breaker.active
    }

    /// Run all compliance checks on a financial commitment.
    pub fn check_compliance(
        &self,
        commitment: &FinancialCommitment,
    ) -> Result<(), FinancialError> {
        // 1. Circuit breaker check (highest priority)
        self.check_circuit_breaker()?;

        // 2. Sanctions check
        self.check_sanctions(commitment)?;

        // 3. AML check
        self.check_aml(commitment)?;

        // 4. Capital adequacy
        self.check_capital_adequacy(commitment)?;

        // 5. Position limits
        self.check_position_limits(commitment)?;

        debug!(
            asset = %commitment.asset,
            amount = commitment.amount_minor,
            "All regulatory checks passed"
        );

        Ok(())
    }

    /// Check circuit breaker.
    fn check_circuit_breaker(&self) -> Result<(), FinancialError> {
        if self.is_circuit_breaker_active() {
            return Err(FinancialError::CircuitBreakerActive {
                reason: self
                    .circuit_breaker
                    .reason
                    .clone()
                    .unwrap_or_else(|| "stress conditions".into()),
            });
        }
        Ok(())
    }

    /// Check sanctions list.
    fn check_sanctions(
        &self,
        commitment: &FinancialCommitment,
    ) -> Result<(), FinancialError> {
        let counterparty_str = format!("{}", commitment.counterparty);
        if self.sanctions.is_sanctioned(&counterparty_str) {
            return Err(FinancialError::SanctionsHit {
                party: counterparty_str,
            });
        }

        let declaring_str = format!("{}", commitment.declaring_identity);
        if self.sanctions.is_sanctioned(&declaring_str) {
            return Err(FinancialError::SanctionsHit {
                party: declaring_str,
            });
        }

        Ok(())
    }

    /// Check AML compliance.
    fn check_aml(
        &self,
        commitment: &FinancialCommitment,
    ) -> Result<(), FinancialError> {
        if !self.aml.enabled {
            return Ok(());
        }

        if commitment.amount_minor > self.aml.block_threshold {
            return Err(FinancialError::AmlViolation {
                reason: format!(
                    "Amount {} exceeds AML block threshold {}",
                    commitment.amount_minor, self.aml.block_threshold
                ),
            });
        }

        // Enhanced due diligence threshold is a warning-level check
        // that would trigger human review in a real system.
        // For now, we allow it but log.
        if commitment.amount_minor > self.aml.enhanced_due_diligence_threshold {
            debug!(
                amount = commitment.amount_minor,
                threshold = self.aml.enhanced_due_diligence_threshold,
                "Enhanced due diligence required"
            );
        }

        Ok(())
    }

    /// Check capital adequacy.
    fn check_capital_adequacy(
        &self,
        _commitment: &FinancialCommitment,
    ) -> Result<(), FinancialError> {
        if !self.capital.enabled {
            return Ok(());
        }

        let ratio = self.capital.ratio();
        if ratio < self.capital.minimum_ratio {
            return Err(FinancialError::CapitalAdequacy {
                ratio,
                minimum: self.capital.minimum_ratio,
            });
        }

        Ok(())
    }

    /// Check position limits.
    fn check_position_limits(
        &self,
        commitment: &FinancialCommitment,
    ) -> Result<(), FinancialError> {
        if let Some((current, limit)) = self
            .position_limits
            .would_exceed(&commitment.asset, commitment.amount_minor)
        {
            return Err(FinancialError::PositionLimitExceeded {
                asset: commitment.asset.clone(),
                current,
                limit,
            });
        }

        Ok(())
    }
}

impl Default for RegulatoryEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SettlementType;
    use maple_mwl_types::{CommitmentId, IdentityMaterial, TemporalAnchor, WorldlineId};

    fn wid_a() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn wid_b() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]))
    }

    fn usd() -> AssetId {
        AssetId::new("USD")
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
    fn aml_passes_below_threshold() {
        let engine = RegulatoryEngine::new();
        let commitment = test_commitment(500_000); // $5,000
        assert!(engine.check_compliance(&commitment).is_ok());
    }

    #[test]
    fn aml_blocks_above_threshold() {
        let engine = RegulatoryEngine::new();
        let commitment = test_commitment(30_000_000); // $300,000
        assert!(matches!(
            engine.check_compliance(&commitment),
            Err(FinancialError::AmlViolation { .. })
        ));
    }

    #[test]
    fn aml_disabled_allows_large_amounts() {
        let mut engine = RegulatoryEngine::new();
        engine.aml_mut().enabled = false;
        let commitment = test_commitment(100_000_000);
        // Other checks may still fail, but AML shouldn't
        // Disable everything to isolate AML
        engine.capital_mut().enabled = false;
        assert!(engine.check_compliance(&commitment).is_ok());
    }

    #[test]
    fn sanctions_blocks_sanctioned_party() {
        let mut engine = RegulatoryEngine::new();
        let counterparty_str = format!("{}", wid_b());
        engine.sanctions_mut().add_party(counterparty_str);

        let commitment = test_commitment(1_000);
        assert!(matches!(
            engine.check_compliance(&commitment),
            Err(FinancialError::SanctionsHit { .. })
        ));
    }

    #[test]
    fn sanctions_passes_for_clean_parties() {
        let mut engine = RegulatoryEngine::new();
        engine.sanctions_mut().add_party("some-other-party");

        let commitment = test_commitment(1_000);
        assert!(engine.check_compliance(&commitment).is_ok());
    }

    #[test]
    fn capital_adequacy_passes_when_sufficient() {
        let engine = RegulatoryEngine::new();
        // Default: 1M capital / 5M RWA = 20% > 8% minimum
        let commitment = test_commitment(1_000);
        assert!(engine.check_compliance(&commitment).is_ok());
    }

    #[test]
    fn capital_adequacy_fails_when_insufficient() {
        let mut engine = RegulatoryEngine::new();
        engine.capital_mut().current_capital = 10_000; // Very low capital
        engine.capital_mut().risk_weighted_assets = 500_000_000;

        let commitment = test_commitment(1_000);
        assert!(matches!(
            engine.check_compliance(&commitment),
            Err(FinancialError::CapitalAdequacy { .. })
        ));
    }

    #[test]
    fn position_limits_block_when_exceeded() {
        let mut engine = RegulatoryEngine::new();
        engine.position_limits_mut().set_limit(usd(), 1_000_000);
        engine.position_limits_mut().set_position(usd(), 800_000);

        // Would bring position to 1.1M > 1M limit
        let commitment = test_commitment(300_000);
        assert!(matches!(
            engine.check_compliance(&commitment),
            Err(FinancialError::PositionLimitExceeded { .. })
        ));
    }

    #[test]
    fn position_limits_allow_within_limit() {
        let mut engine = RegulatoryEngine::new();
        engine.position_limits_mut().set_limit(usd(), 1_000_000);
        engine.position_limits_mut().set_position(usd(), 500_000);

        let commitment = test_commitment(300_000);
        assert!(engine.check_compliance(&commitment).is_ok());
    }

    #[test]
    fn position_limits_skip_unlisted_assets() {
        let _engine = RegulatoryEngine::new();
        // No limits set for USD — should pass
        let commitment = test_commitment(999_999_999);
        // May fail AML but not position limits
        let mut clean_engine = RegulatoryEngine::new();
        clean_engine.aml_mut().enabled = false;
        clean_engine.capital_mut().enabled = false;
        assert!(clean_engine.check_compliance(&commitment).is_ok());
    }

    #[test]
    fn circuit_breaker_blocks_all_operations() {
        let mut engine = RegulatoryEngine::new();
        engine.activate_circuit_breaker("market stress");

        let commitment = test_commitment(1_000);
        assert!(matches!(
            engine.check_compliance(&commitment),
            Err(FinancialError::CircuitBreakerActive { .. })
        ));
    }

    #[test]
    fn circuit_breaker_deactivation_allows_operations() {
        let mut engine = RegulatoryEngine::new();
        engine.activate_circuit_breaker("market stress");
        assert!(engine.is_circuit_breaker_active());

        engine.deactivate_circuit_breaker();
        assert!(!engine.is_circuit_breaker_active());

        let commitment = test_commitment(1_000);
        assert!(engine.check_compliance(&commitment).is_ok());
    }

    #[test]
    fn circuit_breaker_check_first() {
        // Circuit breaker should block before other checks run
        let mut engine = RegulatoryEngine::new();
        engine.activate_circuit_breaker("stress");
        // Even with sanctioned party, circuit breaker error comes first
        let counterparty_str = format!("{}", wid_b());
        engine.sanctions_mut().add_party(counterparty_str);

        let commitment = test_commitment(1_000);
        assert!(matches!(
            engine.check_compliance(&commitment),
            Err(FinancialError::CircuitBreakerActive { .. })
        ));
    }

    #[test]
    fn capital_ratio_computed_correctly() {
        let mut config = CapitalConfig::default();
        config.current_capital = 80_000;
        config.risk_weighted_assets = 1_000_000;
        assert!((config.ratio() - 0.08).abs() < 0.001);

        config.risk_weighted_assets = 0;
        assert!((config.ratio() - 1.0).abs() < f64::EPSILON);
    }
}
