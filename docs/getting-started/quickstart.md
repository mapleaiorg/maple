# 5-Minute Quickstart

This is the fastest path from zero to seeing MAPLE act like an Agent OS instead of a prompt wrapper. The demo boots the kernel, creates worldlines, sends an action through commitment gating, records provenance, and shows that the runtime treats consequence as a governed event.

## Prerequisites

- Rust 1.80 or newer
- Ollama recommended, but optional

## Run the demo

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple

# Optional local model runtime
ollama serve &
ollama pull llama3.2

# Run the MAPLE demo
cargo run -p maple-demo
```

## What you are seeing

1. Kernel boot: Event Fabric, Commitment Gate, and WorldLine ledger services initialize.
2. Worldline creation: the demo registers example identities so the runtime has durable actors instead of ephemeral chat sessions.
3. Coupling: one worldline establishes a relationship with another so signals can become meaningful interaction rather than raw transport.
4. Meaning and intent: the runtime forms an interpretable plan before any side effect is allowed.
5. Commitment gate: the proposed action is evaluated through policy, capability, and invariant checks.
6. Consequence: only an authorized action crosses into execution.
7. Provenance: receipts and outcomes are written so the entire sequence can be replayed or audited later.
8. Invariants: the demo closes by validating the architectural guarantees that prevent silent unsafe execution.

## If Ollama is not running

MAPLE should still let you explore the non-model parts of the flow: worldlines, commitments, receipts, and operator surfaces. That is intentional. The runtime is built so governance and provenance stay visible even when a local model backend is unavailable.

## Next steps

- Install toolchains cleanly in [/docs/getting-started/installation](https://mapleai.org/docs/getting-started/installation)
- Build a TODO agent in [/docs/getting-started/first-agent](https://mapleai.org/docs/getting-started/first-agent)
- Inspect the boundary design in [/docs/architecture/commitment-boundary](https://mapleai.org/docs/architecture/commitment-boundary)
