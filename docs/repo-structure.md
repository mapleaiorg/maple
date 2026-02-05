# Repository Structure

MAPLE now groups major subsystems under dedicated layer directories so the crate graph is easier to understand and reuse.

## Layer Folders

- `crates/palm/*`: PALM operational layer (CLI, daemon, policy, control, registry, deployment, health, state, shared-state, observability, types).
- `crates/resonator/*`: Resonator cognition/lifecycle layer (types, identity, meaning, intent, commitment, profiles, runtime, client).
- `crates/mapleverse/*`: Mapleverse execution layer (types, executor, connectors, evidence, service, world).
- `crates/maple/*`: shared MAPLE service layer (storage contracts/adapters and future cross-runtime services).

## Why this is better

- Clear bounded contexts by directory, not just crate naming prefixes.
- Better discoverability for onboarding (`crates/<layer>/<component>` is predictable).
- Cleaner dependency pathing for local path dependencies.
- Easier selective packaging/deployment by layer.

## Notes

- Crate package names remain stable (`palm-daemon`, `resonator-types`, etc.) to preserve compatibility.
- Cargo workspace members are updated to new paths.
- Existing commands (`cargo run -p palm-daemon`, `cargo run -p maple-cli`) remain unchanged.
- Root `storage/` is reserved for ops assets (migrations/bootstrap scripts), while storage code lives in `crates/maple/storage`.
