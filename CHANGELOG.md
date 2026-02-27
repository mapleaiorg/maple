# Changelog

All notable changes to MAPLE will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `maple-runtime` feature-gated build tiers:
  - core-only standalone mode (`--no-default-features`)
  - optional cognition and AgentKernel layers (`cognition`, `agent-kernel`, `profile-validation`)
- New standalone tutorial: `docs/tutorials/maple-runtime-standalone.md`

- Initial PALM (Platform Agent Lifecycle Management) implementation
- Platform Pack contract with `PlatformPack` trait
- Three canonical platform packs:
  - `mapleverse-pack`: High-throughput swarm orchestration
  - `finalverse-pack`: Human-centric world simulation
  - `ibank-pack`: Autonomous financial operations
- Conformance test suite for platform pack validation
- Boundary enforcement demo
- Complete documentation and tutorials

### Core Crates

- `palm-types`: Core type definitions
- `palm-registry`: Agent specification registry
- `palm-deployment`: Deployment management
- `palm-health`: Health monitoring with probes
- `palm-state`: Checkpoint and state management
- `palm-control`: Unified control plane
- `palm-policy`: Policy gate system with platform-specific policies
- `palm`: Command-line interface
- `palm-daemon`: Background orchestration service
- `palm-observability`: Metrics, tracing, and audit

### Contracts

- `palm-platform-pack`: Platform pack interface contract
- `palm-conformance`: Conformance test framework

### Documentation

- Refreshed root README, runtime README, and tutorial index for current `main` architecture
- Updated getting started, operations, worldline quickstart, and iBank commitment boundary tutorials
- Updated examples/crates layout READMEs with feature-gated runtime example commands

## [0.1.2] - 2026-02-10

### Added
- First Llama model adapter for resonance-field integrations, enabling pluggable LLMs in the cognitive pipeline (Presence → Meaning).
- Core banking updates in the iBank profile, supporting autonomous DeFi executions on platforms like Libra2.org with commitment-gated transactions.
- AgentKernel enhancements for better worldline persistence and memory tiering (Short-term, Working, Long-term, Episodic).
- New examples in `examples/boundary-demo/` demonstrating commitment boundaries and conformance testing.
- Docs updates in `docs/` for runtime invariants, CLI usage, and platform profiles (Mapleverse, Finalverse, iBank).

### Changed
- Refined maple storage system for improved observability and audit trails, aligning with EVOS axioms (worldline primacy, intrinsic memory).
- Minor optimizations in resonator and palm crates for scalability toward 100M+ agents.

### Fixed
- Resolved minor issues in Cargo dependencies and build scripts.

This release advances Maple's Resonance Architecture toward accountable AI civilizations, with direct applications in iBank.io for crypto banking on Libra2.org. See ROADMAP.md for upcoming bio-digital hybrids and multi-level selection.

[Full Changelog](https://github.com/mapleaiorg/maple/compare/v0.1.1...v0.1.2)

## [0.1.1] - 2026-02-03

### Fixed

- `maple-runtime`: first explicit `signal_presence()` after `register_resonator()` could incorrectly return `PresenceError::RateLimitExceeded`.
- `maple-runtime`: restored resonators can send an immediate explicit presence signal without tripping startup rate limiting.

### Documentation

- Updated getting started guidance for the initial presence signaling path.

## [0.1.0] - 2026-02-01

### Added

- Initial release
- Resonance Architecture implementation
- Platform pack system
- Documentation

[Unreleased]: https://github.com/mapleaiorg/maple/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/mapleaiorg/maple/releases/tag/v0.1.1
[0.1.0]: https://github.com/mapleaiorg/maple/releases/tag/v0.1.0
