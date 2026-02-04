# Operations Tutorial: CLI, Daemon, Playground

This tutorial shows how to run the PALM control plane, monitor agents in real time from the terminal, and optionally use the Playground UI for live visualization and replay.

## 1. Choose Your CLI

MAPLE uses a single umbrella CLI (`maple`). PALM operations can run either directly (`maple ...`) or via explicit namespace (`maple palm ...`).

- **`maple`** is the primary entrypoint and includes developer utilities.
- **`maple ...`** (for PALM verbs like `spec`, `deployment`, `instance`, `events`, `playground`) forwards directly to PALM.
- **`maple palm ...`** remains fully supported as explicit namespace.
- **`palm`** remains available as a direct operations CLI (backwards compatible).

Example developer CLI usage:

```bash
cargo run -p maple-cli -- version
cargo run -p maple-cli -- validate --file README.md
cargo run -p maple-cli -- spec list
```

Install once (no `cargo run` required):

```bash
cargo install --path crates/maple-cli --bin maple && cargo install --path crates/palm --bin palm
```

## 2. Start the Daemon

```bash
# Start PALM daemon (API + control plane)
cargo run -p palm-daemon

# Or use maple lifecycle commands
maple daemon start --platform mapleverse
```

Defaults:

- Storage: PostgreSQL (default URL is `postgres://postgres:postgres@localhost:5432/maple`)
- API: `http://127.0.0.1:8080`
- Playground UI: `http://127.0.0.1:8080/playground`

Development behavior: if PostgreSQL is not reachable, the daemon now falls back to in-memory storage so you can still boot and explore. For durable state, run PostgreSQL and keep the default storage config.

Force in-memory mode explicitly:

```bash
PALM_STORAGE_TYPE=memory cargo run -p palm-daemon
```

You can override settings with a config file via `PALM_CONFIG` or `--config`. Environment overrides use the `PALM_` prefix.

Daemon lifecycle commands:

```bash
# Show health + managed PID state
maple daemon status

# Graceful stop via API (fallback to PID terminate)
maple daemon stop

# Alias
maple daemon shutdown
```

### First-Time Local Setup (PostgreSQL + Ollama)

If this is your first local run, do these once.

1. Start PostgreSQL (recommended: Docker)

```bash
docker run --name maple-postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=maple \
  -p 5432:5432 \
  -v maple_pgdata:/var/lib/postgresql/data \
  -d postgres:16
```

2. Verify PostgreSQL is ready

```bash
docker exec maple-postgres pg_isready -U postgres -d maple
```

3. Start PALM daemon with explicit PostgreSQL settings

```bash
PALM_STORAGE_TYPE=postgres \
PALM_STORAGE_URL=postgres://postgres:postgres@localhost:5432/maple \
cargo run -p palm-daemon -- --platform mapleverse
```

4. Start Ollama and pull a model used by Playground inference

```bash
ollama serve
ollama pull llama3
```

5. If you use a different local model, set backend model explicitly

```bash
cargo run -p maple-cli -- playground set-backend \
  --kind local_llama \
  --model llama3 \
  --endpoint http://127.0.0.1:11434
```

### Run Doctor Checks

Use the built-in doctor to validate daemon, storage, and Ollama/model readiness:

```bash
maple doctor
maple doctor --model llama3.2
```

## 3. Real-Time Monitoring from the CLI

Real-time agent status and monitoring can be done entirely from the terminal:

```bash
# Live event stream (Ctrl+C to stop)
cargo run -p maple-cli -- events watch

# Live activity feed from the playground store
cargo run -p maple-cli -- playground activities --limit 50

# Status snapshots
cargo run -p maple-cli -- playground agents
cargo run -p maple-cli -- playground resonators

# Health checks
cargo run -p maple-cli -- health summary
```

Tip: add `--output json` for scripting and automation.

Direct operations CLI (optional):

```bash
cargo run -p palm -- events watch
```

## 4. Playground UI (Optional)

The Playground is a live, game-like view for human/web observation and replay. It is optional and does not affect runtime behavior.

```bash
open http://localhost:8080/playground
```

## 5. AI Backend Selection

Local Llama is the default AI backend for the Playground. You can switch backends via the umbrella CLI:

```bash
# Local Llama (default)
cargo run -p maple-cli -- playground set-backend --kind local_llama --model llama3 --endpoint http://127.0.0.1:11434

# OpenAI
cargo run -p maple-cli -- playground set-backend --kind open_ai --model gpt-4o-mini --api-key YOUR_KEY

# Anthropic
cargo run -p maple-cli -- playground set-backend --kind anthropic --model claude-3-5-sonnet --api-key YOUR_KEY

# Grok (xAI)
cargo run -p maple-cli -- playground set-backend --kind grok --model grok-2-latest --api-key YOUR_KEY

# Gemini (Google)
cargo run -p maple-cli -- playground set-backend --kind gemini --model gemini-2.0-flash --api-key YOUR_KEY
```

Run one-shot inference on the active backend:

```bash
cargo run -p maple-cli -- playground infer "Summarize the latest system activity"
cargo run -p maple-cli -- playground infer "Draft UAL for scaling deployment dep-123 to 5" --system-prompt "You are a MAPLE ops copilot"
```

Auto-inference simulation mode is **enabled by default** and periodically invokes the active backend, writing `agent_cognition` activities. You can tune or disable it in the dashboard (**Simulation** tab) or via CLI.

CLI alternative:

```bash
cargo run -p maple-cli -- playground set-simulation --auto-inference-enabled true --inference-interval-ticks 4 --inferences-per-tick 2
```

## 6. Headless Runtime

You can run MAPLE without PALM or the Playground when you want embedded, headless agents. For example:

```bash
cargo run -p maple-runtime --example 01_basic_resonator
```
