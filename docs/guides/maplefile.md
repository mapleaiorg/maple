# Maplefile Reference

The Maplefile is the package contract for MAPLE artifacts. It describes what an agent is, which models it can use, which skills it may invoke, how memory is wired, and which policy constraints apply.

Important: the current parser expects snake_case YAML fields and kebab-case `kind` values.

## Core shape

```yaml
api_version: "maple.ai/v1"
kind: agent-package
name: "myorg/agents/support-agent"
version: "1.0.0"
description: "Support agent package"

metadata:
  authors:
    - "My Org"
  license: "MIT OR Apache-2.0"
  keywords:
    - "support"
  labels: {}

models:
  default:
    reference: "ollama:llama3.2:3b"
    min_context: 8192
    capabilities:
      - "tool-calling"
  alternatives:
    - reference: "openai:gpt-4o-mini"
      capabilities:
        - "tool-calling"
  constraints:
    data_classification: "internal"
    jurisdictions:
      - "CA"
    max_cost_per_1k_tokens: 0.01

skills:
  - reference: "myorg/skills/zendesk"
    version: "^1.0"
    optional: false
    provides:
      - "zendesk.ticket.read"

contracts:
  - reference: "myorg/contracts/support-safety"
    version: "^1.0"
    enforcement: mandatory

memory:
  worldline:
    mode: "event-ledger"
    backend: "sqlite"

policy:
  deny_by_default: true
  allow:
    - tool: "zendesk.ticket.read"
      requires_approval: false

observability:
  traces: "otel"
  replay: "enabled"
  metrics: "prometheus"
```

## Package kinds implemented in the manifest schema

The current `maple-package` crate implements these package kinds:

1. `agent-package`
2. `skill-package`
3. `contract-bundle`
4. `model-package`
5. `eval-suite`
6. `ui-module`
7. `knowledge-pack`
8. `policy-pack`
9. `evidence-pack`

## What "Docker-like" means here

MAPLE uses OCI-style packaging foundations for agents and related artifacts.

- The Maplefile is the package manifest.
- `maple-build` assembles deterministic layers and writes `maple.lock`.
- `maple-package-trust` signs and attests the result.
- `maple-registry-client` moves artifacts through OCI registries.

That is the MAPLE equivalent of build, sign, and push.

## Current command status

The underlying package pipeline is implemented, but the polished public commands below are not exposed in `maple-cli` yet:

```text
maple build
maple sign
maple sbom
maple push
maple mirror
maple import
```

Today those operations are library-first and live in:

- `maple-init`
- `maple-build`
- `maple-package-trust`
- `maple-registry-client`
- `maple-fleet-stack`

## Package layout by kind

At build time, `maple-build` adds kind-specific directories as OCI layers.

- Agent-like packages can contribute `prompts/`, `contracts/`, `memory/`, `eval/`, and `static/`
- Model packages can contribute `model/`, `tokenizer/`, `config/`, and provenance data

Create only the directories that matter for your package. Empty optional directories are not required.

## What belongs in the package

- Prompts and role contracts
- Skill references and declared capabilities
- Contract and policy references
- Model requirements and routing constraints
- Versioned metadata, runtime constraints, and eval baselines

## What does not belong in the package

- Live secrets
- Tenant-specific credentials
- Unbounded tool access
- Mutable runtime state

## Scaffolding today

The current scaffolding path is:

- use the `maple-init` crate from Rust, or
- create the expected directories manually

For an `agent-package`, the shipped scaffold shape is:

```text
prompts/
contracts/
eval/
```

## Best practices

- Prefer explicit package versions over floating tags.
- Keep contracts and prompts close to the Maplefile so package intent stays inspectable.
- Treat model bindings as policy-routed defaults, not permanent lock-in.
- Generate an SBOM for every artifact that crosses an environment boundary.
