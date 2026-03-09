# Fleet Deployment

Fleet is MAPLE's orchestration layer for running many governed agents as a system rather than as a collection of disconnected demos. It manages lifecycle, topology, rollout policy, budgets, and recovery.

## Core lifecycle commands

```bash
maple run myorg/agents/support:1.0.0
maple ps
maple stop support-001
maple suspend support-001
maple resume support-001
```

## Stack topology

`maple-stack.yml`

```yaml
services:
  support:
    image: myorg/agents/support:1.0.0
    replicas: 3
    budget:
      monthlyUsd: 500
    guardPolicy: policies/support-prod.yaml

  evaluator:
    image: myorg/agents/support-eval:0.4.0
    replicas: 1
```

Use `maple up` and `maple down` to reconcile this desired state.

## Rollout strategy

- Start with a shadow or canary slice
- Compare behavior, not only liveness
- Promote only after Guard and eval signals stay inside threshold
- Keep rollback artifacts and receipts immediately accessible

## Topologies

### Local

Single operator, single machine, optional Ollama. Best for iteration.

### Team

Compose-managed shared stack with Postgres plus observability. Best for staging.

### Enterprise

Helm on Kubernetes with managed databases, tenant isolation, and HA services. Best for production estates.

### Air-gapped

Private registry plus mirrored packages and models. Best for sovereign environments.

## Production checklist

- Define cost budgets per tenant or service
- Separate approval rules for high-risk actions
- Retain receipts long enough for audit needs
- Test rollback with real package versions
- Prove backup and restore before launch
