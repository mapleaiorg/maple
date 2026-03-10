# Fleet Deployment

Fleet is MAPLE's orchestration layer for running many governed agents as a system rather than as a collection of disconnected demos. In the current repo, the operational control surface is PALM, and the stack-definition surface is the `maple-fleet-stack` crate.

## Current operator workflow

Start PALM:

```bash
cargo run -p maple-cli -- daemon start --foreground
```

Then use the PALM CLI:

```bash
cargo run -p palm -- status
cargo run -p palm -- spec list
cargo run -p palm -- deployment list
cargo run -p palm -- playground status
```

The currently exposed PALM command groups are:

- `spec`
- `deployment`
- `instance`
- `state`
- `health`
- `events`
- `playground`

That is the real operator surface today.

## Stack topology

`maple-fleet-stack` already implements a Docker Compose-like stack definition for agents, but it is currently a Rust crate surface rather than a finished `maple up` CLI.

`maple-stack.yml`

```yaml
name: support-stack
version: "1.0"

services:
  support:
    agent_ref: "myorg/agents/support:1.0.0"
    replicas: 3
    environment:
      MAPLE_TENANT: "acme"
    depends_on: []

  evaluator:
    agent_ref: "myorg/agents/support-eval:0.4.0"
    replicas: 1
    depends_on:
      - support
```

The implemented stack schema supports:

- `agent_ref`
- `replicas`
- `environment`
- `depends_on`
- `resources`
- `health_check`

## What is "Docker-compose-like" here

`maple-fleet-stack` provides:

- YAML parsing for multi-service agent stacks
- dependency validation
- topological sort for startup and teardown ordering
- stack and service lifecycle state tracking

This is the MAPLE equivalent of a compose file for agents. The final `maple up`, `maple down`, and `maple ps` UX is not wired into `maple-cli` yet.

## Rollout strategy

- Start with a shadow or canary slice
- Compare behavior, not only liveness
- Promote only after Guard and eval signals stay inside threshold
- Keep rollback artifacts and receipts immediately accessible

## Topologies

### Local

Single operator, single machine, PALM daemon, and optional Ollama. Best for iteration.

### Team

Shared PALM environment with Postgres, playground backends, and basic observability. Best for staging.

### Enterprise

Multiple deployments, stronger tenancy controls, managed persistence, and external orchestration. Best for production estates.

### Air-gapped

Private registry plus mirrored packages and models. Best for sovereign environments.

## Production checklist

- Define cost budgets per tenant or service
- Separate approval rules for high-risk actions
- Retain receipts long enough for audit needs
- Test rollback with real package versions
- Prove backup and restore before launch
