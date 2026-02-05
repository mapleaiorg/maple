# Agent Kernel Composition (Non-bypassable)

This document defines the runtime primitive now implemented in `maple-runtime`:

`Agent = Resonator + Profile + CapabilitySet + ContractSet + State`

## What is enforced

The `AgentKernel` in `crates/maple-runtime/src/agent_kernel/mod.rs` enforces:

1. `presence -> meaning -> intent -> commitment -> consequence`
2. No consequential capability executes without an explicit commitment.
3. Every capability path passes through AAS capability + policy + ledger checks.
4. Explicit failures are persisted in append-only `AgentAuditEvent` records.
5. Model backends remain cognition-only (they cannot trigger execution directly).

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
- `VendorAdapter` for OpenAI / Anthropic / Gemini / Grok using the same parser and guard behavior.

## Consequential execution rule

For consequential capability calls (example: `simulate_transfer`):

- If no explicit commitment is provided, runtime returns `MissingCommitment`.
- If commitment exists but AAS decision is not executable, runtime returns `ApprovalRequired`.
- If approved, execution starts and outcome is written to AAS ledger and MAPLE storage.

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

- Safe capability succeeds without commitment.
- Dangerous capability denied without commitment.
- Dangerous capability succeeds with commitment and emits receipt + audit logs.

## Tests

`cargo test -p maple-runtime --offline` includes:

- Dangerous action denied without commitment.
- Dangerous action allowed with commitment and ledger outcome.
- Consistent no-bypass behavior across all model adapters.
