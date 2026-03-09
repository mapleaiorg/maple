# Installation

MAPLE is designed to be usable in a local laptop loop first, then carried into team and production environments without changing the core operating model. The fastest path is: install Rust, optionally install Ollama for local models, build the repo, and verify the CLI plus demo path.

## Prerequisites

- Rust 1.80 or newer
- Git
- Ollama recommended for local models
- Docker optional for compose-based environments

Install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update
```

Install Ollama:

```bash
curl -fsSL https://ollama.ai/install.sh | sh
ollama pull llama3.2
```

## Install from source

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple
cargo build --release

export PATH="$PWD/target/release:$PATH"
maple --version
```

This path is the best fit if you want the demo binaries, examples, and the latest runtime code at the same time.

## Install via Cargo

When the CLI is published independently, use:

```bash
cargo install maple-cli
maple --version
```

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
- Keep your repository and build cache inside the Linux filesystem rather than a mounted Windows path for better cargo performance.

## Verify the install

```bash
maple --version
curl -s http://localhost:11434/api/tags | head
cargo run -p maple-demo
```

If Ollama is not running, the demo should still degrade gracefully. You lose local-model execution, but the rest of the runtime path remains testable.

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

Run that in a separate terminal before retrying model operations.

### Build failures after dependency changes

```bash
cargo clean
cargo build --release
```

### `maple` is not found

Check that `target/release` is on your `PATH`, or call the binary directly:

```bash
./target/release/maple --version
```

## Next steps

- Continue to the [/docs/getting-started/quickstart](https://mapleai.org/docs/getting-started/quickstart)
- Package an agent in [/docs/getting-started/first-agent](https://mapleai.org/docs/getting-started/first-agent)
- Review the runtime shape in [/docs/architecture/overview](https://mapleai.org/docs/architecture/overview)
