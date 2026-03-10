# Getting Started

The MAPLE getting-started path is now split into three focused documents:

1. [Installation](getting-started/installation.md)
2. [5-Minute Quickstart](getting-started/quickstart.md)
3. [Author Your First Agent Package](getting-started/first-agent.md)

## Fastest Path

If you just want to run something real, start here:

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple
cargo build -p maple-cli -p palm-daemon -p palm
cargo run --manifest-path examples/mwl-worldline-lifecycle/Cargo.toml
```

Then open the quickstart and CLI reference:

- [5-Minute Quickstart](getting-started/quickstart.md)
- [CLI Reference](api/cli-reference.md)

## Why this changed

Older MAPLE docs mixed runtime examples, daemon operations, and conceptual material in one file. The Agent OS redesign separates:

- installation and prerequisites
- first runnable demo
- first package authoring
- deeper architecture and operational guides

Use [docs/README.md](README.md) for the full map.
