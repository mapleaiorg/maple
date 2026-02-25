# Crate Layout

The workspace groups all architecture layers into explicit folders.

## Layered folders

### Constitutional & Protocol Foundations
- `rcf/`: Resonance Constitutional Framework (types, meaning, intent, commitment, validator, compiler, audit)
- `ual/`: Universal Agent Language (types, parser, compiler)
- `mrp/`: MAPLE Routing Protocol (types, router, transport, service)

### Authority & Learning
- `aas/`: Agent Accountability Service (types, identity, capability, policy, adjudication, ledger, service)
- `eve/`: Epistemic Validation Engine (types, ingestion, evaluation, artifacts, service)

### Multi-Agent Coordination
- `collective/`: Multi-agent organization primitives (types, runtime)
- `workflow/`: Receipt-gated workflow engine (types, engine, dsl)

### WorldLine Framework
- `worldline/`: Canonical WorldLine entrypoints (types, identity, core, runtime, ledger, governance, operator-bot, promptkit, substrate, conformance, integration)
- `kernel/`: WorldLine kernel subsystems (fabric, memory, gate, mrp, provenance, governance, safety, profiles, financial, sdk)
- `substrate/`: Self-producing WorldLine substrate (observation, meaning, intent, commitment, consequence, self-mod-gate, codegen, deployment, ir, langgen, compiler, sal, hardware, bootstrap, evos, conformance)
- `mwl/`: MWL compatibility wrappers (types, identity, integration, conformance)
- `waf/`: WorldLine Autopoietic Factory (context-graph, evidence, resonance-monitor, evolution-engine, compiler, wlir, swap-gate, governance, genesis, kernel, tests, demo)

### Cognition & Execution
- `resonator/`: Cognition and lifecycle crates (types, identity, meaning, intent, commitment, consequence, memory, conversation, profiles, runtime, client, cli, observability, conformance)
- `mapleverse/`: World execution and integration crates (types, executor, connectors, evidence, service, world)

### Operations & Orchestration
- `palm/`: Operational control-plane and tooling (types, registry, deployment, health, state, control, policy, shared-state, cli, daemon, observability)
- `maple/`: Cross-runtime shared services (storage, model-openai, model-anthropic, model-gemini, model-grok, protocol-mcp, protocol-a2a)

## Flat crates (top-level entry points)

- `maple-runtime`: Core MAPLE runtime hub
- `maple-integration`: Integration facade
- `maple-cli`: CLI entry point

Package names remain stable (`rcf-types`, `maple-kernel-fabric`, etc.) — only directory paths changed.
