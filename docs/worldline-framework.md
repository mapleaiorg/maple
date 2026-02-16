# WorldLine Framework Guide

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
| 1 | Foundation types | `worldline-types` (compat: `maple-mwl-types`) |
| 2 | WorldLine identity | `worldline-identity` (compat: `maple-mwl-identity`) |
| 3 | Event fabric | `worldline-runtime::fabric` (compat: `maple-kernel-fabric`) |
| 4 | Commitment gate | `worldline-runtime::gate` (compat: `maple-kernel-gate`) |
| 5 | WorldLine ledger | `worldline-ledger` |
| 6 | Provenance index | `worldline-ledger::provenance` (compat: `maple-kernel-provenance`) |
| 7 | Memory engine | `worldline-runtime::memory` (compat: `maple-kernel-memory`) |
| 8 | Operator bus | `worldline-runtime::mrp` (compat: `maple-kernel-mrp`) |
| 9 | Governance engine | `worldline-governance` (compat: `maple-kernel-governance`) |
| 10 | Network adapters | `maple-kernel-sdk` |
| 11 | Protocol suite | `worldline-runtime::mrp` |
| 12 | Safety and human agency | `worldline-governance::safety` (compat: `maple-kernel-safety`) |
| 13 | Profile system | `worldline-governance::profiles` (compat: `maple-kernel-profiles`) |
| 14 | Financial extensions | `worldline-runtime::financial` (compat: `maple-kernel-financial`) |
| 15 | SDK, CLI, REST API | `maple-kernel-sdk`, `maple-cli`, `palm-daemon` |
| 16 | Kernel composition/bootstrap | `worldline-core`, `worldline-runtime`, `worldline-ledger` |
| 17 | Self-observation | `worldline-substrate::observation` (compat: `maple-worldline-observation`) |
| 18 | Meaning formation | `worldline-substrate::meaning` (compat: `maple-worldline-meaning`) |
| 19 | Intent stabilization | `worldline-substrate::intent` (compat: `maple-worldline-intent`) |
| 20 | Self-modification gate | `worldline-substrate::self_mod_gate` (compat: `maple-worldline-self-mod-gate`) |
| 21 | Code generation + sandbox | `worldline-substrate::codegen` (compat: `maple-worldline-codegen`) |
| 22 | Deployment + rollback | `worldline-substrate::deployment` (compat: `maple-worldline-deployment`) |
| 23 | Language generation operator | `worldline-substrate::langgen` (compat: `maple-worldline-langgen`) |
| 24 | WLIR | `worldline-substrate::ir` (compat: `maple-worldline-ir`) |
| 25 | Adaptive compiler | `worldline-substrate::compiler` (compat: `maple-worldline-compiler`) |
| 26 | SAL | `worldline-substrate::sal` (compat: `maple-worldline-sal`) |
| 27 | Hardware + bootstrap protocol | `worldline-substrate::hardware`, `worldline-substrate::bootstrap` |
| 28 | EVOS integration + conformance | `worldline-substrate::evos`, `worldline-substrate::conformance`, `maple-mwl-conformance` |

## User-Facing Interfaces

### Naming model (target)

- `worldline-runtime` (kernel)
- `worldline-ledger` (record + replay)
- `worldline-governance` (control plane)
- `worldline-operator-bot` (agentic operations)
- `worldline-promptkit` (prompt + tool contract bundles)

Compatibility:
- Existing `palm-*` and `maple-*` names remain in use in the current repository.
- See [Architecture Migration Plan](architecture/migration-plan.md).

### Compatibility Mapping (Legacy -> Canonical)

| Existing | Canonical |
|---|---|
| `maple-mwl-types` | `worldline-types` |
| `maple-mwl-identity` | `worldline-identity` |
| `worldline-types` + `worldline-identity` | `worldline-core` |
| `maple-runtime` + `maple-kernel-*` runtime subsystems | `worldline-runtime` |
| `maple-kernel-provenance` (+ fabric/types for lineage) | `worldline-ledger` |
| `maple-kernel-governance` (+ gate/safety/profiles) | `worldline-governance` |
| `maple-worldline-*` substrate crates | `worldline-substrate` |

Implementation note:
- Legacy crates stay available for compatibility.
- New integrations should import canonical `worldline-*` crates.

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

- `cargo test -p worldline-types -p worldline-identity -p worldline-core -p worldline-runtime -p worldline-ledger -p worldline-governance -p worldline-substrate`
- `cargo test -p maple-mwl-conformance -p maple-mwl-integration -p maple-worldline-conformance`
- `cargo test -p maple-worldline-observation -p maple-worldline-meaning -p maple-worldline-intent -p maple-worldline-commitment -p maple-worldline-consequence -p maple-worldline-self-mod-gate -p maple-worldline-codegen -p maple-worldline-deployment -p maple-worldline-langgen -p maple-worldline-ir -p maple-worldline-compiler -p maple-worldline-sal -p maple-worldline-hardware -p maple-worldline-bootstrap -p maple-worldline-evos`
- `cargo test -p maple-kernel-sdk -p maple-cli -p palm-daemon`

The `examples/mwl-*` demos are wired to the WorldLine crates so users can adopt
canonical naming without changing runtime behavior.

## Next Step

Use the hands-on tutorial: [WorldLine Quickstart](tutorials/worldline-quickstart.md).
