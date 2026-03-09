# Architecture Overview

MAPLE is the enterprise control plane for agentic AI systems. It is not a chatbot toolkit and it is not just another agent library. It is the operating layer for software that can reason, call tools, move information, and create real-world consequences under governance.

The core thesis is simple: models will commoditize, but governed execution will not. Enterprises need packaging, model routing, capability controls, auditability, rollout control, and cost boundaries. MAPLE treats those as platform primitives.

## The layer cake

| Layer | MAPLE role | Analogy |
| --- | --- | --- |
| Reference agents | Opinionated agent packages for support, finance, compliance, and operations | Application layer |
| Fleet / Foundry / Guard | Rollout, evaluation, distillation, capability firewall, approvals, compliance packs | Kubernetes + policy engine |
| Packages / Registry / Models | OCI-style distribution, signed artifacts, model pull/serve/route | Docker + registry + Ollama |
| Kernel | Event fabric, commitment gate, worldline ledger, memory engine, operator bus | Agent runtime kernel |
| Foundation | Types, temporal model, cryptography, identity, proofs | System substrate |

## Five non-negotiable principles

### 1. Intelligence implies no authority

An LLM can reason about almost anything. That does not mean it is authorized to act. In MAPLE, model output is advisory until it becomes an explicit commitment proposal and crosses the gate successfully.

### 2. Commitment boundary

There is a hard line between cognition and consequence. Actions that matter do not happen because a model "decided" to do them. They happen because an explicit commitment was evaluated, approved, recorded, and then executed.

### 3. Deny by default

Agents start with zero capability. Every tool call, external API request, or high-risk operation has to be granted. This inverts the usual framework posture where everything is possible until someone remembers to restrict it.

### 4. Immutable provenance

MAPLE records receipts and outcomes in a WorldLine history. That makes replay, explanation, and auditor evidence generation part of the runtime model instead of a custom logging project.

### 5. Model neutrality

MAPLE treats models as backends behind policy and routing. You can prefer a local model for sensitive data, a hosted model for high-complexity tasks, or a benchmarked fallback without rewriting your package contract.

## The resonance ladder

MAPLE describes action as an ordered ladder:

1. Presence: an identity is active and observable.
2. Coupling: the identity is connected to another actor or context.
3. Meaning: the incoming signals become structured understanding.
4. Intent: that understanding becomes a goal with confidence.
5. Commitment: the actor explicitly declares the action it wants to take.
6. Consequence: the system executes the approved action and records the result.

The important property is that stages 1 through 4 can remain exploratory. The irreversible jump happens only at commitment.

## Agent formula

```text
Agent = Resonator + Profile + Capability + Contracts + WorldLine
```

- Resonator: the cognitive runtime that turns signals into plans.
- Profile: the deployment context that constrains what "safe" means.
- Capability: the explicit tool authority the agent can request.
- Contracts: Guard and policy rules that define what is permitted.
- WorldLine: the durable identity and provenance chain for the agent.

## Request lifecycle

For a request like "pay my credit card bill", the platform shape looks like this:

1. Gateway authenticates the tenant and opens a trace.
2. Control selects the right package from the registry.
3. Registry resolves agent, skill, and policy artifacts.
4. Guard validates the package against tenant constraints.
5. Runtime instantiates the worldline and binds secrets.
6. Model routing chooses the right inference backend.
7. Worker executes read-only calls under capability checks.
8. Guard evaluates risk and approval requirements.
9. Runtime submits the payment commitment.
10. The approved consequence executes.
11. Observer captures metrics and traces.
12. WorldLine stores receipts and outcomes for replay.

## Deployment topologies

### Local development

Single-process demo path with Rust binaries, SQLite-friendly storage, and optional Ollama. Best for learning the mental model.

### Team deployment

Compose-based deployment with MAPLE services, Postgres, Ollama, and basic observability. Best for shared staging and integration work.

### Enterprise

Helm-managed Kubernetes deployment with high availability, managed persistence, and rollout policy. Best when multiple teams or tenants share the control plane.

### Air-gapped

Mirror packages and models into a private environment, then import and run without live internet access. Best for sovereign or regulated estates.

## Related reading

- Deep dive into [/docs/architecture/worldline-model](https://mapleai.org/docs/architecture/worldline-model)
- Boundary design in [/docs/architecture/commitment-boundary](https://mapleai.org/docs/architecture/commitment-boundary)
- Runtime guarantees in [/docs/reference/invariants](https://mapleai.org/docs/reference/invariants)
