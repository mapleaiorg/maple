//! Connector adapters for iBank.

#![deny(unsafe_code)]

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use ibank_core::aggregation::{
    AggregationConnector, AggregationUser, AssetPair, BalanceRecord, Balances, ComplianceSignal,
    ConnectorCaps, Limits, QuoteRecord, Quotes, SnapshotProof, TimeRange, TransactionRecord,
    TxDirection, Txns,
};
use ibank_core::connectors::SettlementConnector;
use ibank_core::error::IBankError;
use ibank_core::types::{AccountableWireMessage, ConnectorReceipt};
use std::collections::BTreeMap;

/// Mock ACH connector for deterministic local settlement simulation.
#[derive(Debug, Clone, Default)]
pub struct MockAchConnector;

impl SettlementConnector for MockAchConnector {
    fn rail(&self) -> &'static str {
        "ach"
    }

    fn execute(&self, message: &AccountableWireMessage) -> Result<ConnectorReceipt, IBankError> {
        let short_id: String = message.message_id.chars().take(8).collect();
        let mut metadata = BTreeMap::new();
        metadata.insert("trace_id".to_string(), message.trace_id.clone());
        metadata.insert(
            "destination".to_string(),
            message.payload.destination.clone(),
        );

        Ok(ConnectorReceipt {
            settlement_id: format!("ach-{short_id}"),
            rail: self.rail().to_string(),
            settled_at: Utc::now(),
            metadata,
        })
    }
}

/// Mock chain connector for tokenized settlement flows.
#[derive(Debug, Clone, Default)]
pub struct MockChainConnector;

impl SettlementConnector for MockChainConnector {
    fn rail(&self) -> &'static str {
        "chain"
    }

    fn execute(&self, message: &AccountableWireMessage) -> Result<ConnectorReceipt, IBankError> {
        let short_id: String = message.message_id.chars().take(8).collect();
        let mut metadata = BTreeMap::new();
        metadata.insert("trace_id".to_string(), message.trace_id.clone());
        metadata.insert("asset".to_string(), message.payload.currency.clone());

        Ok(ConnectorReceipt {
            settlement_id: format!("chain-{short_id}"),
            rail: self.rail().to_string(),
            settled_at: Utc::now(),
            metadata,
        })
    }
}

/// Deterministic failing connector useful for chaos testing.
#[derive(Debug, Clone)]
pub struct AlwaysFailConnector {
    rail_name: &'static str,
    reason: String,
}

impl AlwaysFailConnector {
    pub fn new(rail_name: &'static str, reason: impl Into<String>) -> Self {
        Self {
            rail_name,
            reason: reason.into(),
        }
    }
}

impl SettlementConnector for AlwaysFailConnector {
    fn rail(&self) -> &'static str {
        self.rail_name
    }

    fn execute(&self, _message: &AccountableWireMessage) -> Result<ConnectorReceipt, IBankError> {
        Err(IBankError::ConnectorFailure {
            connector: self.rail_name.to_string(),
            message: self.reason.clone(),
        })
    }
}

/// Deterministic OpenBanking aggregation connector fixture.
#[derive(Debug, Clone, Default)]
pub struct OpenBankingAggregationConnector;

#[async_trait]
impl AggregationConnector for OpenBankingAggregationConnector {
    fn connector_id(&self) -> &'static str {
        "openbanking"
    }

    fn capabilities(&self) -> ConnectorCaps {
        ConnectorCaps {
            rails: vec!["ach".to_string(), "wire".to_string(), "sepa".to_string()],
            regions: vec!["US".to_string(), "EU".to_string()],
            assets: vec!["USD".to_string(), "EUR".to_string()],
        }
    }

    async fn fetch_balances(&self, _user: &AggregationUser) -> Result<Balances, IBankError> {
        Ok(vec![
            BalanceRecord {
                account_id: "checking-main".to_string(),
                asset: "USD".to_string(),
                available_minor: 420_000,
                total_minor: 440_000,
            },
            BalanceRecord {
                account_id: "savings-reserve".to_string(),
                asset: "USD".to_string(),
                available_minor: 1_200_000,
                total_minor: 1_200_000,
            },
        ])
    }

    async fn fetch_transactions(
        &self,
        _user: &AggregationUser,
        range: &TimeRange,
    ) -> Result<Txns, IBankError> {
        let records = vec![
            TransactionRecord {
                tx_id: "ob-tx-001".to_string(),
                account_id: "checking-main".to_string(),
                asset: "USD".to_string(),
                amount_minor: -65_000,
                direction: TxDirection::Debit,
                status: "posted".to_string(),
                rail: Some("ach".to_string()),
                counterparty: Some("merchant-z".to_string()),
                occurred_at: fixed_time(1_736_100_000),
            },
            TransactionRecord {
                tx_id: "ob-tx-002".to_string(),
                account_id: "checking-main".to_string(),
                asset: "USD".to_string(),
                amount_minor: 150_000,
                direction: TxDirection::Credit,
                status: "posted".to_string(),
                rail: Some("wire".to_string()),
                counterparty: Some("payroll-inc".to_string()),
                occurred_at: fixed_time(1_736_140_000),
            },
        ];

        Ok(records
            .into_iter()
            .filter(|record| range.contains(record.occurred_at))
            .collect::<Vec<_>>())
    }

    async fn fetch_limits(&self, _user: &AggregationUser) -> Result<Limits, IBankError> {
        Ok(Limits {
            daily_cap_minor: 3_000_000,
            rail_caps_minor: BTreeMap::from([
                ("ach".to_string(), 1_000_000),
                ("wire".to_string(), 3_000_000),
                ("sepa".to_string(), 1_500_000),
            ]),
            compliance: ComplianceSignal {
                kyc_state: "verified".to_string(),
                aml_state: "clear".to_string(),
                sanctions_clear: true,
                risk_level: "low".to_string(),
                observed_at: fixed_time(1_736_140_500),
            },
        })
    }

    async fn fetch_quotes(
        &self,
        pair: &AssetPair,
        _amount_minor: u64,
    ) -> Result<Quotes, IBankError> {
        Ok(vec![
            QuoteRecord {
                rail: "ach".to_string(),
                pair: pair.clone(),
                rate: 1.0,
                fee_minor: 30,
                slippage_bps: 2,
                expires_at: fixed_time(1_736_200_000),
            },
            QuoteRecord {
                rail: "wire".to_string(),
                pair: pair.clone(),
                rate: 1.0,
                fee_minor: 110,
                slippage_bps: 1,
                expires_at: fixed_time(1_736_200_000),
            },
        ])
    }

    async fn attest_state_snapshot(&self) -> Result<SnapshotProof, IBankError> {
        Ok(SnapshotProof {
            connector_id: self.connector_id().to_string(),
            snapshot_hash: "openbanking-fixture-snapshot-v1".to_string(),
            attested_at: fixed_time(1_736_140_900),
        })
    }
}

/// Deterministic crypto wallet aggregation connector fixture.
#[derive(Debug, Clone, Default)]
pub struct CryptoWalletAggregationConnector;

#[async_trait]
impl AggregationConnector for CryptoWalletAggregationConnector {
    fn connector_id(&self) -> &'static str {
        "crypto-wallet"
    }

    fn capabilities(&self) -> ConnectorCaps {
        ConnectorCaps {
            rails: vec!["chain".to_string(), "lightning".to_string()],
            regions: vec!["GLOBAL".to_string()],
            assets: vec!["BTC".to_string(), "USDC".to_string()],
        }
    }

    async fn fetch_balances(&self, _user: &AggregationUser) -> Result<Balances, IBankError> {
        Ok(vec![BalanceRecord {
            account_id: "wallet-hot".to_string(),
            asset: "BTC".to_string(),
            available_minor: 2_750_000,
            total_minor: 2_750_000,
        }])
    }

    async fn fetch_transactions(
        &self,
        _user: &AggregationUser,
        range: &TimeRange,
    ) -> Result<Txns, IBankError> {
        let records = vec![TransactionRecord {
            tx_id: "cw-tx-101".to_string(),
            account_id: "wallet-hot".to_string(),
            asset: "BTC".to_string(),
            amount_minor: -250_000,
            direction: TxDirection::Debit,
            status: "confirmed".to_string(),
            rail: Some("chain".to_string()),
            counterparty: Some("wallet-peer-7".to_string()),
            occurred_at: fixed_time(1_736_120_000),
        }];

        Ok(records
            .into_iter()
            .filter(|record| range.contains(record.occurred_at))
            .collect::<Vec<_>>())
    }

    async fn fetch_limits(&self, _user: &AggregationUser) -> Result<Limits, IBankError> {
        Ok(Limits {
            daily_cap_minor: 7_000_000,
            rail_caps_minor: BTreeMap::from([
                ("chain".to_string(), 7_000_000),
                ("lightning".to_string(), 500_000),
            ]),
            compliance: ComplianceSignal {
                kyc_state: "tier_2".to_string(),
                aml_state: "monitoring".to_string(),
                sanctions_clear: true,
                risk_level: "medium".to_string(),
                observed_at: fixed_time(1_736_141_000),
            },
        })
    }

    async fn fetch_quotes(
        &self,
        pair: &AssetPair,
        _amount_minor: u64,
    ) -> Result<Quotes, IBankError> {
        Ok(vec![QuoteRecord {
            rail: "chain".to_string(),
            pair: pair.clone(),
            rate: 1.0,
            fee_minor: 55,
            slippage_bps: 8,
            expires_at: fixed_time(1_736_200_500),
        }])
    }

    async fn attest_state_snapshot(&self) -> Result<SnapshotProof, IBankError> {
        Ok(SnapshotProof {
            connector_id: self.connector_id().to_string(),
            snapshot_hash: "crypto-wallet-fixture-snapshot-v1".to_string(),
            attested_at: fixed_time(1_736_141_100),
        })
    }
}

fn fixed_time(ts: i64) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(ts, 0)
        .single()
        .expect("fixture timestamp must be valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ibank_core::aggregation::{AggregationUser, AssetPair, TimeRange};
    use ibank_core::types::{AccountableWireMessage, AuditWitness, OriginProof, TransferPayload};

    fn sample_message() -> AccountableWireMessage {
        AccountableWireMessage {
            message_id: "msg-1".to_string(),
            trace_id: "trace-1".to_string(),
            origin_actor: "issuer-a".to_string(),
            payload: TransferPayload {
                from: "issuer-a".to_string(),
                to: "merchant-b".to_string(),
                amount_minor: 10_000,
                currency: "USD".to_string(),
                destination: "acct-1".to_string(),
                purpose: "invoice".to_string(),
            },
            origin_proof: OriginProof {
                key_id: "k".to_string(),
                nonce: "n".to_string(),
                signed_at: Utc::now(),
                signature: "sig".to_string(),
            },
            audit_witness: AuditWitness {
                entry_id: "entry-1".to_string(),
                entry_hash: "hash-1".to_string(),
                observed_at: Utc::now(),
            },
            commitment_ref: None,
        }
    }

    #[test]
    fn ach_adapter_returns_receipt() {
        let connector = MockAchConnector;
        let receipt = connector.execute(&sample_message()).unwrap();
        assert_eq!(receipt.rail, "ach");
    }

    #[test]
    fn failing_adapter_returns_error() {
        let connector = AlwaysFailConnector::new("wire", "forced");
        let err = connector.execute(&sample_message()).unwrap_err();
        assert!(matches!(err, IBankError::ConnectorFailure { .. }));
    }

    #[tokio::test]
    async fn openbanking_aggregation_connector_is_deterministic() {
        let connector = OpenBankingAggregationConnector;
        let user = AggregationUser::new("u-1");
        let range = TimeRange::new(fixed_time(1_736_090_000), fixed_time(1_736_150_000));

        let balances_a = connector.fetch_balances(&user).await.unwrap();
        let balances_b = connector.fetch_balances(&user).await.unwrap();
        assert_eq!(balances_a, balances_b);

        let quotes = connector
            .fetch_quotes(&AssetPair::new("USD", "USD"), 50_000)
            .await
            .unwrap();
        assert!(!quotes.is_empty());

        let txns = connector.fetch_transactions(&user, &range).await.unwrap();
        assert!(!txns.is_empty());
    }

    #[tokio::test]
    async fn crypto_wallet_aggregation_connector_is_deterministic() {
        let connector = CryptoWalletAggregationConnector;
        let user = AggregationUser::new("u-1");
        let range = TimeRange::new(fixed_time(1_736_090_000), fixed_time(1_736_150_000));

        let balances_a = connector.fetch_balances(&user).await.unwrap();
        let balances_b = connector.fetch_balances(&user).await.unwrap();
        assert_eq!(balances_a, balances_b);

        let snapshot_a = connector.attest_state_snapshot().await.unwrap();
        let snapshot_b = connector.attest_state_snapshot().await.unwrap();
        assert_eq!(snapshot_a.snapshot_hash, snapshot_b.snapshot_hash);

        let txns = connector.fetch_transactions(&user, &range).await.unwrap();
        assert_eq!(txns.len(), 1);
    }
}
