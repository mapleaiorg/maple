# MAPLE Agents

This page defines the runtime composition model:

`Agent = Resonator + Profile + Capability + Contracts`

In concrete runtime terms:

`Agent = Resonator + Profile + CapabilitySet + ContractSet + State`

The implementation lives in `crates/maple-runtime/src/agent_kernel/mod.rs`.

## Composition Model

An Agent in MAPLE is not a separate universe from Resonators.

- `Resonator`: persistent identity, lifecycle, coupling, continuity.
- `Profile`: autonomy/risk/governance envelope.
- `CapabilitySet`: declared actions that may be requested.
- `ContractSet`: explicit commitments that authorize consequence.
- `State`: attention budget, coupling graph, short memory, journal, policy/capability/ledger handles.

## Commitment Gateway (Non-bypassable)

All capability execution flows through `CommitmentGateway`.

- Consequential capabilities require an explicit commitment.
- Non-consequential capabilities are still commitment-bound through runtime-generated contracts.
- Policy + capability checks run before execution.
- Execution receipts are persisted to ledger for replay.

If any gate fails, execution returns explicit errors (for example `ContractMissing`, `PolicyDenied`, `CapabilityDenied`, `InvariantViolation`).

## Runtime Flow Diagram

```text
+-------------------+      +-----------------------+      +----------------------+
| ModelAdapter      | ---> | Meaning/Intent Draft  | ---> | ContractDraft (RCF)  |
| (llama/openai/...)|      | (normalized objects)  |      | + capability binding |
+-------------------+      +-----------------------+      +----------------------+
                                                                  |
                                                                  v
                                                       +----------------------+
                                                       | CommitmentGateway    |
                                                       | policy + capability  |
                                                       | + invariant checks   |
                                                       +----------------------+
                                                                  |
                                                                  v
                                                       +----------------------+
                                                       | CapabilityExecutor   |
                                                       | (tool/rail adapter)  |
                                                       +----------------------+
                                                                  |
                                                                  v
                                                       +----------------------+
                                                       | AAS Ledger Receipts  |
                                                       | outcome + replay     |
                                                       +----------------------+
```

## Vendor Stack Integration (Without Bypass)

Vendor stacks can integrate, but they cannot bypass MAPLE semantics.

- LLM vendor APIs integrate via `ModelAdapter`.
- Tool/vendor SDK calls integrate via `CapabilityExecutor` behind the gateway.
- External protocols (MCP/A2A) map into MAPLE events/capabilities, then pass gateway checks.

Key rule:

`No commitment -> no consequence`.

Any attempt to execute direct side effects outside the gateway is a runtime boundary violation.

## Observability And Operator Commands

Daemon/API aliases are intentionally simple:

- `GET /api/v1/agent/status`
- `POST /api/v1/agent/handle`
- `GET /api/v1/agent/commitments`
- `GET /api/v1/agent/commitments/:id`
- `GET /api/v1/agent/commitments/:id/receipts`

CLI operators can inspect lifecycle and receipts directly:

- `maple agent commitments --limit 20`
- `maple agent contract --id <commitment_id>`

The event stream (`maple events watch`) now includes `AgentStageTransition`
and `AgentReceiptRecorded` events so stage flow and accountability writes are visible live.

## Security And Runtime Posture (MVP)

- Receipt hashes are deterministic over canonical receipt material.
- Real external tools are explicitly separated from simulation tools.
- Real tool execution is disabled by default and requires explicit opt-in (`MAPLE_ALLOW_REAL_TOOLS=true`).
- Oversized payloads are compacted in audit paths via hash references instead of full inline blobs.
