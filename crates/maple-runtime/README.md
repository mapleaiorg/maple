# maple-runtime

`maple-runtime` is the MAPLE SDK runtime crate for resonator lifecycle, coupling, temporal coordination, invariants, and optional commitment-gated AgentKernel execution.

This README reflects the current `main` architecture where runtime users can choose between a minimal core footprint and the full boundary stack.

## Install

```toml
[dependencies]
maple-runtime = "0.1.2"
```

## Build Modes

### Default mode (full runtime experience)

Default features include:

- `cognition`
- `agent-kernel`
- `profile-validation`
- profile marker features: `mapleverse`, `finalverse`, `ibank`

This mode compiles adapters and AgentKernel commitment boundary integrations.

### Core-only mode (more independent)

```toml
[dependencies]
maple-runtime = { version = "0.1.2", default-features = false }
```

Core-only mode keeps:

- `MapleRuntime`
- resonator registration + coupling APIs
- attention allocator / scheduler / telemetry scaffolding
- runtime types + invariants

And excludes heavy boundary dependencies (AAS/RCF AgentKernel path).

### Optional feature packs

- `cognition`: enable cognition adapters and structured draft types
- `agent-kernel`: enable non-bypassable handle path (`Agent = Resonator + Profile + Capability + Contract`)
- `profile-validation`: use canonical `resonator-profiles` archetype validation
- `memory-conversation`: enables example `08_memory_and_conversation`
- `observability-examples`: enables example `09_observability_demo`
- `conformance-examples`: enables example `10_conformance_testing`

## Quick Usage

```rust
use maple_runtime::{config::RuntimeConfig, MapleRuntime, ResonatorSpec};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MapleRuntime::bootstrap(RuntimeConfig::default()).await?;
    let resonator = runtime.register_resonator(ResonatorSpec::default()).await?;

    resonator.signal_presence(maple_runtime::PresenceState::new()).await?;

    runtime.shutdown().await?;
    Ok(())
}
```

## Examples

```bash
# core
cargo run -p maple-runtime --example 01_basic_resonator
cargo run -p maple-runtime --example 02_resonator_coupling

# profile config examples
cargo run -p maple-runtime --example 03_mapleverse_config
cargo run -p maple-runtime --example 04_finalverse_config
cargo run -p maple-runtime --example 05_ibank_config

# feature-gated examples
cargo run -p maple-runtime --example 06_agent_kernel_boundary --features agent-kernel
cargo run -p maple-runtime --example 08_memory_and_conversation --features memory-conversation
cargo run -p maple-runtime --example 09_observability_demo --features observability-examples
cargo run -p maple-runtime --example 10_conformance_testing --features conformance-examples
```

## Validate Both Modes

```bash
# default feature set
cargo check -p maple-runtime
cargo test -p maple-runtime

# independent core mode
cargo check -p maple-runtime --no-default-features
cargo test -p maple-runtime --no-default-features --lib
```

## Architecture Summary

`maple-runtime` centers on:

- `runtime_core`: bootstrap, registry, handles, continuity
- `fabrics`: presence and coupling fabrics
- `allocator`: attention budget accounting
- `invariants`: architectural guardrails and operation checks
- `temporal`: temporal anchors and ordering
- `agent_kernel` (feature): commitment-gated capability execution path
- `cognition` (feature): provider adapters and structured cognition drafts

For design context and end-to-end operations:

- [docs/tutorials/maple-runtime-standalone.md](../../docs/tutorials/maple-runtime-standalone.md)
- [docs/tutorials/ibank-commitment-boundary.md](../../docs/tutorials/ibank-commitment-boundary.md)
- [docs/tutorials/worldline-quickstart.md](../../docs/tutorials/worldline-quickstart.md)
