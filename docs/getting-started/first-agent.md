# Build Your First Agent

This tutorial builds a small TODO agent that can add, list, complete, and delete tasks. The point is not sophisticated task management. The point is to feel the MAPLE packaging model: agent contract, model binding, deny-by-default capability grants, and auditable actions.

## 1. Scaffold the package

```bash
mkdir my-todo-agent
cd my-todo-agent
maple init --kind agent-package --name my-todo-agent --org myorg
```

Expected shape:

```text
my-todo-agent/
├── Maplefile.yaml
├── prompts/
│   └── system.md
├── skills/
│   └── todo/
└── policies/
    └── guard.yaml
```

## 2. Configure the Maplefile

```yaml
apiVersion: maple.ai/v1alpha1
kind: AgentPackage
metadata:
  name: my-todo-agent
  org: myorg
  version: 0.1.0

model:
  ref: ollama:llama3.2:8b-q4
  routingPolicy: default-local

memory:
  backend: sqlite
  path: ./.maple/todo.sqlite

skills:
  - ref: ./skills/todo

guard:
  mode: deny-by-default
  allow:
    - todo.add
    - todo.list
    - todo.complete
    - todo.delete
```

The key idea is that the package declares cognition, memory, and tool boundaries together. MAPLE is not treating tool access as an afterthought bolted onto a chat loop.

## 3. Write the system prompt

`prompts/system.md`

```md
You are a TODO list assistant.
You help users add, list, complete, and delete tasks.
Available tools: todo.add, todo.list, todo.complete, todo.delete
Always confirm before deleting tasks.
```

## 4. Define the skill surface

`skills/todo/manifest.yaml`

```yaml
name: todo
tools:
  - id: todo.add
    input: { text: string }
  - id: todo.list
    input: {}
  - id: todo.complete
    input: { id: string }
  - id: todo.delete
    input: { id: string }
```

You can implement the tool handlers in Rust, TypeScript, or Python. For a first pass, keep them in-memory and focus on the capability contract.

## 5. Build the package

```bash
maple build -t myorg/agents/todo:0.1.0 .
```

The build step should assemble your Maplefile, prompts, skill manifest, policy files, and metadata into a versioned package artifact.

## 6. Run the agent

```bash
maple run myorg/agents/todo:0.1.0
```

Example interaction:

```text
> add buy groceries
Added task #1: buy groceries

> list my tasks
1. buy groceries [open]

> complete task 1
Marked task #1 as complete
```

## 7. Inspect the audit trail

```bash
maple provenance worldline-history <worldline-id>
```

You should see that each meaningful action is attached to an identity and a receipt trail. That is the main difference between a packaged MAPLE agent and a plain application-level chat bot.

## 8. Add a delete guard

`policies/guard.yaml`

```yaml
version: v1
default: deny
rules:
  - id: allow-safe-todo-ops
    match:
      capability: [todo.add, todo.list, todo.complete]
    action: allow

  - id: confirm-delete
    match:
      capability: [todo.delete]
    action: require_approval
    approvals:
      count: 1
```

Now the agent can draft a deletion, but consequence is held until confirmation exists.

## Where to go next

- Publish and sign packages in [/docs/guides/maplefile](https://mapleai.org/docs/guides/maplefile)
- Add model routing in [/docs/guides/model-management](https://mapleai.org/docs/guides/model-management)
- Review Guard workflows in [/docs/guides/guard-policies](https://mapleai.org/docs/guides/guard-policies)
