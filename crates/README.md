# MAPLE Crates

The workspace is organized around the MapleAI Agent OS layers. Some crates still reflect earlier naming eras, but the current shape is converging on a clearer product-oriented stack.

## Core Product-Oriented Families

### Runtime and identity

- `worldline-*`
- `maple-runtime`
- `maple-kernel-*`

These crates define identity, continuity, event flow, commitment gating, memory, provenance, and runtime execution.

### Packaging and supply chain

- `maple-package`
- `maple-package-format`
- `maple-build`
- `maple-init`
- `maple-package-trust`
- `maple-registry-*`

These crates define the Maplefile contract, artifact assembly, signing, verification, registry distribution, and mirroring.

### Model management

- `maple-model-*`

These crates define backend adapters, routing, serving, and benchmarking.

### Governance

- `maple-guard-*`
- `worldline-governance`
- `worldline-operator-bot`
- `worldline-promptkit`

These crates define capability controls, approvals, redaction, compliance, and governance coordination.

### Improvement and rollout

- `maple-foundry-*`
- `maple-fleet-*`
- `palm-*`

These crates define traces, evaluation, training loops, rollout control, daemon operations, and operational visibility.

## Compatibility Families

The repository still contains compatibility namespaces and older architectural groupings. They remain useful for source compatibility and implementation history:

- `resonator-*`
- `rcf-*`
- `ual-*`
- `mrp-*`
- `aas-*`
- `mapleverse-*`
- `eve-*`
- `workflow-*`

## How to Navigate

- Start with [README.md](../README.md) for the product and docs map
- Use [docs/architecture/overview.md](../docs/architecture/overview.md) for the system model
- Use `cargo metadata --no-deps --format-version 1` if you need a machine-readable workspace map
