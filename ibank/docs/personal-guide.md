# iBank Personal Guide

This guide is for a single operator or builder running iBank locally with safe defaults.

## What You Get

- One entrypoint: send requests to `handle`.
- Small transfers can run in Pure AI mode automatically.
- Risky/large/ambiguous transfers switch to Hybrid approval mode.
- Every decision and failure is explicitly recorded.

## Quick Mental Model

1. You submit a request.
2. iBank interprets intent and confidence.
3. iBank scores risk.
4. iBank either executes (Pure AI) or queues for approval (Hybrid).
5. Any external action must have a commitment id first.

For cross-rail routes (on-chain/off-chain/hybrid), execute with `POST /v1/bridge/execute` using an existing commitment id.

## Start Service

```bash
cargo run --manifest-path ibank/Cargo.toml -p ibank-service -- --listen 127.0.0.1:8091 --grpc-listen 127.0.0.1:50051
```

Use PostgreSQL-backed core ledger:

```bash
cargo run --manifest-path ibank/Cargo.toml -p ibank-service -- \
  --ledger-storage postgres \
  --ledger-database-url postgres://postgres:postgres@127.0.0.1:5432/ibank
```

## Check Health

```bash
curl -s http://127.0.0.1:8091/v1/health
```

## Execute a Small Transfer (Pure AI)

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

Expected status: `executed_autonomous` with a `commitment_id`.

## Trigger Hybrid Mode (Large Transfer)

```bash
curl -s http://127.0.0.1:8091/v1/handle \
  -H 'content-type: application/json' \
  -d '{
    "origin_actor": "issuer-a",
    "counterparty_actor": "merchant-b",
    "transaction_type": "transfer",
    "amount_minor": 1500000,
    "currency": "USD",
    "rail": "ach",
    "destination": "acct-123",
    "jurisdiction": "US",
    "user_intent": "move treasury funds",
    "ambiguity_hint": 0.1,
    "counterparty_risk": 10,
    "anomaly_score": 10,
    "model_uncertainty": 0.1,
    "compliance_flags": [],
    "metadata": {},
    "approval": null
  }'
```

Expected status: `pending_human_approval`.

## Approve a Pending Item

1. List pending:

```bash
curl -s http://127.0.0.1:8091/v1/approvals/pending
```

2. Approve by trace id:

```bash
curl -s -X POST http://127.0.0.1:8091/v1/approvals/<trace_id>/approve \
  -H 'content-type: application/json' \
  -d '{"approver_id":"ops-user","decision":"approve","signature":"sig-1","anchor":"attestation://ops/1","note":"approved"}'
```

## Reject a Pending Item

```bash
curl -s -X POST http://127.0.0.1:8091/v1/approvals/<trace_id>/reject \
  -H 'content-type: application/json' \
  -d '{"approver_id":"ops-user","decision":"deny","signature":"sig-2","anchor":"attestation://ops/2","note":"not enough evidence"}'
```

This writes an explicit rejection outcome and removes the queue item.

## Where Data Lives

- Pending approval queue: `ibank/data/approvals.json`.
- Core commitment/audit/outcome ledger:
  - memory mode: process-local
  - postgres mode: durable rows in `ibank_ledger_entries`

## Safety Tips

- Keep `approval` null in user requests unless approval is intentional.
- Use unique `origin_actor`/`counterparty_actor` labels for trace clarity.
- Archive API responses for business-level evidence chains.
