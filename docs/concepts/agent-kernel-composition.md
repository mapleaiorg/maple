# Agent Kernel Composition (Non-bypassable)

This document defines the runtime primitive now implemented in `maple-runtime`:

`Agent = Resonator + Profile + CapabilitySet + ContractSet + State`

## What is enforced

The `AgentKernel` in `crates/maple-runtime/src/agent_kernel/mod.rs` enforces:

1. `presence -> meaning -> intent -> commitment -> consequence`
2. No consequential capability executes without an explicit commitment.
3. Every capability path (safe and dangerous) passes through `CommitmentGateway`.
4. Every execution emits durable receipts to AAS ledger (replayable by commitment id).
5. Every execution path passes AAS capability + policy + ledger checks.
6. No silent drops: explicit failures are written to outcome + audit + failed receipt.
7. Explicit failures are persisted in append-only `AgentAuditEvent` records.
8. Model backends remain cognition-only (they cannot trigger execution directly).

## Main runtime pieces

- `AgentKernel`: orchestrates cognition + gating + capability execution.
- `CommitmentGateway`: validates RCF commitment, submits to AAS, records outcome.
- `MapleStorage` integration: shared backing store for AAS commitments, audit chain, and checkpoints.
- `CapabilityExecutor`: pluggable tool adapter interface.
- `ModelAdapter`: pluggable cognition backend interface.

## Llama-first + vendor adapters

Implemented in `crates/maple-runtime/src/cognition/mod.rs`:

- `LlamaAdapter` (default) with strict schema parsing.
- Repair passes for malformed JSON output.
- Deterministic fallback that never suggests executable tools.
- Provider-specific adapters for OpenAI / Anthropic / Gemini / Grok using the same parser and guard behavior.
- Normalized contracts: `ModelProviderConfig`, `ModelUsage`, and `ModelAdapterError`.

## Consequential execution rule

For consequential capability calls (example: `simulate_transfer`):

- If no explicit commitment is provided, runtime returns `MissingCommitment`.
- If commitment exists but AAS decision is not executable, runtime returns `ApprovalRequired`.
- If approved, execution starts and outcome is written to AAS ledger and MAPLE storage.
- Connector/executor boundaries also require explicit commitment references (`no commitment, no consequence`).

## Uniform Gateway Rule (all capabilities)

All capability execution now uses `CommitmentGateway`:

- Consequential capability:
  - requires explicit commitment from caller
  - missing commitment returns `ContractMissing`
- Non-consequential capability:
  - runtime auto-creates a capability-scoped commitment
  - still passes policy + capability checks
  - still records lifecycle, receipt hash, and audit entries

This removes the last direct executor bypass path and gives one durable source of truth for receipts.

## Receipt Persistence (AAS Ledger)

Receipt records are append-only and replayable:

- `receipt_id`
- `tool_call_id`
- `contract_id`
- `capability_id`
- `hash`
- `timestamp`
- `status` (`Succeeded` or `Failed`)

Primary APIs:

- `AasService::record_tool_receipt(...)`
- `AasService::get_tool_receipts(...)`
- `AccountabilityLedger::record_tool_receipt(...)`
- `AccountabilityLedger::get_tool_receipts_by_commitment(...)`

## API and CLI Observation Surfaces

Daemon endpoints:

- `GET /api/v1/agent/status`
- `POST /api/v1/agent/handle`
- `GET /api/v1/agent/audit`
- `GET /api/v1/agent/commitments`
- `GET /api/v1/agent/commitments/:id`

CLI:

- `maple agent status`
- `maple agent handle ...`
- `maple agent audit --limit N`
- `maple agent commitments --limit N`
- `maple agent commitment --id <commitment_id>`

## Persistence behavior

`AgentKernel` now persists:

- `AgentAuditEvent` into `AuditStore` (append-only, hash-linked records).
- commitment decisions + lifecycle transitions via **AAS ledger**, which writes into `CommitmentStore`.
- runtime host state into `AgentStateStore` checkpoints.

Default runtime uses in-memory storage; production should provide PostgreSQL-backed storage.

## Demo

Run:

```bash
cargo run -p maple-runtime --example 06_agent_kernel_boundary
```

The demo shows:

- Safe capability succeeds with runtime-generated commitment (still gateway-routed).
- Dangerous capability denied without commitment.
- Dangerous capability succeeds with commitment and emits receipt + audit logs.

Ops/platform boundary demo:

```bash
cargo run -p palm-boundary-demo --offline
```

This includes a dedicated execution-boundary scenario showing:

- rejection when execution parameters omit commitment reference
- success when explicit commitment reference is present

## Tests

`cargo test -p maple-runtime --offline` includes:

- Dangerous action denied without commitment.
- Dangerous action allowed with commitment and ledger outcome.
- Safe action routed through gateway with auto commitment + durable receipt.
- Consistent no-bypass behavior across all model adapters.
