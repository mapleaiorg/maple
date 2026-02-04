# iBank on MAPLE

iBank is a commitment-first banking engine built on MAPLE Resonance Runtime.

It enforces accountable consequential actions with deterministic policy routing:

- Pure AI mode for low-risk autonomous execution.
- Hybrid mode for high-risk/ambiguous/dispute/compliance-sensitive execution.

## What Is Implemented

- Single entrypoint: `IBankEngine::handle(request) -> HandleResponse`.
- Orchestrator pipeline:
  - meaning formation with explicit ambiguity
  - intent stabilization with confidence profile
  - deterministic risk scoring
  - Pure AI vs Hybrid route decision
- Commitment-before-consequence enforcement.
- Accountable wire protocol with origin proof + audit witness + commitment reference.
- Aggregation layer with deterministic connectors producing a `UnifiedLedgerView`:
  - normalized balances and transaction timeline
  - best-route rail candidates from quotes/caps
  - compliance/KYC status normalization
  - verifiable snapshot hash + connector provenance
- Hash-chained append-only ledger records (commitment/audit/outcome).
- REST + gRPC service surfaces with shared behavior.
- Persisted hybrid approval queue with approve/reject endpoints.

## Non-Negotiable Invariants

1. `presence -> coupling -> meaning -> intent -> commitment -> consequence` stage ordering.
2. No implicit commitment for consequential side effects.
3. Accountable wire payloads include origin proof and audit witness.
4. Risk-bounded autonomy with mandatory hybrid escalation on triggers.
5. Explicit failure outcomes, never silent drops.
6. Routing order fixed: accountability verification -> risk bounds -> route-with-audit.

## Workspace Layout

- `ibank/crates/ibank-core`: core orchestration and invariant logic.
- `ibank/crates/ibank-adapters`: pluggable settlement connectors.
- `ibank/crates/ibank-service`: REST/gRPC service and persisted approval queue.

## Quickstart

Run tests:

```bash
cargo test --manifest-path ibank/Cargo.toml
```

Run service:

```bash
cargo run --manifest-path ibank/Cargo.toml -p ibank-service -- --listen 127.0.0.1:8091 --grpc-listen 127.0.0.1:50051
```

Run service with PostgreSQL-backed core ledger persistence:

```bash
cargo run --manifest-path ibank/Cargo.toml -p ibank-service -- \
  --ledger-storage postgres \
  --ledger-database-url postgres://postgres:postgres@127.0.0.1:5432/ibank \
  --ledger-pg-max-connections 5
```

Local PostgreSQL example (if you need one quickly):

```bash
docker run --name ibank-pg -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=ibank -p 5432:5432 -d postgres:16
```

Auto mode (default) chooses postgres whenever `--ledger-database-url` or `DATABASE_URL` is present.

REST health check:

```bash
curl -s http://127.0.0.1:8091/v1/health
```

## API Surface

### REST

- `POST /v1/handle`
- `GET /v1/ledger/entries`
- `GET /v1/ledger/snapshot/latest`
- `GET /v1/approvals/pending`
- `POST /v1/approvals/{trace_id}/approve`
- `POST /v1/approvals/{trace_id}/reject`

Snapshot endpoint examples:

```bash
# Return latest cached unified snapshot for a user
curl -s "http://127.0.0.1:8091/v1/ledger/snapshot/latest?user_id=issuer-a"

# Force a live refresh (useful for dashboard polling / ops checks)
curl -s "http://127.0.0.1:8091/v1/ledger/snapshot/latest?user_id=issuer-a&refresh=true&base=USD&quote=USD&amount_minor=50000&window_days=30"
```

### gRPC

Service name: `ibank.v1.IBankService`

- `Health`
- `Handle`
- `ListPending`
- `ApprovePending`
- `RejectPending`

Proto and generated artifacts:

- Proto: `ibank/crates/ibank-service/proto/ibank/v1/ibank.proto`
- Generated stubs: `ibank/crates/ibank-service/src/generated/ibank.v1.rs`
- Descriptor set: `ibank/crates/ibank-service/src/generated/ibank_descriptor.bin`

Regenerate descriptor after proto changes:

```bash
ibank/crates/ibank-service/scripts/regenerate_descriptor.sh
```

## Data Durability (Current Phase)

- Pending approvals are persisted to `ibank/data/approvals.json`.
- Core append-only ledger supports:
  - memory mode (development)
  - postgres mode (production hardening)
- In postgres mode, commitment/audit/outcome rows are persisted and re-hydrated on restart.
- Service bootstrapping auto-creates the `ibank_ledger_entries` table and indexes if missing.

## Documentation

- `ibank/docs/README.md`
- `ibank/docs/architecture.md`
- `ibank/docs/personal-guide.md`
- `ibank/docs/enterprise-guide.md`
- `ibank/docs/developer-guide.md`
- `ibank/docs/tutorials/01_local_start.md`
- `ibank/docs/tutorials/02_hybrid_approval_flow.md`
- `ibank/docs/tutorials/03_grpc_client.md`
- `ibank/docs/tutorials/04_connector_extension.md`
