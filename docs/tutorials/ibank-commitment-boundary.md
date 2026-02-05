# iBank Commitment Boundary Tutorial

This tutorial demonstrates the full MAPLE commitment boundary flow for an iBank agent:

1. Register iBank agent capabilities:
   - `echo` (safe)
   - `transfer_funds` (dangerous, simulated only)
2. Attempt `transfer_funds` without a contract -> denied.
3. Draft and declare a commitment -> execute through `CommitmentGateway`.
4. Verify receipt persistence in AAS ledger.

## Run Paths

### Direct Example (recommended for quick verification)

```bash
cargo run -p maple-runtime --example 06_agent_kernel_boundary --offline
```

Expected output includes:

- denial log for transfer without commitment (`ContractMissing`)
- declared contract id
- receipt id + receipt status
- stage transition logs (`meaning -> intent -> commitment -> consequence`)

### Daemon + CLI Path (ops flow)

Start daemon with iBank + memory storage:

```bash
cargo run -p maple-cli -- daemon start --platform ibank --storage memory
```

Then use CLI agent commands (`maple agent handle`, `maple agent commitment`, `maple agent commitments`) for API-based operation and inspection.

## Contract-Boundary Scenario

Prompt:

`transfer $100 to Alice`

Positive path:

- contract drafted with capability-scoped binding
- policy approves within iBank risk/autonomy thresholds
- capability execution runs via `CommitmentGateway`
- receipt is persisted to AAS ledger
- lifecycle reaches completed/fulfilled state

Negative path:

- dangerous capability call without contract is denied
- failure is explicit and auditable

## Validation

Run integration coverage:

```bash
cargo test -p maple-runtime --offline
```

The integration test `crates/maple-runtime/tests/ibank_commitment_boundary.rs` asserts:

- transfer without contract is denied
- transfer with contract persists a receipt in ledger replay
