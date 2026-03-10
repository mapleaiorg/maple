# MAPLE API

This section organizes the public-facing MAPLE control surfaces around the Agent OS redesign.

## Start Here

- [REST API](rest-api.md)
- [CLI Reference](cli-reference.md)
- [Rust SDK](rust-sdk.md)
- [TypeScript SDK](typescript-sdk.md)
- [Python SDK](python-sdk.md)

## Current Implementation Notes

The current repository still exposes some lower-level or compatibility-oriented API material through:

- `palm-daemon` playground and operational endpoints
- `maple-cli` worldline, provenance, kernel, and governance commands
- `maple-kernel-sdk` CLI and REST integration surfaces
- Rust-first workspace crates for packaging, models, guard, and fleet

Additional status:

- Rust integration is workspace-first; there is not a single published `maple-sdk` crate documented here.
- Official TypeScript and Python SDK packages are not published from this repo today.

Use the documents above for the current top-level API story. Use the deeper source files under `crates/` when you need crate-specific implementation details.
