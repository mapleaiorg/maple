# CLI Reference

The current CLI surface is runtime-first. `maple` manages diagnostics, PALM lifecycle, worldlines, commitments, provenance, governance, and the local agent demo. `palm` manages specs, deployments, instances, health, events, and playground backends.

## `maple` command groups

These are the command groups currently exposed by `maple --help`:

- `version`
- `validate`
- `ual`
- `doctor`
- `daemon`
- `agent`
- `worldline`
- `commit`
- `provenance`
- `financial`
- `gov`
- `kernel`
- `palm`

### Notes

- `validate` currently behaves as a developer stub, not as a finished manifest-validation UX.
- `palm` forwards directly to the PALM CLI.
- Product-style package, model, and fleet commands such as `maple build`, `maple model`, and `maple up` are not currently exposed here.

## Common `maple` flows

```bash
# Diagnose local setup
cargo run -p maple-cli -- doctor --model llama3.2:3b

# Start or inspect the daemon
cargo run -p maple-cli -- daemon start --foreground
cargo run -p maple-cli -- daemon status

# Exercise the local agent demo
cargo run -p maple-cli -- agent demo --prompt "log current runtime status"
cargo run -p maple-cli -- agent demo --dangerous --with-commitment --amount 500

# Work with worldlines and provenance
cargo run -p maple-cli -- worldline create --profile agent --label demo-agent
cargo run -p maple-cli -- worldline list
cargo run -p maple-cli -- provenance worldline-history <worldline-id>

# Inspect the runtime
cargo run -p maple-cli -- kernel status
cargo run -p maple-cli -- kernel metrics
```

## `palm` command groups

These are the command groups currently exposed by `palm --help`:

- `spec`
- `deployment`
- `instance`
- `state`
- `health`
- `events`
- `playground`
- `config`
- `status`

## Common `palm` flows

```bash
# Check daemon connectivity
cargo run -p palm -- status

# Inspect playground backends and set the active one
cargo run -p palm -- playground backends
cargo run -p palm -- playground set-backend \
  --kind local_llama \
  --model llama3.2:3b \
  --endpoint http://127.0.0.1:11434
cargo run -p palm -- playground infer "Summarize current runtime health"

# Inspect rollout state
cargo run -p palm -- spec list
cargo run -p palm -- deployment list
```

## Environment variables

These variables are part of the current surfaced toolchain:

| Variable | Used by | Purpose |
| --- | --- | --- |
| `PALM_ENDPOINT` | `maple`, `palm` | Default API endpoint for CLI requests |
| `OLLAMA_HOST` | `maple doctor` | Local Ollama endpoint for connectivity checks |
| `PALM_CONFIG` | `palm`, `palmd` | Config file path |
| `PALM_PLATFORM` | `palm`, `palmd` | Active platform profile |
| `PALM_LISTEN_ADDR` | `palmd` | Daemon listen address |
| `PALM_LOG_LEVEL` | `palmd` | Daemon log level |
| `PALM_LOG_JSON` | `palmd` | Enable JSON logs |
| `PALM_STORAGE_TYPE` | `maple daemon`, `maple doctor` | Storage mode override |
| `PALM_STORAGE_URL` | `maple doctor` and PALM storage config | PostgreSQL connection URL |

## Config

PALM CLI reads:

```text
~/.config/palm/config.toml
```

The current fields are:

```toml
endpoint = "http://127.0.0.1:8080"
default_platform = "development"
default_namespace = "mapleai"
timeout_seconds = 30
```

Use explicit flags for one-off overrides and the PALM config file for stable operator defaults.
