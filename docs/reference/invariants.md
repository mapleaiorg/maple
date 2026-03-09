# Architectural Invariants

MAPLE uses runtime-enforced invariants instead of asking developers to "remember the rules." These are properties the system should refuse to violate.

## The eight core invariants

1. Presence precedes meaning
2. Meaning precedes intent
3. Intent precedes commitment
4. Commitment precedes consequence
5. Coupling is bounded by attention
6. Safety overrides optimization
7. Human agency cannot be bypassed
8. Failure must be explicit

## Why runtime enforcement matters

Policy-only safety is too easy to bypass and too inconsistent across teams. MAPLE pushes these guarantees into the runtime so unsafe execution becomes a system error rather than a documentation violation.

## How to use this page

- Use invariants 1 through 4 to reason about cognitive pipeline integrity.
- Use invariant 5 to reason about bounded resource use and graceful degradation.
- Use invariants 6 and 7 to reason about safety and human protection.
- Use invariant 8 to reason about observability and incident response.

## Related reading

- [/docs/architecture/overview](https://mapleai.org/docs/architecture/overview)
- [/docs/reference/profiles](https://mapleai.org/docs/reference/profiles)
- [/trust](https://mapleai.org/trust)
