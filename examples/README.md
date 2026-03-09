# MAPLE Examples

These examples are the fastest way to see the runtime, provenance, and commitment model in action without reading the entire architecture set first.

## Recommended Order

1. `mwl-worldline-lifecycle`
2. `mwl-commitment-gate`
3. `mwl-provenance-audit`
4. `mwl-human-agency`
5. `mwl-financial-settlement`

## WorldLine Example Programs

| Example | Focus | Run |
| --- | --- | --- |
| `mwl-worldline-lifecycle` | identity, event flow, receipts | `cargo run --manifest-path examples/mwl-worldline-lifecycle/Cargo.toml` |
| `mwl-commitment-gate` | authorization, denial, consequence | `cargo run --manifest-path examples/mwl-commitment-gate/Cargo.toml` |
| `mwl-provenance-audit` | lineage and replay | `cargo run --manifest-path examples/mwl-provenance-audit/Cargo.toml` |
| `mwl-human-agency` | consent and disengagement protections | `cargo run --manifest-path examples/mwl-human-agency/Cargo.toml` |
| `mwl-financial-settlement` | financial consequence and projection | `cargo run --manifest-path examples/mwl-financial-settlement/Cargo.toml` |

## Runtime Examples

`maple-runtime` examples are useful when you want to focus on the kernel and cognition layers rather than the full daemon path.

| Example | Focus | Run |
| --- | --- | --- |
| `01_basic_resonator` | bootstrap and presence | `cargo run -p maple-runtime --example 01_basic_resonator` |
| `02_resonator_coupling` | coupling and attention | `cargo run -p maple-runtime --example 02_resonator_coupling` |
| `06_agent_kernel_boundary` | commitment-gated action | `cargo run -p maple-runtime --example 06_agent_kernel_boundary --features agent-kernel` |
| `08_memory_and_conversation` | memory and conversation | `cargo run -p maple-runtime --example 08_memory_and_conversation --features memory-conversation` |
| `09_observability_demo` | receipts and runtime signals | `cargo run -p maple-runtime --example 09_observability_demo --features observability-examples` |

## Related Docs

- [5-Minute Quickstart](../docs/getting-started/quickstart.md)
- [WorldLine Quickstart Tutorial](../docs/tutorials/worldline-quickstart.md)
- [Operations Tutorial](../docs/tutorials/operations.md)
