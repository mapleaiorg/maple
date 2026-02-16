# WorldLine Examples and Demos

All examples are workspace members and can be run directly.

Phase A note:
- `mwl-*` demos use the facade crates (`worldline-core`, `worldline-runtime`, `worldline-ledger`).
- Legacy `maple-*` crates remain supported for one compatibility cycle.

## Core WorldLine Demos

| Example | Focus | Run |
|---|---|---|
| `mwl-worldline-lifecycle` | Deterministic identity, event fabric, integrity, provenance | `cargo run --manifest-path examples/mwl-worldline-lifecycle/Cargo.toml` |
| `mwl-commitment-gate` | 7-stage commitment adjudication and denial paths | `cargo run --manifest-path examples/mwl-commitment-gate/Cargo.toml` |
| `mwl-provenance-audit` | Worldline-scoped audit history and causal verification | `cargo run --manifest-path examples/mwl-provenance-audit/Cargo.toml` |
| `mwl-human-agency` | Consent protocol, coercion detection, profile restrictions | `cargo run --manifest-path examples/mwl-human-agency/Cargo.toml` |
| `mwl-financial-settlement` | EVOS projection, DvP atomicity, regulatory checks | `cargo run --manifest-path examples/mwl-financial-settlement/Cargo.toml` |

## Platform Boundary Demo

| Example | Focus | Run |
|---|---|---|
| `boundary-demo` | Policy differences across Mapleverse/Finalverse/iBank | `cargo run --manifest-path examples/boundary-demo/Cargo.toml` |

## Suggested Order

1. `mwl-worldline-lifecycle`
2. `mwl-commitment-gate`
3. `mwl-provenance-audit`
4. `mwl-human-agency`
5. `mwl-financial-settlement`
6. `boundary-demo`
