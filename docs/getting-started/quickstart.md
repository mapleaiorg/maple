# 5-Minute Quickstart

This is the fastest accurate path through the current MAPLE repo. It shows three things:

1. a pure worldline example with no daemon dependency
2. the daemon-backed runtime surface exposed by `maple-cli`
3. the optional Ollama-backed playground path exposed by PALM

## 1. Build the tools

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple
cargo build -p maple-cli -p palm-daemon -p palm
```

## 2. Run a worldline example

```bash
cargo run --manifest-path examples/mwl-worldline-lifecycle/Cargo.toml
```

This is the quickest way to see the repo's core thesis in action: durable identity, lifecycle state, and recorded receipts instead of an ephemeral chat loop.

## 3. Start the daemon

In a new terminal:

```bash
cargo run -p maple-cli -- daemon start --foreground
```

## 4. Inspect the runtime from another terminal

```bash
cargo run -p maple-cli -- kernel status
cargo run -p maple-cli -- worldline create --profile agent --label quickstart-agent
cargo run -p maple-cli -- worldline list
```

Those commands exercise the current public runtime surface directly: kernel status, worldline creation, and daemon-backed control.

## 5. Optional: connect a local Ollama backend

If you want the local-model path too:

```bash
ollama serve
ollama pull llama3.2:3b

cargo run -p maple-cli -- doctor --model llama3.2:3b
cargo run -p palm -- playground set-backend \
  --kind local_llama \
  --model llama3.2:3b \
  --endpoint http://127.0.0.1:11434
cargo run -p palm -- playground infer "Summarize the current playground status"
```

This is the practical "Ollama-like" MAPLE workflow today: Ollama provides the local backend, PALM provides governed backend selection and inference entry points, and the model crates provide the underlying store, router, server, and benchmark types.

## What you are seeing

1. Worldlines turn actors into durable identities.
2. PALM turns the runtime into a controllable daemon rather than a one-off process.
3. `maple-cli` exposes current governance and provenance surfaces directly.
4. Ollama is optional; governance and provenance remain visible without it.

## If Ollama is not running

Skip step 5. The worldline, kernel, daemon, provenance, and governance flows are still valid and useful without a local model backend.

## Next steps

- Clean up installation details in [Installation](installation.md)
- Author a package contract in [Author Your First Agent Package](first-agent.md)
- Inspect the boundary design in [Commitment Boundary](../architecture/commitment-boundary.md)
