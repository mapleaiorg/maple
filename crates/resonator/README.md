# Resonator Crates

The `resonator-*` crates capture much of the earlier cognition and lifecycle implementation work that now feeds the broader Agent OS and WorldLine model.

## What They Cover

- identity and presence
- coupling, meaning, and intent
- commitments and consequences
- memory and conversation
- profiles, observability, conformance, and runtime coordination

## How to Read This Family Today

Use these crates as implementation-oriented source material for the cognition side of MAPLE. For the current top-level product and docs narrative, prefer:

- [Architecture Overview](../../docs/architecture/overview.md)
- [WorldLine Model](../../docs/architecture/worldline-model.md)
- [Architectural Invariants](../../docs/reference/invariants.md)

## Practical Entry Points

- `maple-runtime` for the runtime-facing SDK
- `resonator/profiles` for profile constraints
- `resonator/conformance` for invariant and flow verification
