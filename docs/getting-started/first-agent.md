# Author Your First Agent Package

This tutorial authors a small TODO agent package contract. The point is not sophisticated task management. The point is to model an agent the way MAPLE models it now: explicit package identity, model requirements, deny-by-default tool authority, and durable runtime expectations.

## 1. Scaffold the package directory

The repo already ships the `maple-init` crate, but there is not yet a top-level `maple init` command in `maple-cli`. For now, create the expected directories manually:

```bash
mkdir -p my-todo-agent/prompts my-todo-agent/contracts my-todo-agent/eval
cd my-todo-agent
```

Expected shape:

```text
my-todo-agent/
├── Maplefile.yaml
├── prompts/
├── contracts/
└── eval/
```

## 2. Write `Maplefile.yaml`

The current parser expects snake_case field names and kebab-case `kind` values.

```yaml
api_version: "maple.ai/v1"
kind: agent-package
name: "myorg/agents/my-todo-agent"
version: "0.1.0"
description: "TODO agent package contract"

metadata:
  authors:
    - "myorg"
  license: "MIT OR Apache-2.0"
  keywords:
    - "todo"
    - "agent"
  labels: {}

models:
  default:
    reference: "ollama:llama3.2:3b"
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
    max_cost_per_1k_tokens: 0.02

skills:
  - reference: "myorg/skills/todo"
    version: "^0.1.0"
    optional: false
    provides:
      - "todo.add"
      - "todo.list"
      - "todo.complete"
      - "todo.delete"

contracts:
  - reference: "myorg/contracts/todo-safety"
    version: "^0.1.0"
    enforcement: mandatory

memory:
  worldline:
    mode: "event-ledger"
    backend: "sqlite"

policy:
  deny_by_default: true
  allow:
    - tool: "todo.add"
      requires_approval: false
    - tool: "todo.list"
      requires_approval: false
    - tool: "todo.complete"
      requires_approval: false
    - tool: "todo.delete"
      requires_approval: true

observability:
  traces: "otel"
  replay: "enabled"
  metrics: "prometheus"
```

The key idea is that the package declares cognition, memory, contracts, and capability boundaries together. MAPLE does not treat tool access as an afterthought bolted onto a chat loop.

## 3. Write the system prompt

`prompts/system.md`

```md
You are a TODO list assistant.
You help users add, list, complete, and delete tasks.
Use tools only when they are explicitly allowed by policy.
Always request confirmation before deleting tasks.
```

## 4. Decide what belongs in `contracts/` and `eval/`

- `contracts/` is where you place the policy, contract, or compliance material your package depends on.
- `eval/` is where you keep evaluation vectors and regression material.

The exact file formats inside those directories depend on the contract and eval tooling you adopt in your environment. The important point is that MAPLE packages reserve explicit space for them instead of hiding them in app-specific conventions.

## 5. Understand the build path

Today the package pipeline is library-first:

- `maple-package` parses and validates the manifest
- `maple-build` resolves dependencies and assembles OCI layers
- `maple-package-trust` signs, verifies, and generates SBOMs
- `maple-registry-client` pushes, pulls, and mirrors artifacts

There is not yet a polished `maple build` or `maple push` command in `maple-cli`. The package contract you authored here is the input those crates are designed to consume.

## 6. Connect the package design to the runtime

The current runtime CLI does not execute package artifacts directly yet, but you can exercise the daemon and worldline surfaces the package is targeting:

```bash
# Terminal 1
cargo run -p maple-cli -- daemon start --foreground

# Terminal 2
cargo run -p maple-cli -- worldline create --profile agent --label my-todo-agent
cargo run -p maple-cli -- kernel status
cargo run -p maple-cli -- agent demo --prompt "summarize current operator state"
```

Those commands show the governed runtime environment your package contract is meant to live inside: durable identities, explicit runtime status, and consequence-aware execution surfaces.

## Where to go next

- Deepen the package story in [Maplefile Reference](../guides/maplefile.md)
- Add model routing and backend control in [Model Management](../guides/model-management.md)
- Review Guard workflows in [Guard and Policies](../guides/guard-policies.md)
