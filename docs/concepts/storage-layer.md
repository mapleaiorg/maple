# MAPLE Storage Layer

## Why a dedicated storage layer

MAPLE now has multiple persistence concerns that need different guarantees:

- **Commitment/accountability records**: strict consistency and replay safety.
- **Audit trails**: append-only, tamper-evident chains.
- **Agent checkpoints**: fast resume after restarts/migrations.
- **Read-model snapshots**: dashboard and ops projections.
- **Semantic memory**: AI-friendly retrieval that should never bypass core gates.

To keep runtime invariants intact, MAPLE treats storage as **tiered**:

1. **Source of truth**: transactional data (PostgreSQL).
2. **Derived stores**: projections and semantic indexes derived from source-of-truth events.

## Placement decision

Use **`crates/maple/storage`** for runtime storage contracts and adapters.

Reasoning:

- It is application/runtime code and belongs in the Rust workspace.
- It keeps storage APIs versioned, tested, and reusable across `maple-runtime`, `palm-daemon`, and future services.
- It avoids scattering core behavior into ad-hoc root folders.

Use a root-level **`storage/`** directory only for operational assets:

- SQL migrations
- seed fixtures
- local/docker scripts
- backend-specific deployment manifests

So the model is:

- `crates/maple/storage`: code + traits + adapters
- `storage/`: ops artifacts (non-library)

## Core interfaces

`maple-storage` defines these trait surfaces:

- `CommitmentStore`
- `AuditStore`
- `AgentStateStore`
- `ProjectionStore`
- `SemanticMemoryStore`
- `MapleStorage` (composed interface)

These enforce that AI-facing memory is additive, while commitment/audit remains explicit.

## Current adapter status

`InMemoryMapleStorage` is included as a deterministic reference adapter for tests/dev.
`PostgresMapleStorage` is available behind the `maple-storage/postgres` feature.

Production adapter path:

1. Harden PostgreSQL migrations and wire it as default in daemon/runtime environments.
2. Optional pgvector (or external vector DB) backend behind `SemanticMemoryStore`.
3. Optional blob backend for large artifacts in later phases.
