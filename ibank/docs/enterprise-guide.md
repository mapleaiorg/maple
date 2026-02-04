# iBank Enterprise Guide

This guide is for teams deploying iBank with governance, controls, and approval operations.

## Control Objectives

- Prevent unapproved high-risk execution.
- Ensure every side effect is commitment-linked.
- Keep a verifiable audit trail for compliance and incident response.
- Make failures explicit and reviewable.

## Deployment Topology

Recommended baseline:

1. Run `ibank-service` behind API gateway.
2. Keep REST and gRPC enabled for heterogeneous clients.
3. Mount durable storage for approval queue file.
4. Centralize logs with trace id indexing.

Example startup:

```bash
cargo run --manifest-path ibank/Cargo.toml -p ibank-service -- \
  --listen 0.0.0.0:8091 \
  --grpc-listen 0.0.0.0:50051 \
  --approval-queue /var/lib/ibank/approvals.json
```

## Governance Model

Suggested role split:

- `requester`: submits transfer/dispute intents.
- `approver`: approves/rejects hybrid requests.
- `operator`: monitors service health and queue backlog.
- `auditor`: validates commitment/outcome trace and policy behavior.

## Policy and Risk Tuning

Core deterministic policy lives in `RiskPolicyConfig`.

Key levers:

- `pure_ai_max_amount_minor`
- `hard_limit_amount_minor`
- `ambiguity_hybrid_threshold`
- `uncertainty_hybrid_threshold`
- `fraud_hybrid_threshold`
- `hybrid_score_threshold`

Change policy values only through versioned releases and change approval.

## Hybrid Approval Operations

Operational workflow:

1. Request arrives.
2. If hybrid is required, item is queued with trace id and decision rationale.
3. Approver approves or rejects with explicit identity and note.
4. Outcome is persisted and queue updated atomically.

Queue endpoint checks:

- `/v1/approvals/pending`: backlog and SLA monitoring.
- `/v1/approvals/{trace_id}/approve`: controlled release to execution.
- `/v1/approvals/{trace_id}/reject`: explicit risk refusal trail.

## Incident and Failure Handling

iBank records explicit failed outcomes for:

- accountability verification failure
- risk denial
- hybrid-required without approval
- connector execution failure

This enables deterministic postmortems without silent-loss ambiguity.

## Audit and Evidence Strategy

Evidence fields to index in SIEM/log store:

- `trace_id`
- `commitment_id`
- `decision_reason`
- `risk_report.score`
- `status`

Add external retention for:

- API request/response snapshots
- approval decisions (approver id, note, timestamp)
- connector settlement references

## Current Data Durability Scope

- Approval queue is file-persisted and restart-safe.
- Core append-only ledger supports durable PostgreSQL persistence and startup rehydration.
- PostgreSQL schema/indexes are auto-created by service bootstrap.

Recommended production posture:

- Run with `--ledger-storage postgres`.
- Set `--ledger-database-url` (or `DATABASE_URL`) to managed PostgreSQL.
- Keep startup hash-chain verification enabled (default behavior in bootstrap).
- Add read-only ledger export API for internal audit ingestion.

## Hardening Checklist

- Run behind mTLS/API auth gateway.
- Restrict approval endpoints by role.
- Rotate origin signing keys regularly.
- Enforce immutable deployment artifacts for policy changes.
- Run periodic replay tests to validate deterministic policy outcomes.
