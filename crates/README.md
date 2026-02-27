# Crate Layout

The workspace is organized by architectural layer.

## Layered Folders

### Constitutional and Protocol

- `rcf/`: Resonance Constitutional Framework
- `ual/`: Universal Agent Language
- `mrp/`: routing and transport

### Authority and Learning

- `aas/`: accountability and capability governance
- `eve/`: evidence and validation

### Multi-Agent Coordination

- `collective/`: organization primitives
- `workflow/`: receipt-gated workflow engine

### WorldLine Framework

- `worldline/`: canonical WorldLine crates
- `kernel/`: runtime kernel subsystems
- `substrate/`: self-producing substrate
- `mwl/`: compatibility wrappers
- `waf/`: autopoietic factory components

### Cognition and Execution

- `resonator/`: cognition/memory/conformance layer
- `mapleverse/`: world execution layer

### Operations and Shared Services

- `palm/`: daemon, CLI, policy, lifecycle ops
- `maple/`: shared services (storage, model adapters, protocols)

## Flat Entry Points

- `maple-runtime`: runtime SDK (supports `--no-default-features` standalone mode)
- `maple-cli`: umbrella CLI
- `maple-integration`: integration facade

## Notes

- Package names remain stable for compatibility (`worldline-*`, `palm-*`, `maple-kernel-*`, `rcf-*`, etc.).
- `maple-runtime` now supports feature-gated dependency tiers:
  - core-only: `default-features = false`
  - full stack: enable `cognition`, `agent-kernel`, `profile-validation`
