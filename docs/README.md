# MAPLE Documentation

Welcome to the MAPLE AI Framework documentation.

## Contents

| Document | Description |
|----------|-------------|
| [WorldLine Framework Guide](worldline-framework.md) | Prompt 1-28 implementation map, interfaces, and verification commands |
| [WorldLine Quickstart](tutorials/worldline-quickstart.md) | End-to-end CLI/API/demo walkthrough for WorldLine |
| [Architecture 00: Overview](architecture/00-overview.md) | EVOS system map, data plane vs control plane split, invariants |
| [Architecture 01: WorldLine](architecture/01-worldline.md) | Canonical WorldLine model and lifecycle |
| [Architecture 03: Commitment Boundary](architecture/03-commitment-boundary.md) | Non-bypassable gating and policy/capability enforcement |
| [Architecture 04: Ledger WLL](architecture/04-ledger-wll.md) | Append-only ledger model, replay, and proof invariants |
| [Architecture Migration Plan](architecture/phase-plan.md) | Phase A-D migration from PALM naming to WorldLine-governance naming |
| [Architecture Guide](architecture.md) | Deep dive into MAPLE architecture |
| [Repository Structure](repo-structure.md) | Layered crate organization (`palm/*`, `resonator/*`, `mapleverse/*`) |
| [Getting Started](getting-started.md) | Build your first MAPLE app |
| [Platform Packs Tutorial](tutorials/platform-packs.md) | Creating custom platform packs |
| [Operations Tutorial](tutorials/operations.md) | Daemon, CLI, and Playground workflows |
| [iBank Commitment Boundary Tutorial](tutorials/ibank-commitment-boundary.md) | End-to-end contract-gated transfer demo with receipts |
| [Conformance Guide](conformance.md) | Testing platform pack compliance |
| [API Reference](api/README.md) | Complete API documentation |
| [Core: Agents](core/agents.md) | Agent composition + commitment gateway model |
| [Core: Interop](core/interop.md) | MCP/A2A/vendor SDK mapping to MAPLE boundaries |
| [Core: Llama-First](core/llama-first.md) | Llama cognition contract, repair, gating, replay |
| [UAL (Universal Agent Language)](concepts/ual.md) | Agent/human interaction language |
| [Agents, Resonators, and LLMs](concepts/agent-resonator-llm.md) | Mental model + backend integration |
| [Agent Kernel Composition](concepts/agent-kernel-composition.md) | Non-bypassable `Agent = Resonator + Profile + Capability + Contracts` runtime model |
| [Storage Layer](concepts/storage-layer.md) | Source-of-truth + AI-memory storage architecture |
| [Staged Rollout Checklist](staged-rollout-checklist.md) | Stage 1-5 implementation status, files, and release commands |

## Quick Links

- [Main README](../README.md) - Project overview and quick start
- [Examples and Demos](../examples/README.md) - Runnable WorldLine demos
- [Contributing](../CONTRIBUTING.md) - How to contribute
- [Changelog](../CHANGELOG.md) - Version history

## Getting Started

1. Read the [Architecture Guide](architecture.md) to understand MAPLE's design
2. Follow the [Platform Packs Tutorial](tutorials/platform-packs.md) to create your first pack
3. Use the [Conformance Guide](conformance.md) to validate your implementation
4. Reference the [API docs](api/README.md) as needed
