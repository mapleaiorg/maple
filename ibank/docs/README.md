# iBank Documentation

This documentation set explains how the iBank Core Engine is designed, how it enforces MAPLE accountability invariants, and how different audiences can adopt it quickly.

## Start Here

- `ibank/README.md`: top-level quickstart and service entrypoints.
- `ibank/docs/architecture.md`: core design, invariants, pipeline stages, and data model.

## Audience Guides

- `ibank/docs/personal-guide.md`: simple app/API usage for individual builders and operators.
- `ibank/docs/enterprise-guide.md`: governance, policy tuning, approvals, operations, and controls.
- `ibank/docs/developer-guide.md`: crate map, extension points, API contracts, and implementation patterns.

## Tutorials

- `ibank/docs/tutorials/01_local_start.md`: run iBank locally and execute your first autonomous transfer.
- `ibank/docs/tutorials/02_hybrid_approval_flow.md`: trigger hybrid mode and approve/reject explicitly.
- `ibank/docs/tutorials/03_grpc_client.md`: use gRPC/`grpcurl` with reflection and typed contracts.
- `ibank/docs/tutorials/04_connector_extension.md`: add a custom settlement connector safely.
- `ibank/docs/tutorials/05_bridge_execution.md`: execute commitment-authorized on-chain/off-chain/hybrid bridge routes.

## What Is Implemented Today

- Single orchestrator entrypoint: `IBankEngine::handle(request) -> HandleResponse`.
- Deterministic risk classification into Pure AI vs Hybrid routing.
- Explicit compliance gate (`Green | ReviewRequired | Block`) with auditable proof output.
- Hybrid attestation workflow with escalation cases and state transitions.
- Case inspection endpoint for full workflow state + attestation history.
- Agentic commerce lifecycle orchestration with commitment-safe discovery/payment/dispute stages.
- Mandatory commitment creation before consequential routing.
- Unified aggregation snapshots available via `/v1/ledger/snapshot/latest`.
- Blockchain bridge execution via `/v1/bridge/execute` with unified receipt output.
- Bridge receipt retrieval via `/v1/bridge/receipts` for dashboard/ops/audit views.
- Compliance trace retrieval via `/v1/compliance/trace/{trace_id}` with proof + audit details.
- Accountable wire verification (origin proof + audit witness + commitment reference).
- Hash-chained append-only ledger entries for commitment, audit, and outcomes.
- Persisted hybrid approval queue in service layer (`ibank/data/approvals.json`).

## Current Boundaries

- The append-only core ledger supports memory and PostgreSQL backends.
- PostgreSQL mode persists commitments/audit/outcomes and rehydrates them on restart.
- Pending human-approval queue is persisted to disk and survives service restarts.
