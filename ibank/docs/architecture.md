# iBank Core Engine Architecture

## Goal

iBank runs financial actions on top of MAPLE with strict accountability:

1. `presence -> coupling -> meaning -> intent -> commitment -> consequence`
2. No implicit commitment for any consequential side effect.
3. Accountable wire format for routing and execution.
4. Risk-bounded autonomy with mandatory hybrid escalation on policy triggers.
5. Explicit persisted outcomes (success and failure).
6. Fixed routing order: accountability verification -> risk bounds -> route-with-audit.

## Crate Layout

- `ibank/crates/ibank-core`: policy engine, stage machine, aggregation layer, commitment model, accountable protocol, router, append-only ledger, and storage backends.
- `ibank/crates/ibank-adapters`: settlement connectors (ACH/chain mock plus failure connector).
- `ibank/crates/ibank-service`: REST + gRPC surface, pending approval queue persistence.

## Single Entrypoint

The canonical API is:

```rust
IBankEngine::handle(request: HandleRequest) -> HandleResponse
```

It orchestrates:

1. Parse request into `MeaningField` (ambiguity is preserved explicitly).
2. Stabilize to `IntentRecord` with confidence profile.
3. Evaluate deterministic risk scoring.
4. Decide route:
   - Pure AI if within autonomous bounds and ambiguity does not block.
   - Hybrid if thresholds are exceeded, dispute/fraud/compliance flags trigger, or ambiguity blocks.
5. Build commitment before any side effect.
6. Route only through accountable wire verification and risk checks.

## Stage Enforcement

`ConsequenceStageMachine` (`ibank/crates/ibank-core/src/flow.rs`) is an explicit finite-state gate.

- Any stage skip returns `IBankError::InvariantViolation`.
- Consequence cannot execute unless commitment stage completed.

This prevents accidental bypasses and keeps auditability deterministic.

## Risk Engine

`RiskPolicyEngine` (`ibank/crates/ibank-core/src/policy.rs`) is rule-based and deterministic.

Inputs:

- amount
- counterparty risk
- jurisdiction
- anomaly/fraud score
- model uncertainty
- ambiguity and compliance flags

Outputs:

- `RiskDecision::Allow`
- `RiskDecision::RequireHybrid`
- `RiskDecision::Deny`

Default core thresholds include:

- Pure AI cap: `$10,000` (`1_000_000` minor units)
- Hard deny cap: `$250,000` (`25_000_000` minor units)
- Fraud and ambiguity/uncertainty thresholds for mandatory hybrid

## Commitment-First Consequence

`IBankEngine::declare_commitment_record` creates a commitment containing:

- scope: action/resources/constraints
- parties: principal + counterparty
- temporal bounds
- reversibility class
- confidence context
- iBank platform data:
  - `transaction_type`
  - `value`
  - `risk_assessment`
  - `regulatory_compliance` (with explicit proof placeholders)
  - `state_snapshot_hash` from the unified aggregation snapshot

The commitment is appended to the hash-chained ledger before routing.

Before commitment declaration, runtime builds a `UnifiedLedgerView` from all registered aggregation connectors and computes a deterministic snapshot hash. After commitment persistence, an audit entry (`state_snapshot_attested`) links commitment id to snapshot hash.

## Accountable Wire Protocol

`AccountableWireMessage` includes:

- origin proof (signed envelope)
- audit witness (ledger anchor)
- optional commitment reference (required for consequential routing)

`IBankRouter::route` verifies in strict order:

1. Commitment reference exists and points to a known commitment.
2. Origin proof verifies.
3. Audit witness resolves and matches ledger hash.
4. Risk bounds pass for the selected execution mode.
5. Connector executes and outcome is recorded.

## Ledger Model

`AppendOnlyLedger` (`ibank/crates/ibank-core/src/ledger.rs`) stores:

- `commitment`
- `audit`
- `outcome`

Every entry has:

- monotonic index
- timestamp
- previous hash
- computed hash

`verify_chain()` detects tampering and broken history.

`PersistentLedger` (`ibank/crates/ibank-core/src/storage.rs`) wraps this chain with durable backends:

- `memory`: process-local chain for local development.
- `postgres`: persisted `ibank_ledger_entries` table for production durability.

Postgres bootstrap behavior:

1. Ensure schema/indexes exist.
2. Load all ledger rows ordered by `ledger_index`.
3. Rebuild chain and verify hash integrity before serving requests.
4. For each new entry, persist first, then commit in-memory for deterministic continuity.

## Service Layer

`ibank-service` wraps core engine for app/API use:

- REST endpoints: `/v1/handle`, `/v1/approvals/pending`, approve/reject routes.
- REST ledger query endpoint: `/v1/ledger/entries` with filter/pagination for audit tooling.
- REST snapshot endpoint: `/v1/ledger/snapshot/latest` for dashboard/ops unified view retrieval.
- gRPC service: `ibank.v1.IBankService` (proto-first contract).
- Persisted approval queue: `ibank/data/approvals.json`.
- Configurable ledger durability:
  - `--ledger-storage memory|postgres|auto`
  - `--ledger-database-url postgres://...`
  - `--ledger-pg-max-connections <n>`

Approval path:

1. Hybrid-required request is queued.
2. Human approves or rejects explicitly.
3. Re-run with approval metadata for execution, or persist rejection outcome.

## Reliability Characteristics

- Deterministic policy evaluation (same input => same decision).
- Explicit failure records (no silent drops).
- Commitment and audit traceability across all consequential paths.
- Clear extension seams for connectors and storage.
