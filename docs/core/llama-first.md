# Llama-First In MAPLE

MAPLE is Llama-first by default for local cognition, while preserving provider-agnostic contracts.

## What MAPLE Implements For Llama

`LlamaAdapter` is implemented in `crates/maple-runtime/src/cognition/llama.rs`.

It supports:

- strict JSON cognition parsing
- deterministic repair pass for common malformed outputs
- deterministic fallback when parsing remains invalid
- normalized draft objects used by runtime gates

Runtime rule:

- fallback outputs never trigger tool execution
- ambiguous/invalid tool specs do not execute

## ModelAdapter Contract

All providers implement the same `ModelAdapter` trait:

- `propose_meaning(...)`
- `propose_intent(...)`
- `draft_contract(...)`
- `suggest_capability_calls(...)`
- `summarize(...)`

This keeps Llama/OpenAI/Anthropic/Gemini/Grok semantics aligned at the contract boundary.

## Memory + Replay + Coordination

Llama cognition is integrated with MAPLE runtime memory and accountability surfaces.

- short memory: immediate cognition state fields (meaning/intent context)
- journal summaries: stage transitions + capability/accountability events
- receipt replay: ledger receipts tied to commitment ids
- coordination safety: policy/capability/contract checks before consequence

## Gating Model (Always On)

Llama outputs are advisory cognition, not authority.

- authority comes from commitment + profile + policy + capability checks
- execution happens only through `CommitmentGateway`
- outcomes and failures are persisted for audit/replay

So Llama can propose, summarize, and draft — MAPLE decides and enforces.
