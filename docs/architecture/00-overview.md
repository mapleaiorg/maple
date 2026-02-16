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
- `worldline-core`
- `worldline-runtime`
- `worldline-ledger`

Governance / Ops:
- `worldline-governance`
- `worldline-operator-bot` (optional)
- `worldline-promptkit` (optional)

Umbrella:
- `maple-runtime` (compatibility re-export; stable public face)

Compatibility note:
- In the current repository, PALM crates continue to provide control-plane behavior.
- This naming model can be adopted as a phased migration while keeping `palm-*`
  compatibility for one release cycle.

## 0.4 Runtime invariants (non-negotiable)

These are enforced by `worldline-runtime` (kernel-owned):

I1 - Ordering (Resonance)
- WorldLine cognition progresses in order:
  `presence -> coupling -> meaning -> intention -> proposal`.
- Stage skips require explicit policy and ledger annotations.

I2 - Commitment gating
- All irreversible actions require a commitment receipt.
- No direct side effects from cognition stages.

I3 - Non-bypassability
- Exactly one execution path to consequences: the commitment boundary.

I4 - Append-only receipts
- Commitments and outcomes are immutable and append-only.

I5 - Monotonic time
- WorldLine event time is monotonic per worldline.
- Replays must preserve order.

I6 - Capability + policy enforcement
- Every commitment must carry capability proofs/grants and pass governance policy.

I7 - Replayable evolution
- A WorldLine can be reconstructed from snapshot + receipts and validated.

I8 - Provenance completeness
- Every consequence must be attributable to a commitment and evidence bundle.

## 0.5 What this enables

- Deterministic audit: answer who/why/what/when for any change.
- Safe autonomy: operator bots can run ops loops but cannot bypass policy.
- Evolution without chaos: upgrades, migrations, and self-modification become explicit
  commitment classes with stricter policy tiers.

## 0.6 Read next

- [01 - WorldLine Model](01-worldline.md)
- [03 - Commitment Boundary](03-commitment-boundary.md)
- [04 - WorldLine Ledger (WLL)](04-ledger-wll.md)
