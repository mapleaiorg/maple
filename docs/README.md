# MAPLE Documentation

Welcome to the MAPLE AI Framework documentation.

## Contents

| Document | Description |
|----------|-------------|
| [Architecture Guide](architecture.md) | Deep dive into MAPLE architecture |
| [Repository Structure](repo-structure.md) | Layered crate organization (`palm/*`, `resonator/*`, `mapleverse/*`) |
| [Getting Started](getting-started.md) | Build your first MAPLE app |
| [Platform Packs Tutorial](tutorials/platform-packs.md) | Creating custom platform packs |
| [Operations Tutorial](tutorials/operations.md) | Daemon, CLI, and Playground workflows |
| [Conformance Guide](conformance.md) | Testing platform pack compliance |
| [API Reference](api/README.md) | Complete API documentation |
| [UAL (Universal Agent Language)](concepts/ual.md) | Agent/human interaction language |
| [Agents, Resonators, and LLMs](concepts/agent-resonator-llm.md) | Mental model + backend integration |
| [Agent Kernel Composition](concepts/agent-kernel-composition.md) | Non-bypassable `Agent = Resonator + Profile + Capability + Contracts` runtime model |
| [Storage Layer](concepts/storage-layer.md) | Source-of-truth + AI-memory storage architecture |
| [Staged Rollout Checklist](staged-rollout-checklist.md) | Stage 1-5 implementation status, files, and release commands |

## Quick Links

- [Main README](../README.md) - Project overview and quick start
- [Contributing](../CONTRIBUTING.md) - How to contribute
- [Changelog](../CHANGELOG.md) - Version history

## Getting Started

1. Read the [Architecture Guide](architecture.md) to understand MAPLE's design
2. Follow the [Platform Packs Tutorial](tutorials/platform-packs.md) to create your first pack
3. Use the [Conformance Guide](conformance.md) to validate your implementation
4. Reference the [API docs](api/README.md) as needed
