# CLI Reference

The `maple` CLI is the shortest path to the platform model. It combines package workflow, model management, runtime control, governance, and provenance inspection under one command surface.

## Core command groups

```bash
maple init
maple build
maple sign
maple push
maple pull
maple inspect

maple model pull
maple model ls
maple model serve

maple worldline create
maple commit submit
maple provenance worldline-history
maple kernel status
maple gov list
```

## Package workflow

```bash
maple init --kind agent-package --name support-agent
maple build -t myorg/agents/support:1.0.0 .
maple sign myorg/agents/support:1.0.0
maple push myorg/agents/support:1.0.0
```

## Runtime workflow

```bash
maple worldline create --profile financial --label treasury-a
maple commit submit --file ./payment.json
maple provenance worldline-history wl_123
maple kernel metrics
```

## Fleet workflow

```bash
maple up -f maple-stack.yml
maple ps
maple down -f maple-stack.yml
```

## Environment variables

| Variable | Purpose |
| --- | --- |
| `PALM_ENDPOINT` | Default API endpoint for CLI requests |
| `MAPLE_PROFILE` | Default execution or deployment profile |
| `MAPLE_TOKEN` | API credential for shared environments |

## Config

```toml
endpoint = "http://localhost:8080"
profile = "standard"

[model]
default_backend = "ollama"
default_model = "llama3.2:8b-q4"
```

Use environment variables for CI or ephemeral overrides, and a config file for stable operator defaults.
