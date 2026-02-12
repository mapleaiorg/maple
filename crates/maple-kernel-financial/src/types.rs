use maple_mwl_types::{CommitmentId, TemporalAnchor, WorldlineId};
use serde::{Deserialize, Serialize};

/// Asset identifier — a string wrapper for asset codes (e.g., "USD", "BTC", "ETH").
///
/// Consistent with existing iBank which uses `String` for currency/asset fields.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub String);

impl AssetId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Settlement type — how the financial commitment settles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementType {
    /// Delivery versus Payment — both legs must settle atomically
    DvP,
    /// Payment versus Payment — two payment legs settle atomically
    PvP,
    /// Free of Payment — single-leg settlement (no corresponding delivery)
    FreeOfPayment,
}

/// A financial commitment — extends CommitmentDeclaration with financial specifics.
///
/// Per ARES spec: financial commitments carry asset, amount, settlement type,
/// and counterparty information in addition to the base commitment fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinancialCommitment {
    /// Reference to the base commitment in the Gate
    pub commitment_id: CommitmentId,
    /// The asset being transacted
    pub asset: AssetId,
    /// Amount in minor units (cents, satoshis, etc.)
    /// Consistent with iBank's `amount_minor: u64` / `i64` convention.
    pub amount_minor: i64,
    /// Settlement type
    pub settlement_type: SettlementType,
    /// Counterparty worldline
    pub counterparty: WorldlineId,
    /// The declaring identity (payer/sender)
    pub declaring_identity: WorldlineId,
    /// When the commitment was created
    pub created_at: TemporalAnchor,
}

/// A settlement leg — one side of a DvP or PvP transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettlementLeg {
    /// Who is sending
    pub from: WorldlineId,
    /// Who is receiving
    pub to: WorldlineId,
    /// Which asset
    pub asset: AssetId,
    /// Amount in minor units
    pub amount_minor: i64,
}

/// Result of an atomic settlement (DvP/PvP).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AtomicSettlement {
    /// Settlement identifier
    pub settlement_id: String,
    /// The legs that settled
    pub legs: Vec<SettledLeg>,
    /// When settlement completed
    pub settled_at: TemporalAnchor,
    /// Whether settlement was fully atomic
    pub atomic: bool,
}

/// A settled leg — records what actually happened.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettledLeg {
    /// The original leg
    pub leg: SettlementLeg,
    /// Whether this leg settled successfully
    pub settled: bool,
    /// Settlement reference (external system ID)
    pub reference: Option<String>,
}

/// A settlement event in the trajectory (for balance projection).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettlementEvent {
    /// Settlement ID
    pub settlement_id: String,
    /// Associated commitment
    pub commitment_id: CommitmentId,
    /// Asset settled
    pub asset: AssetId,
    /// Amount (positive = credit, negative = debit)
    pub amount_minor: i64,
    /// Counterparty
    pub counterparty: WorldlineId,
    /// When settled
    pub settled_at: TemporalAnchor,
    /// Settlement type
    pub settlement_type: SettlementType,
}

/// Projected balance — computed from settlement trajectory, NOT stored.
///
/// Per I.ME-FIN-1: "Balance is not a stored number. It is a projection
/// computed by replaying the committed settlement trajectory."
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectedBalance {
    /// The worldline whose balance is projected
    pub worldline: WorldlineId,
    /// The asset
    pub asset: AssetId,
    /// Total balance in minor units (can be negative for debt)
    pub balance_minor: i64,
    /// Number of settlements in the trajectory
    pub trajectory_length: usize,
    /// When this projection was computed
    pub projected_at: TemporalAnchor,
    /// Hash of the trajectory used (for verification)
    pub trajectory_hash: String,
}

/// Settlement network — nodes and edges for liquidity field computation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettlementNetwork {
    /// All participant worldlines
    pub participants: Vec<WorldlineId>,
    /// Settlement channels between participants
    pub channels: Vec<SettlementChannel>,
}

/// A settlement channel between two participants.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettlementChannel {
    pub from: WorldlineId,
    pub to: WorldlineId,
    pub asset: AssetId,
    /// Current liquidity in minor units
    pub liquidity_minor: i64,
    /// Maximum capacity in minor units
    pub capacity_minor: i64,
}

/// Liquidity field — result of the ERX operator.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiquidityField {
    /// Per-channel liquidity scores (0.0 = dry, 1.0 = fully liquid)
    pub channel_scores: Vec<ChannelLiquidity>,
    /// Overall network liquidity (0.0 = stressed, 1.0 = healthy)
    pub network_score: f64,
    /// Whether circuit breaker conditions are met
    pub circuit_breaker_triggered: bool,
    /// Stress indicator
    pub stress_level: StressLevel,
}

/// Per-channel liquidity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelLiquidity {
    pub from: WorldlineId,
    pub to: WorldlineId,
    pub asset: AssetId,
    /// Liquidity ratio (current / capacity)
    pub ratio: f64,
}

/// Stress level of the settlement network.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StressLevel {
    Normal,
    Elevated,
    High,
    Critical,
}

/// Collateral record for a worldline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollateralRecord {
    pub worldline: WorldlineId,
    pub asset: AssetId,
    /// Available collateral in minor units
    pub available_minor: i64,
    /// Locked (pledged) collateral in minor units
    pub locked_minor: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_mwl_types::IdentityMaterial;

    fn test_wid() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    #[test]
    fn asset_id_display() {
        let asset = AssetId::new("USD");
        assert_eq!(format!("{}", asset), "USD");
    }

    #[test]
    fn settlement_type_serialization() {
        let types = vec![
            SettlementType::DvP,
            SettlementType::PvP,
            SettlementType::FreeOfPayment,
        ];
        for t in &types {
            let json = serde_json::to_string(t).unwrap();
            let restored: SettlementType = serde_json::from_str(&json).unwrap();
            assert_eq!(*t, restored);
        }
    }

    #[test]
    fn financial_commitment_serialization() {
        let fc = FinancialCommitment {
            commitment_id: maple_mwl_types::CommitmentId::new(),
            asset: AssetId::new("USD"),
            amount_minor: 100_000, // $1,000.00
            settlement_type: SettlementType::DvP,
            counterparty: WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32])),
            declaring_identity: test_wid(),
            created_at: TemporalAnchor::now(0),
        };
        let json = serde_json::to_string(&fc).unwrap();
        let restored: FinancialCommitment = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.amount_minor, 100_000);
        assert_eq!(restored.asset, AssetId::new("USD"));
    }

    #[test]
    fn projected_balance_serialization() {
        let pb = ProjectedBalance {
            worldline: test_wid(),
            asset: AssetId::new("BTC"),
            balance_minor: 50_000_000, // 0.5 BTC in satoshis
            trajectory_length: 10,
            projected_at: TemporalAnchor::now(0),
            trajectory_hash: "abc123".into(),
        };
        let json = serde_json::to_string(&pb).unwrap();
        let restored: ProjectedBalance = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.balance_minor, 50_000_000);
    }

    #[test]
    fn stress_level_ordering() {
        assert!(StressLevel::Normal < StressLevel::Elevated);
        assert!(StressLevel::Elevated < StressLevel::High);
        assert!(StressLevel::High < StressLevel::Critical);
    }

    #[test]
    fn settlement_network_serialization() {
        let wid1 = test_wid();
        let wid2 = WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]));
        let network = SettlementNetwork {
            participants: vec![wid1.clone(), wid2.clone()],
            channels: vec![SettlementChannel {
                from: wid1,
                to: wid2,
                asset: AssetId::new("USD"),
                liquidity_minor: 500_000,
                capacity_minor: 1_000_000,
            }],
        };
        let json = serde_json::to_string(&network).unwrap();
        let restored: SettlementNetwork = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.participants.len(), 2);
        assert_eq!(restored.channels.len(), 1);
    }
}
