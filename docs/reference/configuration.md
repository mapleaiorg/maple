# Configuration

The current repo has two real configuration surfaces:

1. PALM daemon configuration, loaded by `palmd`
2. PALM CLI configuration, loaded by `palm`

`maple-cli` is mostly flag- and environment-driven today. It does not currently read a separate MAPLE-wide config file with the fields that older docs implied.

## PALM daemon config

`palmd` loads defaults, then an optional config file, then `PALM_` environment variables.

Example:

```toml
[server]
listen_addr = "127.0.0.1:8080"
enable_cors = true
request_timeout_secs = 30
max_body_size = 10485760

[storage]
type = "postgres"
url = "postgres://postgres:postgres@localhost:5432/maple"
max_connections = 10
connect_timeout_secs = 5

[scheduler]
reconcile_interval_secs = 10
health_check_interval_secs = 5
metrics_interval_secs = 15
max_concurrent_reconciliations = 10
auto_healing_enabled = true

platform = "development"

[logging]
level = "info"
json = false
timestamps = true
```

## PALM CLI config

`palm` looks for:

```text
~/.config/palm/config.toml
```

Example:

```toml
endpoint = "http://127.0.0.1:8080"
default_platform = "development"
default_namespace = "mapleai"
timeout_seconds = 30
```

## Useful environment variables

| Variable | Used by | Meaning |
| --- | --- | --- |
| `PALM_ENDPOINT` | `maple`, `palm` | Default API endpoint |
| `OLLAMA_HOST` | `maple doctor` | Local Ollama endpoint |
| `PALM_CONFIG` | `palm`, `palmd` | Explicit config file path |
| `PALM_PLATFORM` | `palm`, `palmd` | Active platform profile |
| `PALM_LISTEN_ADDR` | `palmd` | Daemon listen address |
| `PALM_LOG_LEVEL` | `palmd` | Log level |
| `PALM_LOG_JSON` | `palmd` | Enable JSON logs |
| `PALM_STORAGE_TYPE` | `maple daemon`, `maple doctor` | Storage mode override |
| `PALM_STORAGE_URL` | PALM storage checks and config | PostgreSQL connection URL |

## Important status note

These older variables are not the current public config contract:

- `MAPLE_PROFILE`
- `MAPLE_MODEL_BACKEND`
- `MAPLE_MODEL_NAME`
- `MAPLE_GUARD_POLICY`
- `MAPLE_TOKEN`

Use explicit CLI flags, PALM config files, and the verified `PALM_` environment variables above instead.
