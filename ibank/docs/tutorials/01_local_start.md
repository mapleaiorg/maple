# Tutorial 01: Local Start and First Transfer

## Objective

Boot iBank locally, call the single handle entrypoint, and observe autonomous execution.

## Prerequisites

- Rust toolchain installed.
- Working directory at repo root.

## Step 1: Start iBank Service

```bash
cargo run --manifest-path ibank/Cargo.toml -p ibank-service -- --listen 127.0.0.1:8091 --grpc-listen 127.0.0.1:50051
```

Optional: run with PostgreSQL-backed ledger:

```bash
cargo run --manifest-path ibank/Cargo.toml -p ibank-service -- \
  --ledger-storage postgres \
  --ledger-database-url postgres://postgres:postgres@127.0.0.1:5432/ibank
```

## Step 2: Verify Service Health

```bash
curl -s http://127.0.0.1:8091/v1/health
```

Expected:

```json
{"status":"ok","service":"ibank-service","ledger_backend":"memory"}
```

## Step 3: Send a Low-Risk Transfer

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
    "user_intent": "pay invoice 889",
    "ambiguity_hint": 0.1,
    "counterparty_risk": 10,
    "anomaly_score": 8,
    "model_uncertainty": 0.08,
    "compliance_flags": [],
    "metadata": {},
    "approval": null
  }'
```

## Step 4: Validate Response

Look for:

- `status: "executed_autonomous"`
- `mode: "pure_ai"`
- non-empty `commitment_id`
- `route` with connector and external reference

This confirms commitment-first autonomous execution completed successfully.

## Step 5: Query Ledger Entries

```bash
curl -s "http://127.0.0.1:8091/v1/ledger/entries?kind=commitment&order=desc&limit=10"
```

Useful query params:

- `trace_id`
- `commitment_id`
- `kind` (`commitment|audit|outcome`)
- `order` (`asc|desc`)
- `offset`
- `limit` (max 1000)

## Step 6: Read Latest Unified Snapshot

```bash
curl -s "http://127.0.0.1:8091/v1/ledger/snapshot/latest?user_id=issuer-a"
```

Force live refresh if cache does not exist:

```bash
curl -s "http://127.0.0.1:8091/v1/ledger/snapshot/latest?user_id=issuer-a&refresh=true&base=USD&quote=USD&amount_minor=50000&window_days=30"
```
