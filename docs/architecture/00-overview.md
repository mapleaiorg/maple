# 00 - Overview (MAPLE as Digital EVOS)

MAPLE is a digital EVOS implementation: persistent WorldLines that evolve through
a resonance-ordered cognition pipeline and cross a strict Commitment Boundary before
producing ledgered consequences.

The system is designed for:
- continuity (identity persists across time and restarts),
- auditability (every irreversible change is ledgered),
- non-bypassable safety (commitment gating cannot be skipped),
- replayability (worldlines can be reconstructed from receipts + snapshots),
- agentic governance (ops is performed by bots, but enforcement is kernel-owned).

> Core rule: No commitment, no consequence.

## 0.1 Vocabulary (canonical)

WorldLine (WL)
- A persistent computational identity that maintains continuity, memory, and evolution.

Resonance Pipeline
- An ordered cognition/decision pipeline that produces intent (not effects):
  `presence -> coupling -> meaning -> intention -> commitment-proposal`

Commitment Boundary
- A kernel-owned gate that is the only path to irreversible side effects.

WorldLine Ledger (WLL)
- An append-only ledger of commitments, receipts, outcomes, and provenance.

Governance / Ops
- Policies, capabilities, approvals, rollout control. Implemented by agents (bots), but
  enforced by the kernel at the commitment boundary.

## 0.2 System map

```text
         +-----------------------------------------------------------+
         |                  WorldLine Kernel (Data Plane)            |
         |                                                           |
Inputs ->|  Resonance Pipeline -> Commitment Boundary -> Consequences|-> External World
         |        (intent)            (gate)            (effects)    |
         |                                                           |
         |            +----------- WorldLine Ledger -----------+     |
         |            | commitments, receipts, outcomes, proofs|     |
         |            +-----------------------------------------+     |
         +-----------------------------------------------------------+
                           ^                        ^
                           |                        |
                           |                        |
                 +---------+---------+    +---------+----------+
                 | Governance / Ops  |    | Projections/Index  |
                 | (agentic control) |    | search/analytics   |
                 +-------------------+    +--------------------+
```

Governance can be human-driven or bot-driven. Either way it only proposes
commitments; the kernel decides and executes.

## 0.3 Naming (target model)

Kernel:
- `worldline-types`
- `worldline-identity`
- `worldline-core`
- `worldline-runtime`
- `worldline-ledger`

Governance / Ops:
- `worldline-governance`
- `worldline-substrate`
- `worldline-operator-bot` (optional)
- `worldline-promptkit` (optional)

Umbrella:
- `maple-runtime` (compatibility re-export; stable public face)

Compatibility note:
- In the current repository, PALM crates continue to provide control-plane behavior.
- This naming model can be adopted incrementally while keeping `palm-*`
  compatibility for one release cycle.

## 0.4 Runtime invariants (non-negotiable)

These are enforced by `worldline-runtime` (kernel-owned):

I.1 - WorldLine Primacy
- Important entities are trajectories (WorldLines), not sessions.
- Identity is continuity of commitments + provenance.

I.2 - Intrinsic Typed Memory
- Memory is typed and lifecycle-aware: working, episodic, semantic, parametric.

I.3 - Commitment Boundary
- No external consequence without explicit commitment, policy check, and provenance.

I.4 - Causal Provenance
- Commitments and consequences must be attributable to persistent identity + continuity DAG.

I.5 - Resonance-Bounded Coupling
- Coupling is bounded by available attention; no unbounded resonance.

I.6 - Pluggable Evolution Laws
- Evolution operators are swappable, but swaps are commitment-gated and provable.

I.7 - Safety Overrides Optimization
- Safety and agency constraints override task optimization and performance goals.

I.8 - Substrate Independence
- The architecture remains valid across digital, hybrid, distributed, and sovereign substrates.

I.9 - Implementation Provenance & Constitutional Evolution
- Operator upgrades require an upgrade commitment, replay verification, and evidence anchoring.

## 0.5 What this enables

- Deterministic audit: answer who/why/what/when for any change.
- Safe autonomy: operator bots can run ops loops but cannot bypass policy.
- Evolution without chaos: upgrades, migrations, and self-modification become explicit
  commitment classes with stricter policy tiers.

## 0.6 Read next

- [01 - WorldLine Model](01-worldline.md)
- [02 - Crate and Component Layout](02-crate-layout.md)
- [03 - Commitment Boundary](03-commitment-boundary.md)
- [04 - WorldLine Ledger (WLL)](04-ledger-wll.md)
