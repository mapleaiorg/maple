# Tutorial 05: Commitment-Authorized Bridge Execution

## Objective

Execute a hybrid bridge flow (`chain + rail`) and get one unified receipt.

## Prerequisites

- iBank service running locally.
- A valid `commitment_id` created by `POST /v1/handle`.

## Step 1: Create a Commitment

```bash
curl -s http://127.0.0.1:8091/v1/handle \
  -H 'content-type: application/json' \
  -d '{
    "origin_actor": "issuer-a",
    "counterparty_actor": "merchant-b",
    "transaction_type": "transfer",
    "amount_minor": 50000,
    "currency": "USD",
    "rail": "ach",
    "destination": "acct-123",
    "jurisdiction": "US",
    "user_intent": "seed commitment for bridge execution",
    "ambiguity_hint": 0.1,
    "counterparty_risk": 10,
    "anomaly_score": 8,
    "model_uncertainty": 0.08,
    "compliance_flags": [],
    "metadata": {},
    "approval": null
  }'
```

Save the returned `commitment_id`.

## Step 2: Execute Hybrid Bridge Route

```bash
curl -s http://127.0.0.1:8091/v1/bridge/execute \
  -H 'content-type: application/json' \
  -d '{
    "trace_id": "trace-bridge-1",
    "execution_id": "exec-bridge-1",
    "commitment_id": "<commitment_id>",
    "origin_actor": "issuer-a",
    "counterparty_actor": "merchant-b",
    "legs": [
      {
        "type": "chain",
        "leg_id": "leg-chain-1",
        "adapter_id": "evm-mock",
        "network": "base-sepolia",
        "asset": "USDC",
        "asset_kind": "stablecoin",
        "from_address": "0xaaa",
        "to_address": "0xbbb",
        "amount_minor": 25000,
        "memo": "fiat->stablecoin"
      },
      {
        "type": "rail",
        "leg_id": "leg-rail-1",
        "adapter_id": "rail-mock",
        "rail": "ach",
        "currency": "USD",
        "from_account": "acct-a",
        "to_account": "acct-b",
        "amount_minor": 25000,
        "memo": "stablecoin->local rail"
      }
    ]
  }'
```

## Step 3: Verify Unified Receipt

Check response fields:

- `status` (`settled` or `failed`)
- `route_type` (`on_chain`, `off_chain`, `hybrid`)
- `commitment_id`
- `snapshot_hash`
- `leg_receipts` (all leg references)
- `recovery_plan` (non-empty when failure/compensation occurs)

## Step 4: Verify Audit Trail

```bash
curl -s "http://127.0.0.1:8091/v1/ledger/entries?trace_id=trace-bridge-1&kind=audit&order=asc"
```

Expect staged entries such as:

- `bridge_proposed`
- `bridge_authorized`
- `bridge_leg_prepared`
- `bridge_leg_wire_emitted`
- `bridge_leg_settled` or `bridge_leg_failed`
- `bridge_compensation` (failure path)
- `bridge_recorded`

## Step 5: Query Unified Receipts Directly

```bash
curl -s "http://127.0.0.1:8091/v1/bridge/receipts?trace_id=trace-bridge-1&status=settled"
```
