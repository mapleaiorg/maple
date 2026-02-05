# MAPLE Interop Boundaries

This page maps common ecosystem protocols into MAPLE runtime primitives.

## Interop Mapping

| External Surface | MAPLE Mapping | Enforcement Boundary |
|---|---|---|
| MCP Tool | `CapabilityDescriptor` + `CapabilityExecutor` | `CommitmentGateway` |
| A2A Message | Presence/coupling signal + meaning/intent input | Invariant guard + policy |
| Vendor Agent SDK Call | Capability proxy invocation | `CommitmentGateway` + ledger |
| Vendor LLM API | `ModelAdapter` cognition call | Structured parse/repair/fallback |
| Ops/Event Stream | `AgentStageTransition`, `AgentReceiptRecorded` | Live accountability visibility |

## MCP: Tools -> Capabilities

MCP tools are represented as MAPLE capabilities.

- Tool schema maps to capability id + argument JSON.
- Tool handler maps to `CapabilityExecutor`.
- Execution must include commitment context and pass policy/capability checks.

Result:

MCP transport can invoke tools, but consequence is still gated by MAPLE contracts.

## A2A: Messages -> Presence/Coupling Events

A2A messages are transport-level envelopes. In MAPLE they become semantic/runtime events.

- Session join/hello maps to presence establishment.
- Routing/relationship messages map to coupling updates.
- Intent-like messages become meaning/intent candidates, not direct consequences.

Result:

A2A can carry intent, but it cannot directly trigger side effects without commitment.

## Vendor Agent SDKs -> Capability Proxy

Vendor agent SDK actions (tool calls, chain calls, payment calls) integrate through a capability proxy layer.

- SDK request maps to one capability invocation.
- MAPLE checks commitment existence, profile/policy constraints, and capability authorization.
- MAPLE records outcome + receipt in AAS ledger.

Result:

Vendor SDK orchestration remains compatible, while MAPLE retains accountability and replay.

## What Is Explicitly Not Allowed

- Direct tool execution path bypassing `CommitmentGateway`.
- Network/transaction side effects without commitment reference.
- Silent failures (all failures must be explicit and persisted).
