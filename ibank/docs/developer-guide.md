# iBank Developer Guide

This guide helps you integrate with iBank and extend it safely.

## Core API

Primary entrypoint:

```rust
IBankEngine::handle(request: HandleRequest) -> HandleResponse
```

Use this entrypoint for app/API integration. It guarantees the full orchestrator pipeline and invariants.

## Data Types You Will Use

From `ibank-core`:

- `HandleRequest` / `HandleResponse`
- `HandleStatus` / `ExecutionMode`
- `MeaningField`, `IntentRecord`, `RiskReport`
- `CommitmentRecord` and `AccountableWireMessage`
- `ComplianceDecision` and `ComplianceProof`
- `EscalationCase`, `EscalationWorkflowState`, `HumanAttestation`
- `BridgeExecutionRequest`, `BridgeLeg`, and `UnifiedBridgeReceipt`
- `CommerceIntent`, `AgenticCommerceAgent`, `CommerceOrder`, and tracking/dispute result types

## Integrating in Rust

```rust
use ibank_core::{HandleRequest, IBankEngine, IBankEngineConfig, RiskPolicyConfig};

# async fn run() -> anyhow::Result<()> {
let engine = IBankEngine::bootstrap(RiskPolicyConfig::default(), IBankEngineConfig::default()).await?;

let mut req = HandleRequest::new(
    "issuer-a",
    "merchant-b",
    50_000,
    "USD",
    "ach",
    "acct-123",
    "pay invoice 889",
);
req.jurisdiction = "US".to_string();
req.counterparty_risk = 10;
req.anomaly_score = 8;
req.model_uncertainty = 0.08;

let response = engine.handle(req).await;
println!("{:?}", response.status);
# Ok(())
# }
```

Configure durable PostgreSQL ledger storage:

```rust
use ibank_core::{IBankEngineConfig, LedgerStorageConfig};

let mut cfg = IBankEngineConfig::default();
cfg.ledger_storage = LedgerStorageConfig::postgres(
    "postgres://postgres:postgres@127.0.0.1:5432/ibank",
    5,
);
```

## REST Integration

Service routes:

- `POST /v1/handle`
- `POST /v1/bridge/execute`
- `GET /v1/bridge/receipts`
- `GET /v1/compliance/trace/{trace_id}`
- `GET /v1/ledger/entries`
- `GET /v1/ledger/snapshot/latest`
- `GET /v1/approvals/pending`
- `GET /v1/approvals/case/{trace_id}`
- `POST /v1/approvals/{trace_id}/approve`
- `POST /v1/approvals/{trace_id}/reject`

Approval attestation payload supports:

- `decision`: `approve | deny | modify`
- `signature`
- `anchor`
- `constraints` (for modify)

## Executing Bridge Routes

Bridge execution is commitment-authorized and designed for on-chain/off-chain/hybrid paths.

Core API:

```rust
use ibank_core::{BridgeExecutionRequest, BridgeLeg, ChainAssetKind, ChainBridgeLeg, RailBridgeLeg};

let request = BridgeExecutionRequest::new(
    "exec-1",
    "trace-1",
    commitment_id,
    "issuer-a",
    "merchant-b",
    vec![
        BridgeLeg::Chain(ChainBridgeLeg {
            leg_id: "leg-chain-1".to_string(),
            adapter_id: "evm-mock".to_string(),
            network: "base-sepolia".to_string(),
            asset: "USDC".to_string(),
            asset_kind: ChainAssetKind::Stablecoin,
            from_address: "0xaaa".to_string(),
            to_address: "0xbbb".to_string(),
            amount_minor: 25_000,
            memo: Some("fiat->stablecoin".to_string()),
        }),
        BridgeLeg::Rail(RailBridgeLeg {
            leg_id: "leg-rail-1".to_string(),
            adapter_id: "rail-mock".to_string(),
            rail: "ach".to_string(),
            currency: "USD".to_string(),
            from_account: "acct-a".to_string(),
            to_account: "acct-b".to_string(),
            amount_minor: 25_000,
            memo: Some("stablecoin->local rail".to_string()),
        }),
    ],
);

let receipt = engine.execute_bridge_route(request).await?;
```

Register bridge adapters:

```rust
use ibank_adapters::{MockEvmBridgeAdapter, MockRailBridgeAdapter};
use std::sync::Arc;

engine.register_chain_adapter(Arc::new(MockEvmBridgeAdapter)).await?;
engine.register_rail_adapter(Arc::new(MockRailBridgeAdapter)).await?;
```

## gRPC Integration

Proto contract:

- `ibank/crates/ibank-service/proto/ibank/v1/ibank.proto`

Service:

- `ibank.v1.IBankService`

Methods:

- `Health`
- `Handle`
- `ListPending`
- `ApprovePending`
- `RejectPending`

Reflection is enabled via embedded descriptor set.

## Adding a Settlement Connector

Implement `SettlementConnector`:

```rust
use chrono::Utc;
use ibank_core::{IBankError, SettlementConnector};
use ibank_core::types::{AccountableWireMessage, ConnectorReceipt};
use std::collections::BTreeMap;

struct MyConnector;

impl SettlementConnector for MyConnector {
    fn rail(&self) -> &'static str { "my_rail" }

    fn execute(&self, message: &AccountableWireMessage) -> Result<ConnectorReceipt, IBankError> {
        Ok(ConnectorReceipt {
            settlement_id: format!("my-{}", message.message_id),
            rail: self.rail().to_string(),
            settled_at: Utc::now(),
            metadata: BTreeMap::new(),
        })
    }
}
```

Register connector:

```rust
use std::sync::Arc;

# async fn run(engine: ibank_core::IBankEngine) -> Result<(), ibank_core::IBankError> {
engine.register_connector(Arc::new(MyConnector))?;
# Ok(())
# }
```

## Adding an Aggregation Connector

Aggregation connectors power unified balances, transaction history, quotes, limits, and compliance signals.

Required trait methods:

- `capabilities()`
- `fetch_balances()`
- `fetch_transactions()`
- `fetch_limits()`
- `fetch_quotes()`
- `attest_state_snapshot()`

Reference implementations:

- `OpenBankingAggregationConnector` in `ibank/crates/ibank-adapters/src/lib.rs`
- `CryptoWalletAggregationConnector` in `ibank/crates/ibank-adapters/src/lib.rs`

## Adding Bridge Adapters

Bridge adapters are separate from settlement connectors and support leg-level compensation.

- On-chain adapters implement `ChainAdapter`.
- Rail adapters implement `RailAdapter`.

Required adapter behavior:

1. Reject leg execution if commitment reference is missing from accountable wire message.
2. Return deterministic leg receipts (`tx_hash` or rail reference).
3. Implement compensating action callback used on partial-failure recovery.

## Extending Policy

You can adjust deterministic thresholds via `RiskPolicyConfig`.

Compliance-specific thresholds and block rules are configured under:

- `RiskPolicyConfig::compliance` (`CompliancePolicyConfig`)

If adding new policy factors:

1. Add factor to input type (`TransferIntent` / request mapping).
2. Add deterministic scoring logic in `RiskPolicyEngine`.
3. Add explicit reasons in `RiskReport`.
4. Add unit tests for allow/hybrid/deny branches.

## Invariant Mapping to Code

- Stage chain enforcement: `ibank/crates/ibank-core/src/flow.rs`
- Commitment-first declaration: `ibank/crates/ibank-core/src/runtime.rs`
- Accountable verification and routing order: `ibank/crates/ibank-core/src/router.rs`
- Wire signing/verifying: `ibank/crates/ibank-core/src/protocol.rs`
- Append-only hash-chain ledger: `ibank/crates/ibank-core/src/ledger.rs`
- Durable storage backend and startup rehydration: `ibank/crates/ibank-core/src/storage.rs`

## Testing

Run all iBank tests:

```bash
cargo test --manifest-path ibank/Cargo.toml
```

Focus tests include:

- Pure AI $500 transfer executes and records commitment/audit.
- `>$10k` or dispute routes to hybrid and blocks until explicit approval.
- Routing order is accountability -> risk -> route.
