# Operations Tutorial: CLI, Daemon, Playground

This tutorial shows how to run the PALM control plane, monitor agents in real time from the terminal, and optionally use the Playground UI for live visualization and replay.

## 1. Choose Your CLI

MAPLE uses a single umbrella CLI (`maple`) with an operations subcommand (`maple palm ...`).

- **`maple`** is the primary entrypoint and includes developer utilities.
- **`maple palm ...`** forwards to the PALM control plane for specs, deployments, instances, health, events, and Playground.
- **`palm`** remains available as a direct operations CLI (backwards compatible).

Example developer CLI usage:

```bash
cargo run -p maple-cli -- version
cargo run -p maple-cli -- validate --file README.md
```

Install once (no `cargo run` required):

```bash
cargo install --path crates/maple-cli --bin maple && cargo install --path crates/palm --bin palm
```

## 2. Start the Daemon

```bash
# Start PALM daemon (API + control plane)
cargo run -p palm-daemon
```

Defaults:

- Storage: PostgreSQL (default URL is `postgres://postgres:postgres@localhost:5432/maple`)
- API: `http://127.0.0.1:8080`
- Playground UI: `http://127.0.0.1:8080/playground`

You can override settings with a config file via `PALM_CONFIG` or `--config`. Environment overrides use the `PALM_` prefix.

## 3. Real-Time Monitoring from the CLI

Real-time agent status and monitoring can be done entirely from the terminal:

```bash
# Live event stream (Ctrl+C to stop)
cargo run -p maple-cli -- palm events watch

# Live activity feed from the playground store
cargo run -p maple-cli -- palm playground activities --limit 50

# Status snapshots
cargo run -p maple-cli -- palm playground agents
cargo run -p maple-cli -- palm playground resonators

# Health checks
cargo run -p maple-cli -- palm health summary
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
cargo run -p maple-cli -- palm playground set-backend --kind local_llama --model llama3 --endpoint http://127.0.0.1:11434

# OpenAI
cargo run -p maple-cli -- palm playground set-backend --kind open_ai --model gpt-4o-mini --api-key YOUR_KEY

# Anthropic
cargo run -p maple-cli -- palm playground set-backend --kind anthropic --model claude-3-5-sonnet --api-key YOUR_KEY

# Grok (xAI)
cargo run -p maple-cli -- palm playground set-backend --kind grok --model grok-2-latest --api-key YOUR_KEY

# Gemini (Google)
cargo run -p maple-cli -- palm playground set-backend --kind gemini --model gemini-2.0-flash --api-key YOUR_KEY
```

Run one-shot inference on the active backend:

```bash
cargo run -p maple-cli -- palm playground infer "Summarize the latest system activity"
cargo run -p maple-cli -- palm playground infer "Draft UAL for scaling deployment dep-123 to 5" --system-prompt "You are a MAPLE ops copilot"
```

## 6. Headless Runtime

You can run MAPLE without PALM or the Playground when you want embedded, headless agents. For example:

```bash
cargo run -p maple-runtime --example 01_basic_resonator
```
