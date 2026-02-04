# Tutorial 02: Hybrid Approval Workflow

## Objective

Trigger hybrid mode and complete approval/rejection with explicit outcomes.

## Step 1: Submit a High-Value Transfer

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

## Step 2: List Pending Queue

```bash
curl -s http://127.0.0.1:8091/v1/approvals/pending
```

Capture the `trace_id` from the response.

## Step 3A: Approve

```bash
curl -s -X POST http://127.0.0.1:8091/v1/approvals/<trace_id>/approve \
  -H 'content-type: application/json' \
  -d '{"approver_id":"risk-officer-1","note":"manual review passed"}'
```

Expected status: `executed_hybrid`.

## Step 3B: Reject (Alternative Path)

```bash
curl -s -X POST http://127.0.0.1:8091/v1/approvals/<trace_id>/reject \
  -H 'content-type: application/json' \
  -d '{"approver_id":"risk-officer-1","note":"counterparty mismatch"}'
```

Expected payload:

```json
{"trace_id":"...","status":"rejected"}
```

The rejection is explicit and persisted as an outcome record.

## Step 4: Confirm Queue Persistence

Queue file path:

- `ibank/data/approvals.json` (or your custom `--approval-queue` path)

Restarting service keeps pending items intact.

## Step 5: Confirm Core Ledger Persistence (PostgreSQL Mode)

If running with `--ledger-storage postgres`, inspect persisted commitment/audit/outcome rows:

```bash
psql postgres://postgres:postgres@127.0.0.1:5432/ibank -c \"SELECT ledger_index, kind, trace_id, commitment_id FROM ibank_ledger_entries ORDER BY ledger_index;\"
```
