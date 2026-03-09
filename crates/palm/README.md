# PALM Crates

PALM is the current daemon and operational control-plane layer in the MAPLE repository.

## What PALM Does

- daemon lifecycle and HTTP surfaces
- deployment and instance coordination
- health, state, and operational visibility
- policy and registry operations
- compatibility path for the broader Fleet and control-plane story

## Current Crates

- `types`
- `registry`
- `deployment`
- `health`
- `state`
- `control`
- `policy`
- `shared-state`
- `daemon`
- `observability`
- `cli`

## How to Use It

- start with [docs/tutorials/operations.md](../../docs/tutorials/operations.md)
- use [docs/api/cli-reference.md](../../docs/api/cli-reference.md) for command groups
- use [docs/api/rest-api.md](../../docs/api/rest-api.md) for endpoint orientation
