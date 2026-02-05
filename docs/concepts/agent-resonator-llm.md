# AI Agents, Resonators, and LLM Backends

This note answers a common MAPLE architecture question: what is the relationship between an **AI agent**, a **Resonator**, and an **LLM backend**?

## 1) Agent vs Resonator

- In MAPLE, a **Resonator** is the persistent identity and lifecycle unit.
- An **AI agent** is typically the behavior/process running as (or within) a Resonator identity.
- PALM orchestrates fleets of agent instances (`AgentSpec`/`Deployment`/`AgentInstance`), while Resonator runtime owns continuity, presence, coupling, and attention.

So in practice:

`Agent behavior` + `Resonator identity/state` + `PALM orchestration` = deployable MAPLE agent system.

The runtime now exposes this explicitly via `AgentKernel` in `maple-runtime`:

`Agent = Resonator + Profile + CapabilitySet + ContractSet + State`

Related core docs:

- `docs/core/agents.md`
- `docs/core/interop.md`
- `docs/core/llama-first.md`

## 2) Where LLMs fit (Trait-Level)

LLMs (Ollama, GPT, Claude, Grok, Gemini) are **cognition engines**, not identity.

- Resonator: identity, accountability, coupling, continuity.
- LLM backend: text generation/reasoning service used by the Resonator/agent.
- Commitments and governance still flow through MAPLE controls (UAL → RCF validation, policy/adjudication, evidence/audit).

In code:

- Cognition contract: `ModelAdapter`
- Tool surface: `CapabilityExecutor`
- Consequence boundary: `CommitmentGateway`
- Normalized call candidate: `CapabilityCallCandidate`
- Contract draft object: `ContractDraft`

## 3) Supported Playground Backends

- `local_llama` (default, Ollama-style endpoint)
- `open_ai`
- `anthropic`
- `grok` (xAI OpenAI-compatible chat API)
- `gemini` (Google Generative Language API)

In `maple-runtime`, all backends now share a normalized adapter contract:

- `ModelProviderConfig` (backend/model/endpoint/auth env/timeout)
- `ModelUsage` (prompt/completion/total token accounting envelope)
- `ModelAdapterError` with normalized `ModelErrorKind`

Provider modules live under `crates/maple-runtime/src/cognition/`:
`llama`, `openai`, `anthropic`, `gemini`, `grok`.

All adapters share the same hardening path:

1. strict schema parse
2. deterministic repair pass
3. deterministic fallback with **no tool suggestion**

This guarantees malformed model output never bypasses runtime gates.

`ModelAdapter` methods used by runtime:

- `propose_meaning(...)`
- `propose_intent(...)`
- `draft_contract(...)`
- `suggest_capability_calls(...)`
- `summarize(...)`

You can inspect and switch from CLI:

```bash
maple palm playground backends
maple palm playground set-backend --kind local_llama --model llama3 --endpoint http://127.0.0.1:11434
maple palm playground set-backend --kind open_ai --model gpt-4o-mini --api-key YOUR_KEY
maple palm playground set-backend --kind anthropic --model claude-3-5-sonnet --api-key YOUR_KEY
maple palm playground set-backend --kind grok --model grok-2-latest --api-key YOUR_KEY
maple palm playground set-backend --kind gemini --model gemini-2.0-flash --api-key YOUR_KEY
```

## 4) One-shot Backend Inference

Playground exposes a backend inference API and CLI command:

```bash
maple palm playground infer "Summarize current resonator activity"
maple palm playground infer "Draft a UAL commit statement" --system-prompt "You are a MAPLE operator assistant"
```

This is useful for operator workflows and backend smoke-testing. Inference activity is recorded in Playground activity history for visibility.

Playground simulation runs **auto-inference mode by default**, where simulated agents periodically invoke the active backend and emit richer `agent_cognition` traces.

## 5) Governance Path for LLM-Generated Intent

Recommended flow:

1. LLM proposes intent in **UAL**.
2. `ual-compiler` compiles to RCF commitments / PALM ops.
3. `rcf-validator` validates commitment constraints.
4. AAS policy/adjudication gates execution.
5. EVE and audit trails record outcomes.

This keeps LLMs in proposal/execution-assist mode while MAPLE preserves authority, accountability, and traceability.

Execution authority always remains at the boundary:

`ModelAdapter -> Contract Draft -> CommitmentGateway -> CapabilityExecutor -> Ledger Receipt`.
