# Changelog

All notable changes to MAPLE are documented here.

The project follows Keep a Changelog style and uses semantic versioning where release tags exist.

## [Unreleased]

### Documentation

- rewrote the root README around the MapleAI Agent OS narrative
- added the new docs information architecture under `docs/getting-started`, `docs/guides`, `docs/api`, `docs/reference`, and `docs/architecture/overview.md`
- refreshed top-level docs indexes, examples docs, and crate layout docs
- rewrote core tutorials to reflect the current runtime, PALM, and worldline flows
- aligned brand and corporation references with `MapleAI` and `MapelAI Intelligence Inc.`

## [0.1.3] - 2026-03-05

### Added

- `maple-runtime` feature-gated build tiers:
  - core-only standalone mode (`--no-default-features`)
  - optional cognition and AgentKernel layers (`cognition`, `agent-kernel`, `profile-validation`)
- initial package, model, guard, foundry, and fleet crate families for the Agent OS redesign
- new standalone tutorial for `maple-runtime`

### Changed

- PALM and worldline documentation were reorganized around clearer runtime and operations workflows
- examples and crate layout docs were refreshed for feature-gated runtime examples

## [0.1.2] - 2026-02-10

### Added

- initial Llama adapter work for resonance-field integrations
- iBank profile improvements for commitment-gated financial flows
- AgentKernel enhancements for worldline persistence and memory tiering

### Changed

- storage and observability layers were refined around auditability and provenance

## [0.1.1] - 2026-02-03

### Fixed

- `maple-runtime`: initial explicit `signal_presence()` after registration no longer trips startup rate limiting

## [0.1.0] - 2026-02-01

### Added

- initial release
- resonance architecture implementation
- platform pack and control-plane foundations

[0.1.3]: https://github.com/mapleaiorg/maple/releases/tag/v0.1.3
[0.1.2]: https://github.com/mapleaiorg/maple/releases/tag/v0.1.2
[0.1.1]: https://github.com/mapleaiorg/maple/releases/tag/v0.1.1
[0.1.0]: https://github.com/mapleaiorg/maple/releases/tag/v0.1.0
