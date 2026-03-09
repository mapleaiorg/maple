# Commitment Boundary

The commitment boundary is the kernel-owned gate between cognition and consequence. Models and operators can form intent, but they do not get to execute high-consequence actions directly. Every real effect has to cross this boundary.

## Why the boundary exists

Without a non-bypassable gate, agent systems drift into:

- direct side effects with weak attribution
- policy bypass under pressure
- replay gaps
- unsafe upgrades and self-modification

MAPLE fixes that by making irreversible action an explicit runtime event.

## Canonical flow

1. An operator proposes a commitment with intent, plan, requested capabilities, and evidence.
2. Guard or governance evaluates policy and risk.
3. The runtime issues a decision receipt.
4. Only accepted commitments can invoke consequence drivers.
5. The outcome is recorded as another receipt.

## Practical consequences

- Drivers should never be callable without a receipt.
- Rejections and holds are first-class outcomes.
- Evidence is referenced, not dumped into the ledger blindly.
- High-risk tiers can require stronger approvals and staged rollout.

## Boundary invariants

- Proposal is intent, not effect
- Receipts are immutable
- No driver without receipt
- Rejections must be explainable
- Pending approvals must still be ledgered

## Related reading

- [/docs/architecture/worldline-model](https://mapleai.org/docs/architecture/worldline-model)
- [/docs/reference/invariants](https://mapleai.org/docs/reference/invariants)
