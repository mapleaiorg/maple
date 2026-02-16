# WorldLine Framework Guide (v1.2.1)

This repository now ships the full Maple WorldLine prompt stack (Prompts 1-28) as concrete crates, tests, and runnable demos.

## Conceptual split (EVOS-aligned)

The architecture keeps two roles distinct:

- WorldLine Kernel (data plane):
  - invariants
  - commitment boundary
  - ledger and replay
- WorldLine Ops/Governance (control plane):
  - lifecycle and deployment
  - policy/capability control
  - audits and rollback orchestration

Control-plane functions can be executed by humans or bots, but enforcement remains kernel-owned.

## Implementation Coverage

| Prompt | Design Area | Primary Crates |
|---|---|---|
| 1 | Foundation types | `maple-mwl-types` |
| 2 | WorldLine identity | `maple-mwl-identity` |
| 3 | Event fabric | `maple-kernel-fabric` |
| 4 | Commitment gate | `maple-kernel-gate` |
| 5 | WorldLine ledger | `maple-kernel-provenance`, `maple-kernel-gate` |
| 6 | Provenance index | `maple-kernel-provenance` |
| 7 | Memory engine | `maple-kernel-memory` |
| 8 | Operator bus | `maple-kernel-mrp` |
| 9 | Governance engine | `maple-kernel-governance` |
| 10 | Network adapters | `maple-kernel-sdk` |
| 11 | Protocol suite | `maple-kernel-mrp` |
| 12 | Safety and human agency | `maple-kernel-safety` |
| 13 | Profile system | `maple-kernel-profiles` |
| 14 | Financial extensions | `maple-kernel-financial` |
| 15 | SDK, CLI, REST API | `maple-kernel-sdk`, `maple-cli`, `palm-daemon` |
| 16 | Kernel composition/bootstrap | `maple-runtime`, `maple-kernel-*` |
| 17 | Self-observation | `maple-worldline-observation` |
| 18 | Meaning formation | `maple-worldline-meaning` |
| 19 | Intent stabilization | `maple-worldline-intent` |
| 20 | Self-modification gate | `maple-worldline-self-mod-gate` |
| 21 | Code generation + sandbox | `maple-worldline-codegen` |
| 22 | Deployment + rollback | `maple-worldline-deployment` |
| 23 | Language generation operator | `maple-worldline-langgen` |
| 24 | WLIR | `maple-worldline-ir` |
| 25 | Adaptive compiler | `maple-worldline-compiler` |
| 26 | SAL | `maple-worldline-sal` |
| 27 | Hardware + bootstrap protocol | `maple-worldline-hardware`, `maple-worldline-bootstrap` |
| 28 | EVOS integration + conformance | `maple-worldline-evos`, `maple-worldline-conformance`, `maple-mwl-conformance` |

## User-Facing Interfaces

### Naming model (target)

- `worldline-runtime` (kernel)
- `worldline-ledger` (record + replay)
- `worldline-governance` (control plane)
- `worldline-operator-bot` (agentic operations)
- `worldline-promptkit` (prompt + tool contract bundles)

Compatibility:
- Existing `palm-*` and `maple-*` names remain in use in the current repository.
- See migration phases in [Architecture Migration Plan](architecture/phase-plan.md).

### Phase A compatibility mapping (old -> facade)

| Existing | Facade (Phase A) |
|---|---|
| `maple-mwl-types` + `maple-mwl-identity` | `worldline-core` |
| `maple-runtime` + `maple-kernel-*` runtime subsystems | `worldline-runtime` |
| `maple-kernel-provenance` (+ fabric/types for lineage) | `worldline-ledger` |

Implementation note:
- `maple-mwl-types` and `maple-mwl-identity` remain foundational in Phase A.
- Higher-level crates (`maple-worldline-*`, `maple-mwl-conformance`, `maple-mwl-integration`)
  now consume `worldline-*` facades directly.

### CLI (maple)

The umbrella `maple` CLI exposes WorldLine commands directly:

- `maple worldline ...`
- `maple commit ...`
- `maple provenance ...`
- `maple financial ...`
- `maple policy ...`
- `maple kernel ...`

All commands use `--endpoint` (default `http://localhost:8080`) and support `PALM_ENDPOINT`.

### REST API (PALM daemon)

WorldLine routes are merged into PALM daemon under `/api/v1`:

- `POST /worldlines`, `GET /worldlines`, `GET /worldlines/:id`
- `POST /commitments`, `GET /commitments/:id`, `GET /commitments/:id/audit-trail`
- `GET /provenance/:event_id/ancestors`, `GET /provenance/worldline/:id/history`
- `POST /governance/policies`, `GET /governance/policies`, `POST /governance/simulate`
- `POST /financial/settle`, `GET /financial/:worldline_id/balance/:asset`
- `GET /kernel/status`, `GET /kernel/metrics`

## Verification Status

The following suites pass in this repository:

- `cargo test -p worldline-core -p worldline-runtime -p worldline-ledger`
- `cargo test -p maple-mwl-conformance -p maple-mwl-integration -p maple-worldline-conformance`
- `cargo test -p maple-worldline-observation -p maple-worldline-meaning -p maple-worldline-intent -p maple-worldline-commitment -p maple-worldline-consequence -p maple-worldline-self-mod-gate -p maple-worldline-codegen -p maple-worldline-deployment -p maple-worldline-langgen -p maple-worldline-ir -p maple-worldline-compiler -p maple-worldline-sal -p maple-worldline-hardware -p maple-worldline-bootstrap -p maple-worldline-evos`
- `cargo test -p maple-kernel-sdk -p maple-cli -p palm-daemon`

The `examples/mwl-*` demos are wired to the facade crates so users can adopt
Phase A naming without changing runtime behavior.

## Next Step

Use the hands-on tutorial: [WorldLine Quickstart](tutorials/worldline-quickstart.md).
