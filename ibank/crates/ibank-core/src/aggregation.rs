use crate::error::IBankError;
use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Connector capability metadata used by routing and compatibility checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorCaps {
    pub rails: Vec<String>,
    pub regions: Vec<String>,
    pub assets: Vec<String>,
}

impl ConnectorCaps {
    pub fn normalized(mut self) -> Self {
        self.rails.sort();
        self.rails.dedup();
        self.regions.sort();
        self.regions.dedup();
        self.assets.sort();
        self.assets.dedup();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AggregationUser {
    pub user_id: String,
}

impl AggregationUser {
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    pub fn last_days(days: i64) -> Self {
        let end = Utc::now();
        let start = end - Duration::days(days.max(1));
        Self { start, end }
    }

    pub fn contains(&self, at: DateTime<Utc>) -> bool {
        at >= self.start && at <= self.end
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetPair {
    pub base: String,
    pub quote: String,
}

impl AssetPair {
    pub fn new(base: impl Into<String>, quote: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            quote: quote.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BalanceRecord {
    pub account_id: String,
    pub asset: String,
    pub available_minor: i64,
    pub total_minor: i64,
}

pub type Balances = Vec<BalanceRecord>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TxDirection {
    Credit,
    Debit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransactionRecord {
    pub tx_id: String,
    pub account_id: String,
    pub asset: String,
    pub amount_minor: i64,
    pub direction: TxDirection,
    pub status: String,
    pub rail: Option<String>,
    pub counterparty: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

pub type Txns = Vec<TransactionRecord>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComplianceSignal {
    pub kyc_state: String,
    pub aml_state: String,
    pub sanctions_clear: bool,
    pub risk_level: String,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Limits {
    pub daily_cap_minor: u64,
    pub rail_caps_minor: BTreeMap<String, u64>,
    pub compliance: ComplianceSignal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuoteRecord {
    pub rail: String,
    pub pair: AssetPair,
    pub rate: f64,
    pub fee_minor: u64,
    pub slippage_bps: u32,
    pub expires_at: DateTime<Utc>,
}

pub type Quotes = Vec<QuoteRecord>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotProof {
    pub connector_id: String,
    pub snapshot_hash: String,
    pub attested_at: DateTime<Utc>,
}

#[async_trait]
pub trait AggregationConnector: Send + Sync {
    fn connector_id(&self) -> &'static str;
    fn capabilities(&self) -> ConnectorCaps;
    async fn fetch_balances(&self, user: &AggregationUser) -> Result<Balances, IBankError>;
    async fn fetch_transactions(
        &self,
        user: &AggregationUser,
        range: &TimeRange,
    ) -> Result<Txns, IBankError>;
    async fn fetch_limits(&self, user: &AggregationUser) -> Result<Limits, IBankError>;
    async fn fetch_quotes(&self, pair: &AssetPair, amount_minor: u64)
        -> Result<Quotes, IBankError>;
    async fn attest_state_snapshot(&self) -> Result<SnapshotProof, IBankError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldProvenance {
    pub connector_id: String,
    pub snapshot_hash: String,
    pub attested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedBalance {
    pub account_ref: String,
    pub asset: String,
    pub available_minor: i64,
    pub total_minor: i64,
    pub field_provenance: BTreeMap<String, FieldProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedTransaction {
    pub tx_ref: String,
    pub account_ref: String,
    pub asset: String,
    pub amount_minor: i64,
    pub direction: TxDirection,
    pub status: String,
    pub rail: Option<String>,
    pub counterparty: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub field_provenance: BTreeMap<String, FieldProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedComplianceStatus {
    pub connector_id: String,
    pub kyc_state: String,
    pub aml_state: String,
    pub sanctions_clear: bool,
    pub risk_level: String,
    pub observed_at: DateTime<Utc>,
    pub daily_cap_minor: u64,
    pub rail_caps_minor: BTreeMap<String, u64>,
    pub field_provenance: BTreeMap<String, FieldProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteCandidate {
    pub rail: String,
    pub pair: AssetPair,
    pub estimated_rate: f64,
    pub fee_minor: u64,
    pub slippage_bps: u32,
    pub estimated_total_cost_minor: u64,
    pub score: f64,
    pub path: Vec<String>,
    pub field_provenance: BTreeMap<String, FieldProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedLedgerView {
    pub user_id: String,
    pub generated_at: DateTime<Utc>,
    pub normalized_balances: Vec<NormalizedBalance>,
    pub normalized_txn_timeline: Vec<NormalizedTransaction>,
    pub best_route_candidates: Vec<RouteCandidate>,
    pub compliance_signals: Vec<NormalizedComplianceStatus>,
    pub connector_caps: BTreeMap<String, ConnectorCaps>,
    pub connector_attestations: Vec<SnapshotProof>,
    pub snapshot_hash: String,
}

pub struct UnifiedLedgerAssembler;

impl UnifiedLedgerAssembler {
    pub async fn build(
        connectors: &[Arc<dyn AggregationConnector>],
        user: &AggregationUser,
        range: &TimeRange,
        pair: &AssetPair,
        amount_minor: u64,
    ) -> Result<UnifiedLedgerView, IBankError> {
        let mut connector_refs = connectors.to_vec();
        connector_refs.sort_by(|a, b| a.connector_id().cmp(b.connector_id()));

        let mut normalized_balances = Vec::new();
        let mut normalized_txn_timeline = Vec::new();
        let mut compliance_signals = Vec::new();
        let mut best_route_candidates = Vec::new();
        let mut connector_caps = BTreeMap::new();
        let mut connector_attestations = Vec::new();

        for connector in connector_refs {
            let connector_id = connector.connector_id().to_string();
            let caps = connector.capabilities().normalized();
            connector_caps.insert(connector_id.clone(), caps.clone());

            let snapshot = connector.attest_state_snapshot().await?;
            let provenance = FieldProvenance {
                connector_id: snapshot.connector_id.clone(),
                snapshot_hash: snapshot.snapshot_hash.clone(),
                attested_at: snapshot.attested_at,
            };
            connector_attestations.push(snapshot.clone());

            let balances = connector.fetch_balances(user).await?;
            for balance in balances {
                normalized_balances.push(NormalizedBalance {
                    account_ref: format!("{}:{}", connector_id, balance.account_id),
                    asset: balance.asset,
                    available_minor: balance.available_minor,
                    total_minor: balance.total_minor,
                    field_provenance: provenance_map(
                        &["account_ref", "asset", "available_minor", "total_minor"],
                        &provenance,
                    ),
                });
            }

            let transactions = connector.fetch_transactions(user, range).await?;
            for tx in transactions {
                if !range.contains(tx.occurred_at) {
                    continue;
                }

                normalized_txn_timeline.push(NormalizedTransaction {
                    tx_ref: format!("{}:{}", connector_id, tx.tx_id),
                    account_ref: format!("{}:{}", connector_id, tx.account_id),
                    asset: tx.asset,
                    amount_minor: tx.amount_minor,
                    direction: tx.direction,
                    status: tx.status,
                    rail: tx.rail,
                    counterparty: tx.counterparty,
                    occurred_at: tx.occurred_at,
                    field_provenance: provenance_map(
                        &[
                            "tx_ref",
                            "account_ref",
                            "asset",
                            "amount_minor",
                            "direction",
                            "status",
                            "rail",
                            "counterparty",
                            "occurred_at",
                        ],
                        &provenance,
                    ),
                });
            }

            let limits = connector.fetch_limits(user).await?;
            compliance_signals.push(NormalizedComplianceStatus {
                connector_id: connector_id.clone(),
                kyc_state: limits.compliance.kyc_state,
                aml_state: limits.compliance.aml_state,
                sanctions_clear: limits.compliance.sanctions_clear,
                risk_level: limits.compliance.risk_level,
                observed_at: limits.compliance.observed_at,
                daily_cap_minor: limits.daily_cap_minor,
                rail_caps_minor: limits.rail_caps_minor.clone(),
                field_provenance: provenance_map(
                    &[
                        "connector_id",
                        "kyc_state",
                        "aml_state",
                        "sanctions_clear",
                        "risk_level",
                        "observed_at",
                        "daily_cap_minor",
                        "rail_caps_minor",
                    ],
                    &provenance,
                ),
            });

            let quotes = connector.fetch_quotes(pair, amount_minor).await?;
            for quote in quotes {
                if !caps.rails.iter().any(|rail| rail == &quote.rail) {
                    continue;
                }

                if let Some(rail_cap) = limits.rail_caps_minor.get(&quote.rail) {
                    if amount_minor > *rail_cap {
                        continue;
                    }
                }

                let slippage_minor =
                    amount_minor.saturating_mul(quote.slippage_bps as u64) / 10_000;
                let estimated_total_cost_minor = quote.fee_minor.saturating_add(slippage_minor);
                let score = 1.0 / (1.0 + estimated_total_cost_minor as f64);

                best_route_candidates.push(RouteCandidate {
                    rail: quote.rail,
                    pair: quote.pair,
                    estimated_rate: quote.rate,
                    fee_minor: quote.fee_minor,
                    slippage_bps: quote.slippage_bps,
                    estimated_total_cost_minor,
                    score,
                    path: vec![
                        format!("asset:{}", pair.base),
                        format!("rail:{}", caps.regions.join("|")),
                        format!("asset:{}", pair.quote),
                    ],
                    field_provenance: provenance_map(
                        &[
                            "rail",
                            "pair",
                            "estimated_rate",
                            "fee_minor",
                            "slippage_bps",
                            "estimated_total_cost_minor",
                            "score",
                            "path",
                        ],
                        &provenance,
                    ),
                });
            }
        }

        normalized_balances.sort_by(|a, b| {
            (a.asset.as_str(), a.account_ref.as_str())
                .cmp(&(b.asset.as_str(), b.account_ref.as_str()))
        });

        normalized_txn_timeline.sort_by(|a, b| {
            (b.occurred_at, a.tx_ref.as_str()).cmp(&(a.occurred_at, b.tx_ref.as_str()))
        });

        compliance_signals.sort_by(|a, b| a.connector_id.cmp(&b.connector_id));

        best_route_candidates.sort_by(|a, b| {
            (
                a.estimated_total_cost_minor,
                sort_provenance_key(a),
                a.rail.as_str(),
            )
                .cmp(&(
                    b.estimated_total_cost_minor,
                    sort_provenance_key(b),
                    b.rail.as_str(),
                ))
        });

        connector_attestations.sort_by(|a, b| a.connector_id.cmp(&b.connector_id));

        let mut view = UnifiedLedgerView {
            user_id: user.user_id.clone(),
            generated_at: Utc::now(),
            normalized_balances,
            normalized_txn_timeline,
            best_route_candidates,
            compliance_signals,
            connector_caps,
            connector_attestations,
            snapshot_hash: String::new(),
        };

        view.snapshot_hash = snapshot_hash(&view)?;
        Ok(view)
    }
}

fn provenance_map(
    fields: &[&str],
    provenance: &FieldProvenance,
) -> BTreeMap<String, FieldProvenance> {
    fields
        .iter()
        .map(|field| ((*field).to_string(), provenance.clone()))
        .collect::<BTreeMap<_, _>>()
}

fn sort_provenance_key(candidate: &RouteCandidate) -> String {
    candidate
        .field_provenance
        .get("rail")
        .map(|p| format!("{}:{}", p.connector_id, p.snapshot_hash))
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn snapshot_hash(view: &UnifiedLedgerView) -> Result<String, IBankError> {
    let mut canonical = view.clone();
    canonical.generated_at = Utc.timestamp_opt(0, 0).single().ok_or_else(|| {
        IBankError::InvariantViolation("failed to build canonical timestamp".to_string())
    })?;
    canonical.snapshot_hash.clear();

    let bytes = serde_json::to_vec(&canonical)
        .map_err(|e| IBankError::Serialization(format!("unified snapshot encode failed: {e}")))?;

    Ok(blake3::hash(&bytes).to_hex().to_string())
}

pub fn connected_rails(view: &UnifiedLedgerView) -> Vec<String> {
    let mut rails = BTreeSet::new();
    for caps in view.connector_caps.values() {
        for rail in &caps.rails {
            rails.insert(rail.clone());
        }
    }
    rails.into_iter().collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixtureConnector {
        id: &'static str,
        caps: ConnectorCaps,
        balances: Balances,
        txns: Txns,
        limits: Limits,
        quotes: Quotes,
        snapshot: SnapshotProof,
    }

    #[async_trait]
    impl AggregationConnector for FixtureConnector {
        fn connector_id(&self) -> &'static str {
            self.id
        }

        fn capabilities(&self) -> ConnectorCaps {
            self.caps.clone()
        }

        async fn fetch_balances(&self, _user: &AggregationUser) -> Result<Balances, IBankError> {
            Ok(self.balances.clone())
        }

        async fn fetch_transactions(
            &self,
            _user: &AggregationUser,
            _range: &TimeRange,
        ) -> Result<Txns, IBankError> {
            Ok(self.txns.clone())
        }

        async fn fetch_limits(&self, _user: &AggregationUser) -> Result<Limits, IBankError> {
            Ok(self.limits.clone())
        }

        async fn fetch_quotes(
            &self,
            _pair: &AssetPair,
            _amount_minor: u64,
        ) -> Result<Quotes, IBankError> {
            Ok(self.quotes.clone())
        }

        async fn attest_state_snapshot(&self) -> Result<SnapshotProof, IBankError> {
            Ok(self.snapshot.clone())
        }
    }

    fn dt(ts: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(ts, 0).single().unwrap()
    }

    fn fixture_connectors() -> Vec<Arc<dyn AggregationConnector>> {
        let bank = FixtureConnector {
            id: "openbank",
            caps: ConnectorCaps {
                rails: vec!["ach".to_string(), "wire".to_string()],
                regions: vec!["US".to_string()],
                assets: vec!["USD".to_string()],
            },
            balances: vec![BalanceRecord {
                account_id: "checking".to_string(),
                asset: "USD".to_string(),
                available_minor: 125_000,
                total_minor: 130_000,
            }],
            txns: vec![TransactionRecord {
                tx_id: "btx-1".to_string(),
                account_id: "checking".to_string(),
                asset: "USD".to_string(),
                amount_minor: -20_000,
                direction: TxDirection::Debit,
                status: "posted".to_string(),
                rail: Some("ach".to_string()),
                counterparty: Some("merchant-a".to_string()),
                occurred_at: dt(1_736_000_000),
            }],
            limits: Limits {
                daily_cap_minor: 2_000_000,
                rail_caps_minor: BTreeMap::from([
                    ("ach".to_string(), 1_000_000),
                    ("wire".to_string(), 2_000_000),
                ]),
                compliance: ComplianceSignal {
                    kyc_state: "verified".to_string(),
                    aml_state: "clear".to_string(),
                    sanctions_clear: true,
                    risk_level: "low".to_string(),
                    observed_at: dt(1_736_100_000),
                },
            },
            quotes: vec![QuoteRecord {
                rail: "ach".to_string(),
                pair: AssetPair::new("USD", "USD"),
                rate: 1.0,
                fee_minor: 35,
                slippage_bps: 3,
                expires_at: dt(1_736_200_000),
            }],
            snapshot: SnapshotProof {
                connector_id: "openbank".to_string(),
                snapshot_hash: "snap-openbank-fixture".to_string(),
                attested_at: dt(1_736_100_100),
            },
        };

        let wallet = FixtureConnector {
            id: "wallet",
            caps: ConnectorCaps {
                rails: vec!["chain".to_string(), "lightning".to_string()],
                regions: vec!["GLOBAL".to_string()],
                assets: vec!["BTC".to_string(), "USD".to_string()],
            },
            balances: vec![BalanceRecord {
                account_id: "hot-wallet".to_string(),
                asset: "BTC".to_string(),
                available_minor: 2_500_000,
                total_minor: 2_500_000,
            }],
            txns: vec![TransactionRecord {
                tx_id: "ctx-1".to_string(),
                account_id: "hot-wallet".to_string(),
                asset: "BTC".to_string(),
                amount_minor: 250_000,
                direction: TxDirection::Credit,
                status: "confirmed".to_string(),
                rail: Some("chain".to_string()),
                counterparty: Some("wallet-peer".to_string()),
                occurred_at: dt(1_736_100_000),
            }],
            limits: Limits {
                daily_cap_minor: 5_000_000,
                rail_caps_minor: BTreeMap::from([
                    ("chain".to_string(), 5_000_000),
                    ("lightning".to_string(), 300_000),
                ]),
                compliance: ComplianceSignal {
                    kyc_state: "tier_2".to_string(),
                    aml_state: "monitoring".to_string(),
                    sanctions_clear: true,
                    risk_level: "medium".to_string(),
                    observed_at: dt(1_736_100_500),
                },
            },
            quotes: vec![QuoteRecord {
                rail: "chain".to_string(),
                pair: AssetPair::new("USD", "USD"),
                rate: 1.0,
                fee_minor: 55,
                slippage_bps: 8,
                expires_at: dt(1_736_200_100),
            }],
            snapshot: SnapshotProof {
                connector_id: "wallet".to_string(),
                snapshot_hash: "snap-wallet-fixture".to_string(),
                attested_at: dt(1_736_100_900),
            },
        };

        vec![Arc::new(bank), Arc::new(wallet)]
    }

    #[tokio::test]
    async fn deterministic_merge_results() {
        let user = AggregationUser::new("user-a");
        let range = TimeRange::new(dt(1_735_900_000), dt(1_736_200_000));
        let pair = AssetPair::new("USD", "USD");
        let connectors = fixture_connectors();

        let view = UnifiedLedgerAssembler::build(&connectors, &user, &range, &pair, 50_000)
            .await
            .unwrap();

        assert_eq!(view.normalized_balances.len(), 2);
        assert_eq!(view.normalized_txn_timeline.len(), 2);
        assert_eq!(view.best_route_candidates.len(), 2);
        assert_eq!(view.best_route_candidates[0].rail, "ach");
        assert_eq!(view.best_route_candidates[1].rail, "chain");
    }

    #[tokio::test]
    async fn provenance_preserved_per_field() {
        let user = AggregationUser::new("user-a");
        let range = TimeRange::new(dt(1_735_900_000), dt(1_736_200_000));
        let pair = AssetPair::new("USD", "USD");
        let connectors = fixture_connectors();

        let view = UnifiedLedgerAssembler::build(&connectors, &user, &range, &pair, 50_000)
            .await
            .unwrap();

        let balance = view
            .normalized_balances
            .iter()
            .find(|row| row.account_ref.starts_with("openbank:"))
            .unwrap();

        let amount_prov = balance.field_provenance.get("available_minor").unwrap();
        assert_eq!(amount_prov.connector_id, "openbank");
        assert_eq!(amount_prov.snapshot_hash, "snap-openbank-fixture");
    }

    #[tokio::test]
    async fn snapshot_hash_is_stable() {
        let user = AggregationUser::new("user-a");
        let range = TimeRange::new(dt(1_735_900_000), dt(1_736_200_000));
        let pair = AssetPair::new("USD", "USD");
        let connectors = fixture_connectors();

        let view_a = UnifiedLedgerAssembler::build(&connectors, &user, &range, &pair, 50_000)
            .await
            .unwrap();
        let view_b = UnifiedLedgerAssembler::build(&connectors, &user, &range, &pair, 50_000)
            .await
            .unwrap();

        assert_eq!(view_a.snapshot_hash, view_b.snapshot_hash);
    }
}
