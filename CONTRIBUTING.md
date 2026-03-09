# Contributing to MAPLE

MAPLE is the open-source implementation foundation for the MapleAI Agent OS. Contributions should improve one of three things:

1. governed execution
2. developer usability
3. operational clarity

Brand: `MapleAI`  
Legal entity: `MapelAI Intelligence Inc.`

## Before You Open a PR

- Read the top-level [README.md](README.md)
- Check the [docs index](docs/README.md)
- Search existing issues and pull requests
- Be explicit about whether your change affects runtime behavior, operator behavior, or documentation only

## Contribution Areas

### Runtime and WorldLine

- identity, memory, provenance, commitment gating
- safety invariants and replay behavior
- model-neutral execution surfaces

### Supply chain and packaging

- Maplefile schema
- build, signing, SBOM, and mirroring flows
- registry and artifact movement

### Governance and operations

- Guard policies
- approvals, compliance, and risk controls
- PALM daemon and CLI workflows
- fleet and observability surfaces

### Documentation

- getting-started guides
- API and SDK references
- tutorials that reflect the current implementation and Agent OS direction

## Development Setup

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple
cargo build
```

Recommended docs and runtime checks before a PR:

```bash
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

If you touch `maple-runtime`, also validate the minimal matrix:

```bash
cargo check -p maple-runtime --no-default-features
cargo test -p maple-runtime --no-default-features --lib
```

## Documentation Expectations

- Prefer the Agent OS vocabulary over older architecture summaries
- Keep runnable commands accurate for the current repository
- Explain what is implemented now versus what is target UX
- Link outward from indexes instead of duplicating large blocks of reference text

## Pull Request Checklist

- change is scoped and described clearly
- tests added or updated where behavior changed
- docs updated if public behavior or developer workflow changed
- commands in docs were sanity-checked locally where practical
- changelog updated when the change is user-visible

## Commit Style

Use clear commit messages such as:

```text
docs(readme): rewrite root docs for Agent OS layout
feat(guard): add approval hold metadata to decision receipts
fix(cli): preserve endpoint override for worldline commands
```

## Getting Help

- Website: <https://mapleai.org>
- Docs: <https://mapleai.org/docs>
- Email: <hello@mapleai.org>
