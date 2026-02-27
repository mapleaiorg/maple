# MAPLE AI Framework

MAPLE is a Rust workspace for building resonance-native multi-agent systems with explicit commitment boundaries, auditable consequence flow, and WorldLine continuity.

The repository currently exposes two complementary entry paths:

- `maple-runtime`: runtime-first SDK for resonator lifecycle, coupling, cognition adapters, and AgentKernel boundary execution.
- `worldline-*` + `palm-*`: canonical WorldLine kernel/governance crates and operational control plane.

## Current Status (main)

This repository is aligned with upstream `mapleaiorg/maple` `main` at commit `759a5f49` (2026-02-26 fetch).

Recent practical updates reflected in this docs refresh:

- `maple-runtime` now supports a **minimal standalone core mode** via Cargo feature gates.
- AgentKernel/cognition/accountability dependencies can be opted in only when needed.
- Example and test build requirements are explicitly feature-gated.
- Docs/tutorials now separate **core runtime** workflows from **full WorldLine + PALM ops** workflows.

## Quick Start

### Prerequisites

- Rust `1.80+`
- Cargo
- Optional for ops tutorials: PostgreSQL and Ollama

### Clone and Build

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple
cargo build --release
```

### Run the first runtime example

```bash
cargo run -p maple-runtime --example 01_basic_resonator
```

### Run WorldLine demo examples

```bash
cargo run --manifest-path examples/mwl-worldline-lifecycle/Cargo.toml
cargo run --manifest-path examples/mwl-commitment-gate/Cargo.toml
```

## maple-runtime Independence Modes

`maple-runtime` now supports two practical dependency modes.

### 1) Core runtime only (most independent)

Use this when you only need runtime lifecycle/coupling/attention primitives.

```toml
[dependencies]
maple-runtime = { version = "0.1.2", default-features = false }
tokio = { version = "1", features = ["full"] }
```

### 2) Full runtime boundary stack

Use this when you need cognition adapters + AgentKernel commitment gating.

```toml
[dependencies]
maple-runtime = { version = "0.1.2", features = ["cognition", "agent-kernel", "profile-validation"] }
tokio = { version = "1", features = ["full"] }
```

### Validate both build modes in this repo

```bash
cargo check -p maple-runtime
cargo check -p maple-runtime --no-default-features
```

## Core Commands

### Runtime and examples

```bash
cargo test -p maple-runtime
cargo run -p maple-runtime --example 02_resonator_coupling
cargo run -p maple-runtime --example 06_agent_kernel_boundary --features agent-kernel
cargo run -p maple-runtime --example 08_memory_and_conversation --features memory-conversation
cargo run -p maple-runtime --example 09_observability_demo --features observability-examples
cargo run -p maple-runtime --example 10_conformance_testing --features conformance-examples
```

### WorldLine / PALM operations

```bash
cargo run -p palm-daemon
cargo run -p maple-cli -- worldline list
cargo run -p maple-cli -- kernel status
cargo run -p maple-cli -- gov list
```

## Repository Layout

See [docs/repo-structure.md](docs/repo-structure.md) for complete details.

High-level layout:

- `crates/maple-runtime`: runtime SDK and boundary kernel APIs
- `crates/worldline/*`: canonical WorldLine namespaces
- `crates/palm/*`: control plane, daemon, and ops tooling
- `crates/resonator/*`: cognition/memory/conformance layer
- `examples/*`: runnable lifecycle/commitment/provenance demos
- `docs/*`: architecture, API, tutorials, rollout guides

## Documentation Map

- [docs/README.md](docs/README.md)
- [docs/getting-started.md](docs/getting-started.md)
- [docs/tutorials/maple-runtime-standalone.md](docs/tutorials/maple-runtime-standalone.md)
- [docs/tutorials/worldline-quickstart.md](docs/tutorials/worldline-quickstart.md)
- [docs/tutorials/operations.md](docs/tutorials/operations.md)
- [docs/worldline-framework.md](docs/worldline-framework.md)

## Contributing

- [CONTRIBUTING.md](CONTRIBUTING.md)
- [CHANGELOG.md](CHANGELOG.md)
- [ROADMAP.md](ROADMAP.md)

## License

Dual-licensed under MIT OR Apache-2.0.
