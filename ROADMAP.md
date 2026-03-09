# MAPLE Roadmap

This roadmap tracks the MAPLE Agent Operating System redesign and the implementation layers that support it.

## Direction

MAPLE is converging on six operating surfaces:

1. Maple Runtime
2. Maple Registry
3. Maple Models
4. Maple Guard
5. Maple Foundry
6. Maple Fleet

The repository already contains significant portions of these layers as crates, daemon surfaces, and documentation. The roadmap below describes the convergence work.

## Phase A: Foundation and WorldLine Kernel

Status: active

- worldline identity and continuity
- event fabric and commitment gate
- provenance, replay, and ledger bindings
- typed memory and profile-aware runtime constraints

## Phase B: Packaging and Supply Chain

Status: active

- Maplefile and package schema hardening
- build provenance and reproducibility
- signing, verification, SBOM, and trust metadata
- registry push, pull, mirror, and catalog flows

## Phase C: Model Operations

Status: active

- local and hosted model adapter support
- routing and benchmarking
- open serving surfaces for compatible tooling
- policy-aware backend selection

## Phase D: Guard and Governance

Status: active

- deny-by-default capability firewall
- approvals and risk scoring
- PII and secret handling
- compliance overlays and evidence export

## Phase E: Foundry and Fleet

Status: active

- trace capture and eval loops
- distillation and routing improvements
- stack topology and rollout policy
- cost budgets, tenancy, and operational visibility

## Documentation Priorities

- keep repo docs and `mapleai.org/docs` aligned
- maintain runnable getting-started paths
- separate current implementation guidance from target UX language
- rewrite older indexes and tutorials as the Agent OS IA stabilizes

## Short-Term Priorities

- stabilize top-level docs and package workflow guidance
- expose more of the package and model ergonomics through the `maple` CLI
- tighten Guard, provenance, and financial consequence walkthroughs
- improve official website and repo-doc consistency

## Long-Term Goal

Make MAPLE the production operating layer for governed agent systems built under the MapleAI brand and operated by MapelAI Intelligence Inc.
