# maple-storage

Unified storage contracts for MAPLE runtime/stateful services.

## What this crate provides

- `CommitmentStore`: commitment lifecycle persistence.
- `AuditStore`: append-only tamper-evident audit events.
- `AgentStateStore`: agent checkpoint/resume state.
- `ProjectionStore`: dashboard/read-model snapshots.
- `SemanticMemoryStore`: optional AI-friendly retrieval index.
- `MapleStorage`: composed trait bundle.

## Adapters

- `memory::InMemoryMapleStorage`: deterministic test/dev adapter.
- `postgres::PostgresMapleStorage` (feature `postgres`): transactional source-of-truth backend.

## Runtime usage

- `maple-runtime::AgentKernel` uses this store for audit events and checkpoints.
- `aas-ledger` also uses this store for commitment lifecycle persistence, so AAS
  and runtime components share one durable ledger source.

## Feature flags

- `postgres`: enables SQLx PostgreSQL adapter.
- `strict-docs`: warns on missing docs.
