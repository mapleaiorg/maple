# WorldLine Rename and Migration Plan

This plan replaces control-plane naming centered on PALM with WorldLine-governance naming,
while keeping compatibility for one release cycle.

## Migration goals

- Keep kernel invariants unchanged.
- Move terminology to:
  - `worldline-core`
  - `worldline-runtime`
  - `worldline-ledger`
  - `worldline-governance`
  - `worldline-operator-bot`
  - `worldline-promptkit`
- Preserve existing `palm-*` crate/API compatibility during transition.

## Stage 1 - Canonical facades with no breakage

Scope:
- Introduce facade crates (`worldline-core`, `worldline-runtime`, `worldline-ledger`)
  as thin wrappers/re-exports over current runtime and ledger-capable modules.
- Keep all existing `maple-*` and `palm-*` crates fully functional.

Acceptance:
- Existing public APIs continue to compile unchanged.
- New facade crates compile and publish docs.
- `cargo test --workspace` passes.

Prompt skeleton:
1. Add new workspace members for facade crates.
2. Implement re-export modules and crate docs with migration notes.
3. Add `#[deprecated]` guidance only where safe (no hard break).
4. Update docs index with old->new mapping table.
5. Run workspace tests.

## Stage 2 - Commitment boundary hardening

Scope:
- Ensure all irreversible side-effect paths require commitment receipts.
- Eliminate direct driver entry points that bypass gate checks.

Acceptance:
- Driver APIs require receipt context.
- Side-effect tests without receipts fail.
- Conformance invariants for commitment gating pass.

Prompt skeleton:
1. Enumerate consequence execution entrypoints.
2. Refactor to receipt-first driver interfaces.
3. Add regression tests for bypass attempts.
4. Update boundary docs and operational runbooks.

## Stage 3 - Ledger module extraction

Scope:
- Formalize WLL writer/reader traits and implementation boundary.
- Add stable projections (`latest_state`, `audit_index`) from receipts.

Acceptance:
- Replay from snapshot + receipts converges.
- Projection rebuild is deterministic and idempotent.
- Ledger invariants L1-L5 are tested.

Prompt skeleton:
1. Define `LedgerWriter` and `LedgerReader` traits.
2. Move concrete storage implementation behind trait boundary.
3. Implement projection builders and replay checks.
4. Add invariants and load tests.

## Stage 4 - Governance naming and agentic operations

Scope:
- Introduce `worldline-governance` naming across docs and new API aliases.
- Optional operator plane:
  - `worldline-operator-bot`
  - `worldline-promptkit`

Acceptance:
- New naming is primary in docs and examples.
- Legacy PALM names remain available for one cycle.
- Operator bot can perform policy-safe ops loops through the same boundary.

Prompt skeleton:
1. Add governance naming aliases (crate/module/API).
2. Keep PALM compatibility exports and endpoint aliases.
3. Add operator-bot reference implementation (watch -> propose -> commit -> audit).
4. Add promptkit templates (operator prompts + tool contracts + runbooks).

## Compatibility policy (one cycle)

- Keep old crate names and endpoint aliases stable.
- Emit migration warnings in docs and release notes.
- Remove aliases only after one planned release cycle with explicit cutoff date.
