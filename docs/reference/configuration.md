# Configuration

MAPLE configuration should be explicit enough that an operator can answer three questions quickly:

1. Which services are enabled?
2. Which models and policies are default?
3. Which deployment profile is active?

## Example config

```toml
profile = "standard"
endpoint = "http://localhost:8080"
registry = "registry.mapleai.org"

[model]
default_backend = "ollama"
default_model = "llama3.2:8b-q4"

[guard]
default_policy = "policies/default.yaml"
pii_redaction = true

[fleet]
budget_monthly_usd = 2000
canary_enabled = true
```

## Useful environment variables

| Variable | Meaning |
| --- | --- |
| `PALM_ENDPOINT` | CLI and SDK endpoint override |
| `MAPLE_PROFILE` | Active deployment profile |
| `MAPLE_MODEL_BACKEND` | Default routing backend |
| `MAPLE_MODEL_NAME` | Default model identifier |
| `MAPLE_GUARD_POLICY` | Guard policy override |

## Deployment profiles

### Minimal

Single-node development with lightweight persistence.

### Standard

Shared team environment with observability and policy simulation.

### Financial

Higher approval, audit, and idempotency requirements.

### Sovereign

Private registry, mirrored artifacts, and controlled connectivity.

### Federated

Multi-domain deployment with stronger tenancy boundaries and mirrored policy packs.
