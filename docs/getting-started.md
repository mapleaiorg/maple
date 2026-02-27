# Getting Started

This guide gives the shortest path to run MAPLE today, with two tracks:

- `maple-runtime` SDK track (library-first)
- WorldLine + PALM ops track (daemon + CLI)

## Prerequisites

- Rust `1.80+`
- Cargo
- Optional for ops tutorials: PostgreSQL and Ollama

## Track A: maple-runtime SDK

### A1) Use core-only independent mode

```toml
[dependencies]
maple-runtime = { version = "0.1.2", default-features = false }
tokio = { version = "1", features = ["full"] }
```

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

### A2) Enable boundary/cognition stack when needed

```toml
[dependencies]
maple-runtime = { version = "0.1.2", default-features = false, features = ["cognition", "agent-kernel", "profile-validation"] }
tokio = { version = "1", features = ["full"] }
```

### A3) Validate both build matrices

```bash
cargo check -p maple-runtime
cargo test -p maple-runtime
cargo check -p maple-runtime --no-default-features
cargo test -p maple-runtime --no-default-features --lib
```

## Track B: WorldLine + PALM Ops

### B1) Clone and build

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple
cargo build
```

### B2) Start daemon

```bash
cargo run -p palm-daemon
```

### B3) Use CLI

```bash
cargo run -p maple-cli -- worldline list
cargo run -p maple-cli -- kernel status
cargo run -p maple-cli -- gov list
```

### B4) Run end-to-end tutorial

Follow [tutorials/worldline-quickstart.md](tutorials/worldline-quickstart.md).

## Recommended Example Order

```bash
cargo run -p maple-runtime --example 01_basic_resonator
cargo run -p maple-runtime --example 02_resonator_coupling
cargo run -p maple-runtime --example 03_mapleverse_config
cargo run -p maple-runtime --example 04_finalverse_config
cargo run -p maple-runtime --example 05_ibank_config
cargo run -p maple-runtime --example 06_agent_kernel_boundary --features agent-kernel
```

## Related Docs

- [Maple Runtime Standalone Tutorial](tutorials/maple-runtime-standalone.md)
- [iBank Commitment Boundary Tutorial](tutorials/ibank-commitment-boundary.md)
- [Operations Tutorial](tutorials/operations.md)
- [WorldLine Framework Guide](worldline-framework.md)
