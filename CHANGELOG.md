# Changelog

All notable changes to MAPLE will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
- `palm-cli`: Command-line interface
- `palm-daemon`: Background orchestration service
- `palm-observability`: Metrics, tracing, and audit

### Contracts

- `palm-platform-pack`: Platform pack interface contract
- `palm-conformance`: Conformance test framework

### Documentation

- Architecture guide
- Platform packs tutorial
- Conformance testing guide
- API reference

## [0.1.0] - 2026-02-01

### Added

- Initial release
- Resonance Architecture implementation
- Platform pack system
- Documentation

[Unreleased]: https://github.com/mapleaiorg/maple/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/mapleaiorg/maple/releases/tag/v0.1.0
