# WorldLine Examples and Demos

All examples are workspace members and run directly from this repo.

## Core WorldLine Demos

| Example | Focus | Run |
|---|---|---|
| `mwl-worldline-lifecycle` | Identity, event fabric, provenance | `cargo run --manifest-path examples/mwl-worldline-lifecycle/Cargo.toml` |
| `mwl-commitment-gate` | Commitment adjudication and denial paths | `cargo run --manifest-path examples/mwl-commitment-gate/Cargo.toml` |
| `mwl-provenance-audit` | Causal history and lineage checks | `cargo run --manifest-path examples/mwl-provenance-audit/Cargo.toml` |
| `mwl-human-agency` | Consent/coercion protections | `cargo run --manifest-path examples/mwl-human-agency/Cargo.toml` |
| `mwl-financial-settlement` | DvP-style settlement and projection | `cargo run --manifest-path examples/mwl-financial-settlement/Cargo.toml` |

## Runtime Examples (`maple-runtime`)

| Example | Focus | Run |
|---|---|---|
| `01_basic_resonator` | Runtime bootstrap + register + presence | `cargo run -p maple-runtime --example 01_basic_resonator` |
| `02_resonator_coupling` | Coupling/attention behavior | `cargo run -p maple-runtime --example 02_resonator_coupling` |
| `03_mapleverse_config` | Mapleverse profile defaults | `cargo run -p maple-runtime --example 03_mapleverse_config` |
| `04_finalverse_config` | Finalverse profile defaults | `cargo run -p maple-runtime --example 04_finalverse_config` |
| `05_ibank_config` | iBank profile defaults | `cargo run -p maple-runtime --example 05_ibank_config` |
| `06_agent_kernel_boundary` | Commitment-gated dangerous capability path | `cargo run -p maple-runtime --example 06_agent_kernel_boundary --features agent-kernel` |
| `08_memory_and_conversation` | Memory and conversation stack | `cargo run -p maple-runtime --example 08_memory_and_conversation --features memory-conversation` |
| `09_observability_demo` | Observability components | `cargo run -p maple-runtime --example 09_observability_demo --features observability-examples` |
| `10_conformance_testing` | Conformance checks | `cargo run -p maple-runtime --example 10_conformance_testing --features conformance-examples` |

## Suggested Order

1. `01_basic_resonator`
2. `02_resonator_coupling`
3. `06_agent_kernel_boundary`
4. `mwl-worldline-lifecycle`
5. `mwl-commitment-gate`
