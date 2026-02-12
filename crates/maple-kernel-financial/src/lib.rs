//! # maple-kernel-financial
//!
//! Financial extensions for the Maple WorldLine Framework kernel:
//!
//! - **ARES** (Commitment Gate + Financial Extension) — collateral checks,
//!   DvP/PvP atomicity enforcement, regulatory compliance
//! - **EVOS** (Balance-as-Projection) — balance computed by replaying the
//!   committed settlement trajectory, NEVER stored
//! - **ERX** (Liquidity Field Operator) — reaction-diffusion model for
//!   settlement network liquidity and stress detection
//!
//! ## Constitutional Invariants
//!
//! - **I.CEP-FIN-1**: DvP/PvP required. Partial settlement = violation.
//! - **I.ME-FIN-1**: Balance computed from trajectory, not stored.
//!
//! ## Regulatory Policies
//!
//! - AML (Anti-Money Laundering) screening with configurable thresholds
//! - Sanctions list checking
//! - Capital adequacy requirements (Basel III inspired)
//! - Position limits per asset
//! - Circuit breaker under network stress
//!
//! ## iBank Integration
//!
//! The `IBankBridge` connects these kernel extensions to the existing iBank
//! infrastructure, mapping iBank's `TransferIntent` → `FinancialCommitment`,
//! `BalanceRecord` → `ProjectedBalance`, and `RiskPolicyConfig` → `RegulatoryEngine`.

pub mod ares;
pub mod erx;
pub mod error;
pub mod evos;
pub mod ibank;
pub mod regulatory;
pub mod types;

pub use ares::FinancialGateExtension;
pub use erx::{LiquidityConfig, LiquidityFieldOperator};
pub use error::{FinancialCheckResult, FinancialError};
pub use evos::BalanceProjection;
pub use ibank::IBankBridge;
pub use regulatory::{
    AmlConfig, CapitalConfig, CircuitBreakerState, PositionLimits, RegulatoryEngine, SanctionsList,
};
pub use types::{
    AssetId, AtomicSettlement, ChannelLiquidity, CollateralRecord, FinancialCommitment,
    LiquidityField, ProjectedBalance, SettlementChannel, SettlementEvent, SettlementLeg,
    SettlementNetwork, SettlementType, SettledLeg, StressLevel,
};
