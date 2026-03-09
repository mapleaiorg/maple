# Getting Started

The MAPLE getting-started path is now split into three focused documents:

1. [Installation](getting-started/installation.md)
2. [5-Minute Quickstart](getting-started/quickstart.md)
3. [Build Your First Agent](getting-started/first-agent.md)

## Fastest Path

If you just want to run something real, start here:

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple
cargo build
cargo run -p palm-daemon
```

Then open the quickstart and CLI reference:

- [5-Minute Quickstart](getting-started/quickstart.md)
- [CLI Reference](api/cli-reference.md)

## Why this changed

Older MAPLE docs mixed runtime examples, daemon operations, and conceptual material in one file. The Agent OS redesign separates:

- installation and prerequisites
- first runnable demo
- first package build
- deeper architecture and operational guides

Use [docs/README.md](README.md) for the full map.
