# Mapleverse Crates

`crates/mapleverse/*` contains execution and world-facing service crates from the earlier profile-specific architecture.

## Current Role

These crates remain useful as execution-layer and connector-oriented implementation surfaces. In the newer Agent OS framing, that work maps mostly into:

- Runtime for execution semantics
- Guard for governed consequence
- Fleet for operational rollout
- reference agents for domain packages

## Components

- `types`
- `executor`
- `connectors`
- `evidence`
- `service`
- `world`

## Recommended Reading

- [Architecture Overview](../../docs/architecture/overview.md)
- [Commitment Boundary](../../docs/architecture/commitment-boundary.md)
- [Fleet Deployment Guide](../../docs/guides/fleet-deployment.md)
