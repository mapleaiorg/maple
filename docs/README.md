# MAPLE Documentation

This is the canonical documentation set for the MAPLE Agent Operating System redesign. It is organized by job-to-be-done first, then by deeper architecture and reference material.

## Start Here

- [Installation](getting-started/installation.md)
- [5-Minute Quickstart](getting-started/quickstart.md)
- [Build Your First Agent](getting-started/first-agent.md)

## Architecture

- [Architecture Overview](architecture/overview.md)
- [WorldLine Model](architecture/worldline-model.md)
- [Commitment Boundary](architecture/commitment-boundary.md)
- [Detailed WorldLine Source Guide](architecture/01-worldline.md)
- [Detailed Commitment Boundary Source Guide](architecture/03-commitment-boundary.md)

## Guides

- [Maplefile Reference](guides/maplefile.md)
- [Model Management](guides/model-management.md)
- [Guard and Policies](guides/guard-policies.md)
- [Fleet Deployment](guides/fleet-deployment.md)

## API

- [REST API](api/rest-api.md)
- [CLI Reference](api/cli-reference.md)
- [Rust SDK](api/rust-sdk.md)
- [TypeScript SDK](api/typescript-sdk.md)
- [Python SDK](api/python-sdk.md)

## Reference

- [Architectural Invariants](reference/invariants.md)
- [Profiles](reference/profiles.md)
- [Protocols](reference/protocols.md)
- [Configuration](reference/configuration.md)
- [Why MAPLE](comparison.md)

## Tutorials

- [WorldLine Quickstart](tutorials/worldline-quickstart.md)
- [Operations Tutorial](tutorials/operations.md)
- [Maple Runtime Standalone](tutorials/maple-runtime-standalone.md)
- [Platform Packs](tutorials/platform-packs.md)
- [iBank Commitment Boundary](tutorials/ibank-commitment-boundary.md)

## Advanced and Historical Source Material

The repository still contains deeper source documents from earlier architecture phases. They remain useful as implementation references while the top-level docs converge on the Agent OS structure.

- [WorldLine Framework Guide](worldline-framework.md)
- [ADRs](adr/)
- [Concepts](concepts/)
- [Core](core/)
- [Platforms](platforms/)
- [Products](products/)
- [Conformance](conformance.md)
- [Staged Rollout Checklist](staged-rollout-checklist.md)
