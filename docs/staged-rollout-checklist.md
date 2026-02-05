# MAPLE Staged Rollout Checklist (Stage 1-5)

This checklist is the release-readiness control sheet for the staged implementation plan.

Updated: 2026-02-05
Status: Stages 1-5 completed

## Post-Stage Hardening Update (Gateway + Durable Receipts)

Additional hardening completed after Stage 5:

- Removed the last direct capability execution branch in `AgentKernel`; all capability execution now routes through `CommitmentGateway`.
- Added durable, replayable tool receipts in AAS ledger.
- Added runtime policy guardrails for:
  - profile tier (`mapleverse`/`finalverse`/`ibank`)
  - attention budget limits
  - capability risk (`safe`/`dangerous`)
  - autonomous spending thresholds
- Added bypass detector coverage that panics in tests if executors are invoked outside gateway context.

Validation:

```bash
cargo test -p aas-policy -p aas-ledger -p maple-runtime --offline
cargo test --workspace --offline
```

## Stage Summary

| Stage | Name | Status |
|-------|------|--------|
| 1 | Canonical Profile Layer | Completed |
| 2 | Commitment Gateway Unification | Completed |
| 3 | ModelAdapter Contract Hardening | Completed |
| 4 | Connector Boundary Enforcement | Completed |
| 5 | Docs + Boundary Examples | Completed |

## Stage 1: Canonical Profile Layer

### Files
- `crates/resonator/profiles/src/lib.rs`
- `crates/resonator/types/src/lib.rs`
- `crates/maple-runtime/src/types/profile.rs`
- `crates/maple-runtime/src/runtime_core/profile_manager.rs`

### Acceptance Commands
```bash
cargo test -p resonator-profiles --offline
cargo test -p maple-runtime --offline
```

### Release Gate
- `resonator-profiles` validator rejects invalid profile constraints.
- `maple-runtime` rejects invalid profile registration.

## Stage 2: Commitment Gateway Unification

### Files
- `crates/maple-runtime/src/agent_kernel/mod.rs`
- `crates/aas-ledger/src/lib.rs`
- `crates/aas-service/src/lib.rs`
- `crates/maple/storage/src/model.rs`
- `crates/maple/storage/src/memory.rs`
- `crates/maple/storage/src/postgres.rs`
- `crates/palm/daemon/src/api/rest/handlers/agent_kernel.rs`
- `crates/maple-cli/src/main.rs`

### Acceptance Commands
```bash
cargo test -p aas-ledger -p aas-service -p maple-runtime --offline
cargo test -p palm-daemon -p maple-cli --offline
```

### Release Gate
- Dangerous capability is denied without commitment.
- Approved commitment allows consequence and records durable lifecycle outcome.
- Commitment lifecycle timestamps persist across restarts with PostgreSQL storage.

## Stage 3: ModelAdapter Contract Hardening

### Files
- `crates/maple-runtime/src/cognition/mod.rs`
- `crates/maple-runtime/src/cognition/llama.rs`
- `crates/maple-runtime/src/cognition/openai.rs`
- `crates/maple-runtime/src/cognition/anthropic.rs`
- `crates/maple-runtime/src/cognition/gemini.rs`
- `crates/maple-runtime/src/cognition/grok.rs`
- `crates/maple-runtime/src/agent_kernel/mod.rs`
- `crates/maple-runtime/src/lib.rs`

### Acceptance Commands
```bash
cargo test -p maple-runtime --offline
```

### Release Gate
- Malformed model output triggers deterministic fallback.
- Fallback never suggests tool execution.
- Commitment gating behavior remains identical across all backends.

## Stage 4: Connector Boundary Enforcement

### Files
- `crates/mapleverse/connectors/src/lib.rs`
- `crates/mapleverse/executor/src/lib.rs`
- `crates/mapleverse/service/src/lib.rs`
- `crates/mrp-transport/src/lib.rs`
- `crates/mrp-transport/Cargo.toml`
- `crates/mrp-service/src/lib.rs`
- `crates/mrp-service/Cargo.toml`

### Acceptance Commands
```bash
cargo test -p mapleverse-executor --offline
cargo test -p mrp-service --offline
cargo test -p mapleverse-connectors -p mapleverse-service --offline
```

### Release Gate
- Execution without explicit commitment reference is rejected.
- Multi-leg failures are explicit and logged as outcomes.
- No silent drops in transport/service execution paths.

## Stage 5: Docs + Boundary Examples

### Files
- `crates/maple-runtime/examples/06_agent_kernel_boundary.rs`
- `examples/boundary-demo/src/main.rs`
- `examples/boundary-demo/Cargo.toml`
- `docs/concepts/agent-kernel-composition.md`
- `docs/concepts/agent-resonator-llm.md`

### Acceptance Commands
```bash
cargo run -p maple-runtime --example 06_agent_kernel_boundary --offline
cargo run -p palm-boundary-demo --offline
```

### Release Gate
- Runtime boundary demo shows safe path, denied dangerous path, approved dangerous path, and persisted lifecycle timestamps.
- Ops/platform demo includes explicit “no commitment, no consequence” scenario.
- Docs match current API and CLI behavior.

## Final Workspace Gate

Run before tagging/release:

```bash
cargo check --workspace --offline
```
