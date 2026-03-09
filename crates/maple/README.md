# MAPLE Shared Crates

This folder contains shared MapleAI Agent OS crates that support packaging, storage, models, protocols, and cross-cutting runtime services.

## What Lives Here

- `storage/`: storage contracts and adapters
- `model-*`: model adapters, routing, serving, benchmarking
- `protocol-*`: compatibility layers such as MCP and A2A mappings
- shared utility crates used across runtime, guard, and control-plane surfaces

## Role in the Agent OS

These crates are not the kernel themselves. They are the shared substrate that lets higher-level surfaces behave consistently:

- package and registry flows can persist and inspect artifacts
- runtime and daemon layers can call model backends uniformly
- compatibility protocols can map external ecosystems into MAPLE capability and commitment semantics

## Start with

- [Root README](../../README.md)
- [Docs Index](../../docs/README.md)
- [Model Management Guide](../../docs/guides/model-management.md)
