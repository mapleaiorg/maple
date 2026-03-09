# Maplefile Reference

The Maplefile is the package contract for MAPLE artifacts. It describes what an agent is, which models it can use, which skills it may invoke, how memory is wired, and which Guard rules apply.

## Core shape

```yaml
apiVersion: maple.ai/v1alpha1
kind: AgentPackage
metadata:
  org: myorg
  name: support-agent
  version: 1.0.0

model:
  ref: ollama:llama3.2:8b-q4

skills:
  - ref: registry.mapleai.org/skills/zendesk:1.2.0

memory:
  backend: sqlite
  path: ./.maple/support.sqlite

guard:
  policyRef: ./policies/support.yaml
```

## Common package kinds

The prompt pack frames MAPLE packages as a supply chain, not a single artifact type. Public documentation should at least account for:

1. `AgentPackage`
2. `SkillPackage`
3. `ModelPackage`
4. `StackPackage`
5. `PolicyPack`
6. `CompliancePack`
7. `ConnectorPack`
8. `EvalPack`
9. `DeploymentPack`

## Build, sign, and ship

```bash
maple build -t myorg/agents/support:1.0.0 .
maple sign myorg/agents/support:1.0.0
maple sbom myorg/agents/support:1.0.0
maple push myorg/agents/support:1.0.0
```

That sequence is the packaging story in one line: reproducible artifact, signed metadata, software bill of materials, then registry distribution.

## What belongs in the package

- Prompts and role contracts
- Skill manifests and sandbox policy
- Guard policy references
- Model requirements and routing hints
- Versioned metadata and ownership

## What does not belong in the package

- Live secrets
- Tenant-specific credentials
- Unbounded tool access
- Mutable runtime state

## Air-gap and mirroring

```bash
maple mirror registry.mapleai.org/myorg/agents/support:1.0.0 ./mirror
maple import ./mirror
```

Use this flow when a deployment domain cannot pull from public registries directly.

## Best practices

- Prefer explicit package versions over floating tags.
- Keep Guard policy references close to the Maplefile so package intent stays inspectable.
- Treat model bindings as policy-routed defaults, not permanent lock-in.
- Generate an SBOM for every artifact that crosses an environment boundary.
