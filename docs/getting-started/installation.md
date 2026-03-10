# Installation

MAPLE is easiest to use from a source checkout today. Build the workspace tools, optionally install Ollama for local model-backed playground flows, and verify the runtime with the shipped examples plus CLI surfaces.

## Prerequisites

- Rust 1.80 or newer
- Git
- PostgreSQL recommended for persistent PALM storage
- Ollama optional for local model-backed playground use
- Docker optional if you want containerized Postgres or Ollama during development

Install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update
```

Install Ollama if you want a local model backend:

```bash
curl -fsSL https://ollama.ai/install.sh | sh
ollama pull llama3.2:3b
```

## Build from source

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple
cargo build --release -p maple-cli -p palm-daemon -p palm

export PATH="$PWD/target/release:$PATH"
maple --help
palmd --help
palm --help
```

This is the recommended install path for the current repo. It gives you the runtime CLI, the PALM daemon, the PALM operator CLI, and the example programs in one checkout.

## Current install status

The repository does not document a single published install target for the full product surface yet. The source build above is the accurate path today.

## Platform notes

### macOS

- Homebrew is the easiest way to install supporting tools.
- Ollama can use Metal-backed local inference on Apple Silicon.
- If you see linker issues after a Rust upgrade, run `xcode-select --install`.

### Linux

- On Ubuntu or Debian, install standard build tooling before compiling Rust dependencies.
- If you need GPU-backed local models, validate CUDA separately before treating MAPLE issues as runtime issues.

### Windows

- Use WSL2 for the cleanest developer path today.
- Keep the repo and cargo cache inside the Linux filesystem rather than a mounted Windows path for better build performance.

## Verify the install

Start with the binaries:

```bash
maple --help
palmd --help
palm --help
```

Then run a non-daemon example:

```bash
cargo run --manifest-path examples/mwl-worldline-lifecycle/Cargo.toml
```

If you want to verify the daemon-backed path, start PALM in one terminal:

```bash
palmd --platform development
```

And in another terminal:

```bash
maple doctor --model llama3.2:3b
```

If Ollama is not running, `maple doctor` will tell you that explicitly. The worldline and daemon surfaces are still usable without it.

## Troubleshooting

### Rust is too old

```bash
rustup update
rustc --version
```

### Ollama is installed but not serving

```bash
ollama serve
```

Run that in a separate terminal before retrying PALM playground or `maple doctor`.

### PALM doctor reports PostgreSQL failures

By default, PALM expects:

```text
postgres://postgres:postgres@localhost:5432/maple
```

Override with `PALM_STORAGE_URL`, or start the daemon in memory mode through `maple daemon start --storage memory`.

### Build failures after dependency changes

```bash
cargo clean
cargo build --release -p maple-cli -p palm-daemon -p palm
```

### `maple`, `palmd`, or `palm` is not found

Call the binaries directly from `target/release`, or ensure that directory is on your `PATH`.

## Next steps

- Continue to [5-Minute Quickstart](quickstart.md)
- Author a package in [Author Your First Agent Package](first-agent.md)
- Review the runtime shape in [Architecture Overview](../architecture/overview.md)
