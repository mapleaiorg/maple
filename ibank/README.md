# iBank on MAPLE

iBank is a commitment-first banking engine built on MAPLE Resonance Runtime.

It enforces accountable consequential actions with deterministic policy routing:

- Pure AI mode for low-risk autonomous execution.
- Hybrid mode for high-risk/ambiguous/dispute/compliance-sensitive execution.

## What Is Implemented

- Single entrypoint: `IBankEngine::handle(request) -> HandleResponse`.
- Hybrid workflow engine:
  - `EscalationCase` created for high-risk/ambiguous flows
  - `HumanAttestation` required to resume (approve/deny/modify)
  - workflow states: `open -> in_review -> approved|denied -> executed|closed`
- Agentic Commerce Agent lifecycle:
  - `Discover -> Quote -> Commit -> Pay -> Track -> After-sales/Dispute`
  - discovery outputs plan-only (no payment side effects)
  - payment initiation enforces commitment-before-side-effect via iBank handle flow
  - tracking updates require temporal anchors and are mirrored into audit trail
  - disputes escalate to hybrid by default
- Orchestrator pipeline:
  - meaning formation with explicit ambiguity
  - intent stabilization with confidence profile
  - explicit compliance gate (KYC/AML/sanctions/fraud/jurisdiction)
  - deterministic risk scoring
  - Pure AI vs Hybrid route decision
- Commitment-before-consequence enforcement.
- ComplianceProof generation persisted in commitment platform data:
  - `policy_version`
  - `decision` (`green` / `review_required` / `block`)
  - `reason_codes`
  - redacted `evidence_hashes`
- Accountable wire protocol with origin proof + audit witness + commitment reference.
- Aggregation layer with deterministic connectors producing a `UnifiedLedgerView`:
  - normalized balances and transaction timeline
  - best-route rail candidates from quotes/caps
  - compliance/KYC status normalization
  - verifiable snapshot hash + connector provenance
- Blockchain Bridge execution engine:
  - on-chain, off-chain, and hybrid multi-leg routing
  - commitment-authorized state machine (`Proposed -> Authorized -> Executing -> Settled|Failed -> Recorded`)
  - compensating-action recovery plan on multi-leg failures
  - unified bridge receipt with all leg references + snapshot hash
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
- `POST /v1/bridge/execute`
- `GET /v1/bridge/receipts`
- `GET /v1/compliance/trace/{trace_id}`
- `GET /v1/ledger/entries`
- `GET /v1/ledger/snapshot/latest`
- `GET /v1/approvals/pending`
- `GET /v1/approvals/case/{trace_id}`
- `POST /v1/approvals/{trace_id}/approve`
- `POST /v1/approvals/{trace_id}/reject`

Snapshot endpoint examples:

```bash
# Return latest cached unified snapshot for a user
curl -s "http://127.0.0.1:8091/v1/ledger/snapshot/latest?user_id=issuer-a"

# Force a live refresh (useful for dashboard polling / ops checks)
curl -s "http://127.0.0.1:8091/v1/ledger/snapshot/latest?user_id=issuer-a&refresh=true&base=USD&quote=USD&amount_minor=50000&window_days=30"
```

Bridge execution example:

```bash
curl -s http://127.0.0.1:8091/v1/bridge/execute \
  -H 'content-type: application/json' \
  -d '{
    "trace_id": "trace-bridge-1",
    "execution_id": "exec-bridge-1",
    "commitment_id": "<commitment_id>",
    "origin_actor": "issuer-a",
    "counterparty_actor": "merchant-b",
    "legs": [
      {
        "type": "chain",
        "leg_id": "leg-chain-1",
        "adapter_id": "evm-mock",
        "network": "base-sepolia",
        "asset": "USDC",
        "asset_kind": "stablecoin",
        "from_address": "0xaaa",
        "to_address": "0xbbb",
        "amount_minor": 25000,
        "memo": "fiat->stablecoin"
      },
      {
        "type": "rail",
        "leg_id": "leg-rail-1",
        "adapter_id": "rail-mock",
        "rail": "ach",
        "currency": "USD",
        "from_account": "acct-a",
        "to_account": "acct-b",
        "amount_minor": 25000,
        "memo": "stablecoin->bank rail"
      }
    ]
  }'
```

Bridge receipt query:

```bash
curl -s "http://127.0.0.1:8091/v1/bridge/receipts?trace_id=trace-bridge-1&status=settled"
```

Compliance trace query:

```bash
curl -s "http://127.0.0.1:8091/v1/compliance/trace/<trace_id>"
```

Signed attestation examples:

```bash
# Approve
curl -s -X POST "http://127.0.0.1:8091/v1/approvals/<trace_id>/approve" \
  -H 'content-type: application/json' \
  -d '{"approver_id":"risk-officer","decision":"approve","signature":"sig-1","anchor":"attestation://ops/1","note":"approved"}'

# Modify with constraints
curl -s -X POST "http://127.0.0.1:8091/v1/approvals/<trace_id>/approve" \
  -H 'content-type: application/json' \
  -d '{"approver_id":"risk-officer","decision":"modify","signature":"sig-2","anchor":"attestation://ops/2","constraints":[{"key":"max_amount_minor","value":"500000"},{"key":"require_check","value":"manual_kyc_required"}]}'

# Deny
curl -s -X POST "http://127.0.0.1:8091/v1/approvals/<trace_id>/reject" \
  -H 'content-type: application/json' \
  -d '{"approver_id":"risk-officer","decision":"deny","signature":"sig-3","anchor":"attestation://ops/3","note":"declined"}'

# Inspect workflow state + attestation history
curl -s "http://127.0.0.1:8091/v1/approvals/case/<trace_id>"
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
- `ibank/docs/tutorials/05_bridge_execution.md`
