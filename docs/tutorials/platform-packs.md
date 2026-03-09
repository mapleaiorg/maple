# Platform Packs

This tutorial covers the current platform-pack implementation surfaces that feed PALM and the broader Agent OS control plane.

## 1. Create a new pack crate

```bash
cargo new --lib my-platform-pack
cd my-platform-pack
```

## 2. Add the contract dependencies

```toml
[dependencies]
palm-platform-pack = { path = "../contracts/platform-pack" }
palm-types = { path = "../crates/palm/types" }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
tracing = "0.1"
```

## 3. Implement the contract

Platform packs currently define:

- profile metadata
- policy defaults
- health configuration
- state behavior
- resource limits
- recovery strategy
- supported capability surface

After implementation, validate with:

```bash
cargo test -p palm-conformance
```

## 4. Why platform packs matter

Platform packs are one of the bridges between the older profile-specific implementation surfaces and the newer Agent OS packaging and fleet model. They let you define deployment defaults and control-plane expectations without hard-coding them in the daemon.

## 5. Current built-in profiles

- Mapleverse
- Finalverse
- iBank
- development

These remain useful as compatibility and domain-shaping profiles while the top-level product story converges on Runtime, Registry, Models, Guard, Foundry, and Fleet.

## Next

- [Operations Tutorial](operations.md)
- [Architecture Overview](../architecture/overview.md)
