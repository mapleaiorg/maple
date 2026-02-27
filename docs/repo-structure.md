# Repository Structure

MAPLE groups all subsystems under dedicated layer directories so the crate graph is easier to understand and reuse.

## Layer Folders

### Constitutional & Protocol Foundations
- `crates/rcf/*`: Resonance Constitutional Framework (types, meaning, intent, commitment, validator, compiler, audit).
- `crates/ual/*`: Universal Agent Language (types, parser, compiler).
- `crates/mrp/*`: MAPLE Routing Protocol (types, router, transport, service).

### Authority & Learning
- `crates/aas/*`: Agent Accountability Service (types, identity, capability, policy, adjudication, ledger, service).
- `crates/eve/*`: Epistemic Validation Engine (types, ingestion, evaluation, artifacts, service).

### Multi-Agent Coordination
- `crates/collective/*`: Multi-agent organization primitives (types, runtime).
- `crates/workflow/*`: Receipt-gated workflow engine (types, engine, dsl).

### WorldLine Framework
- `crates/worldline/*`: Canonical WorldLine namespace (types, identity, core, runtime, ledger, governance, operator-bot, promptkit, substrate, conformance, integration).
- `crates/kernel/*`: WorldLine kernel subsystems (fabric, memory, gate, mrp, provenance, governance, safety, profiles, financial, sdk).
- `crates/substrate/*`: Self-producing WorldLine substrate (observation, meaning, intent, commitment, consequence, self-mod-gate, codegen, deployment, ir, langgen, compiler, sal, hardware, bootstrap, evos, conformance, integration-suite, conformance-suite).
- `crates/mwl/*`: MWL compatibility wrappers (types, identity, integration, conformance).
- `crates/waf/*`: WorldLine Autopoietic Factory (context-graph, evidence, resonance-monitor, evolution-engine, compiler, wlir, swap-gate, governance, genesis, kernel, tests, demo).

### Cognition & Execution
- `crates/resonator/*`: Resonator cognition/lifecycle layer (types, identity, meaning, intent, commitment, consequence, memory, conversation, profiles, runtime, client, cli, observability, conformance).
- `crates/mapleverse/*`: Mapleverse execution layer (types, executor, connectors, evidence, service, world).

### Operations & Orchestration
- `crates/palm/*`: PALM operational layer (types, registry, deployment, health, state, control, policy, shared-state, cli, daemon, observability).
- `crates/maple/*`: Shared MAPLE service layer (storage, model-openai, model-anthropic, model-gemini, model-grok, protocol-mcp, protocol-a2a).

### Top-Level Entry Points (flat)
- `crates/maple-runtime`: Core MAPLE runtime hub.
- `crates/maple-integration`: Integration facade.
- `crates/maple-cli`: CLI entry point.

## Why this is better

- Clear bounded contexts by directory, not just crate naming prefixes.
- Better discoverability for onboarding (`crates/<layer>/<component>` is predictable).
- Cleaner dependency pathing for local path dependencies.
- Easier selective packaging/deployment by layer.

## Notes

- Crate package names remain stable (`rcf-types`, `palm-daemon`, `maple-kernel-fabric`, etc.) to preserve compatibility.
- Cargo workspace members are updated to new paths.
- Existing commands (`cargo run -p palm-daemon`, `cargo run -p maple-cli`) remain unchanged.
- `maple-runtime` supports standalone core builds via `cargo check -p maple-runtime --no-default-features`.
- WorldLine governance API aliases are available under `/api/v1/worldline-governance/*`.
- WorldLine command groups are exposed via `maple worldline|commit|provenance|financial|gov|kernel` (`policy` remains as alias).
- Root `storage/` is reserved for ops assets (migrations/bootstrap scripts), while storage code lives in `crates/maple/storage`.
