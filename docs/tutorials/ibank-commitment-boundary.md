# iBank Commitment Boundary Tutorial

This tutorial verifies MAPLE's non-bypassable commitment boundary for dangerous financial capabilities.

## What This Covers

1. Register safe and dangerous capabilities.
2. Observe denial for dangerous execution without commitment.
3. Execute dangerous path with explicit commitment.
4. Verify receipt persistence and auditability.

## 1. Run the Runtime Example

`06_agent_kernel_boundary` is feature-gated behind `agent-kernel`.

```bash
cargo run -p maple-runtime --example 06_agent_kernel_boundary --features agent-kernel
```

Expected output includes:

- dangerous call denied as `ContractMissing`
- commitment id emission
- receipt id + status
- stage transitions (`meaning -> intent -> commitment -> consequence`)

## 2. Run Integration Tests

```bash
cargo test -p maple-runtime --test ibank_commitment_boundary
cargo test -p maple-runtime --test model_adapter_conformance
```

## 3. Optional Daemon + CLI Path

```bash
cargo run -p palm-daemon
cargo run -p maple-cli -- agent status
cargo run -p maple-cli -- agent handle \
  --prompt "transfer 500 usd to demo" \
  --tool simulate_transfer \
  --args '{"amount":500,"to":"demo"}' \
  --with-commitment
cargo run -p maple-cli -- agent audit --limit 20
cargo run -p maple-cli -- agent commitments --limit 20
```

## 4. Validation Criteria

Success criteria:

- dangerous capability without commitment is denied
- dangerous capability with valid commitment executes
- receipt is persisted and queryable by commitment id
- audit trail keeps explicit stage transitions and outcomes

## Next

- [Maple Runtime Standalone Tutorial](maple-runtime-standalone.md)
- [Operations Tutorial](operations.md)
