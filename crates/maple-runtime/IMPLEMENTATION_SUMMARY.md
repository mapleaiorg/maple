# MAPLE Runtime Implementation Summary

## Scope

`maple-runtime` implements the non-bypassable execution kernel for WorldLine-based agents.
The runtime integrates cognition, policy evaluation, capability checks, commitment adjudication,
and consequence execution with durable accountability records.

## Current Architecture

### Runtime kernel

- `AgentKernel` orchestrates one execution loop:
  `meaning -> intent -> commitment authorization -> consequence`.
- `AgentState` composes profile constraints, capability registry, policy engine,
  contract engine, short memory, and journal handles.
- `CommitmentGateway` is the only supported path for capability execution.

### Commitment boundary enforcement

- Consequential capabilities require explicit commitments when profile policy demands it.
- Commitment-to-capability binding is validated before authorization:
  - principal identity match
  - temporal validity
  - effect-domain match
  - scope coverage
  - explicit required capability reference
- Capability execution runs only after:
  - RCF validation
  - AAS adjudication
  - contract activation
  - policy + capability checks

### Accountability and receipts

- Execution writes explicit lifecycle records (`approved -> executing -> fulfilled/failed`).
- Tool execution receipts are persisted with deterministic content hashes.
- Journal entries capture stage transitions and tool call results.
- Audit events are persisted through `maple-storage`.

## WorldLine Namespace Alignment

Canonical crates now provide the architecture-level namespace:

- `worldline-types`
- `worldline-identity`
- `worldline-core`
- `worldline-runtime`
- `worldline-ledger`
- `worldline-governance`

Compatibility wrappers remain for existing integrations:

- `maple-mwl-types` -> `worldline-types`
- `maple-mwl-identity` -> `worldline-identity`
- `maple-mwl-conformance` -> `worldline-conformance`
- `maple-mwl-integration` -> `worldline-integration`

## Commitment Gate Hardening (latest)

The gate lifecycle model now enforces explicit, valid transitions:

- Pending -> Approved | Denied | Expired | Revoked
- Approved -> ExecutionStarted | Expired | Revoked
- Active -> Fulfilled | Failed | Expired | Revoked
- Denied/Fulfilled/Failed/Expired/Revoked are terminal

Additional behavior:

- `record_outcome` rejects outcomes for denied/terminal commitments.
- `record_outcome` auto-records `ExecutionStarted` when invoked from `Approved`.
- `PartiallyFulfilled` outcomes are recorded as explicit failure with
  `partial_completion` metadata (no silent promotion to success).

## Validation Status

Verified with targeted tests:

- `cargo test -p maple-kernel-gate`
- `cargo test -p worldline-integration`
- `cargo test -p worldline-conformance`
- `cargo test -p maple-runtime --test ibank_commitment_boundary`
- `cargo test -p maple-kernel-governance`

Known workspace caveat:

- `cargo fmt --all` currently fails due missing file:
  `crates/palm/state/src/storage/postgres.rs`.
  File-level formatting on modified sources is clean.

## Next Engineering Priorities

1. Move remaining direct consequence entrypoints behind explicit gate receipts.
2. Continue migration from legacy `maple-kernel-*` internals to canonical `worldline-*` implementations.
3. Add end-to-end replay verification tests for operator evolution and upgrade commitments.
