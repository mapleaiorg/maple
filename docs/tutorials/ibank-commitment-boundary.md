# iBank Commitment Boundary

This tutorial demonstrates the most important financial safety rule in MAPLE: no high-risk financial consequence should execute without an explicit commitment and an inspectable receipt trail.

## 1. Run the boundary example

```bash
cargo run -p maple-runtime --example 06_agent_kernel_boundary --features agent-kernel
```

Look for:

- a dangerous path denied without commitment
- a dangerous path allowed when a commitment exists
- explicit stage transitions through meaning, intent, commitment, and consequence

## 2. Run the integration tests

```bash
cargo test -p maple-runtime --test ibank_commitment_boundary
cargo test -p maple-runtime --test model_adapter_conformance
```

## 3. Optional daemon and CLI flow

```bash
cargo run -p palm-daemon
cargo run -p maple-cli -- agent demo --dangerous --prompt "transfer 500 usd to demo"
cargo run -p maple-cli -- agent demo --dangerous --with-commitment --amount 500 --prompt "transfer 500 usd to demo"
cargo run -p maple-cli -- agent audit --limit 20
```

## 4. What success looks like

- dangerous capability calls without commitment are denied
- valid commitments produce traceable receipts
- outcome inspection is possible after execution
- provenance makes the full decision chain reviewable

## Next

- [Guard and Policies](../guides/guard-policies.md)
- [Operations Tutorial](operations.md)
- [Architectural Invariants](../reference/invariants.md)
