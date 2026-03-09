# WorldLine Model

WorldLine is MAPLE's kernel concept for durable identity. Agents, services, operator bots, and institutions are all modeled as worldlines with continuity, state, memory, operator logic, and ledger bindings.

## Core tuple

```text
WL(t) = { ID, Sigma(t), M(t), Theta(t), Pi, Lambda }
```

- `ID`: identity plus continuity keys
- `Sigma(t)`: observable and latent state
- `M(t)`: working, episodic, semantic, and parametric memory
- `Theta(t)`: temporal anchors and event cursor
- `Pi`: operator and pipeline configuration
- `Lambda`: ledger bindings and snapshot policy

## Why it matters

Worldlines are not just IDs for a request. They give MAPLE a way to explain:

- who acted
- which operator version was active
- which memory state influenced the action
- which receipts define continuity

That is why replay and audit work at the runtime level instead of being reconstructed from disconnected application logs.

## Lifecycle

1. Input arrives
2. Presence and coupling are established
3. Meaning and intent are formed
4. A commitment may be proposed
5. Authorized consequences execute
6. Outcome receipts extend the worldline history

## Worldline invariants

- Continuity must validate against the ledger head
- Ordered cognition must remain intact
- Canonical state changes need receipts
- Operator version is part of provenance
- Replay must converge given identical inputs and receipts

## Related reading

- [/docs/architecture/commitment-boundary](https://mapleai.org/docs/architecture/commitment-boundary)
- [/docs/reference/invariants](https://mapleai.org/docs/reference/invariants)
