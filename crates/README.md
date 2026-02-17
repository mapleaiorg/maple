# Crate Layout

The workspace groups major architecture layers into explicit folders.

## Layered folders

- `palm/`: operational control-plane and tooling crates.
- `resonator/`: cognition and lifecycle crates.
- `mapleverse/`: world execution and integration crates.
- `maple/`: cross-runtime MAPLE shared services (e.g. storage layer).
- `worldline/`: canonical WorldLine entrypoint crates:
  - `types`, `identity`, `core`, `runtime`, `ledger`, `governance`, `substrate`, `conformance`, `integration`

## Flat folders

Some cross-cutting layers remain flat because they are shared protocol/foundation domains:

- `rcf-*`, `ual-*`, `mrp-*`, `aas-*`, `eve-*`, `workflow-*`, `collective-*`
- `maple-runtime`, `maple-cli`, `maple-integration`
- `maple-mwl-*` compatibility wrappers (`types`, `identity`, `integration`, `conformance`)
- `maple-kernel-*`, `maple-worldline-*` legacy implementation set
- `worldline-*` names are the canonical integration surface
- legacy `maple-*` crates remain available as compatibility layers

This keeps package names stable while improving navigation and modular deployment boundaries.
