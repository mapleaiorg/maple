# Rust SDK

MAPLE does not currently publish a single `maple-sdk` crate. The Rust integration surface is workspace-first.

## What to use today

- `maple-kernel-sdk` for shared worldline, commitment, provenance, and kernel integration surfaces
- `maple-runtime` when you want to embed runtime behavior directly
- `maple-package`, `maple-build`, `maple-package-trust`, and `maple-registry-client` for the package pipeline
- `maple-model-*` crates for model storage, routing, serving, and benchmarking

## Installation

Use path dependencies from a local checkout or a vendored copy of the workspace:

```toml
[dependencies]
maple-kernel-sdk = { path = "../maple/crates/maple-kernel-sdk" }
maple-runtime = { path = "../maple/crates/maple-runtime" }
```

Adjust the paths to match your repository layout.

## Typical flow

1. Run `palmd` or `maple daemon start --foreground`.
2. Use `maple-kernel-sdk` or plain HTTP against `PALM_ENDPOINT`.
3. Keep package and model flows in Rust using the workspace crates directly.
4. Use the CLI and examples when you want operator-facing behavior rather than embedded library behavior.

## Status

A single polished published Rust SDK may come later. The implemented Rust surface today is the workspace itself.
