# 02 - Crate and Component Layout

This document defines the canonical crate layout for WorldLine, plus the
compatibility layer that preserves existing `maple-*` and `palm-*` consumers.

## 2.1 Canonical layout (filesystem)

```text
crates/
  worldline/
    types/        (crate: worldline-types)
    identity/     (crate: worldline-identity)
    core/         (crate: worldline-core)
    runtime/      (crate: worldline-runtime)
    ledger/       (crate: worldline-ledger)
    governance/   (crate: worldline-governance)
    substrate/    (crate: worldline-substrate)
    conformance/  (crate: worldline-conformance)
    integration/  (crate: worldline-integration)

  # compatibility and legacy implementation crates
  maple-mwl-*
  maple-kernel-*
  maple-worldline-*
  maple-runtime
  palm/*
```

## 2.2 Component dependency graph (text)

```text
maple-mwl-types     -> worldline-types
maple-mwl-identity  -> worldline-identity
worldline-types     -> (native implementation)
worldline-identity  -> worldline-types
worldline-core      -> worldline-types + worldline-identity

worldline-runtime   -> maple-runtime
                    -> maple-kernel-{fabric,memory,gate,mrp,provenance,
                                     governance,safety,profiles,financial}

worldline-ledger    -> maple-kernel-provenance + maple-kernel-fabric + maple-mwl-types

worldline-governance -> maple-kernel-governance
                      -> maple-kernel-gate
                      -> maple-kernel-safety
                      -> maple-kernel-profiles

worldline-substrate -> maple-worldline-{observation,meaning,intent,commitment,
                                        consequence,self-mod-gate,codegen,
                                        deployment,ir,langgen,compiler,sal,
                                        hardware,bootstrap,evos,conformance}

maple-mwl-{conformance,integration} -> worldline-{conformance,integration}
worldline-{conformance,integration} -> worldline-{core,runtime,ledger}
maple-worldline-*                   -> worldline-{core,runtime} (selected crates)
```

## 2.3 Component dependency graph (visual)

### Flowchart

```mermaid
flowchart TB
  subgraph Canonical["Canonical WorldLine crates"]
    WT["worldline-types"]
    WI["worldline-identity"]
    WC["worldline-core"]
    WR["worldline-runtime"]
    WL["worldline-ledger"]
    WG["worldline-governance"]
    WS["worldline-substrate"]
  end

  subgraph Legacy["Legacy implementation crates"]
    MMT["maple-mwl-types"]
    MMI["maple-mwl-identity"]
    MK["maple-kernel-*"]
    MR["maple-runtime"]
    MWS["maple-worldline-*"]
    MMC["maple-mwl-conformance"]
    MMI2["maple-mwl-integration"]
  end

  subgraph Validation["Conformance and integration"]
    MC["worldline-conformance"]
    MI["worldline-integration"]
  end

  MMT --> WT
  MMI --> WI
  WI --> WT
  WC --> WT
  WC --> WI
  WR --> MK
  WR --> MR
  WL --> MK
  WG --> MK
  WS --> MWS

  MC --> WC
  MC --> WR
  MC --> WL
  MMC --> MC
  MI --> WC
  MI --> WR
  MI --> WL
  MMI2 --> MI
```

### UML-style component view

```mermaid
classDiagram
  class worldline_types
  class worldline_identity
  class worldline_core
  class worldline_runtime
  class worldline_ledger
  class worldline_governance
  class worldline_substrate
  class worldline_conformance
  class worldline_integration

  class maple_mwl_types
  class maple_mwl_identity
  class maple_kernel_family
  class maple_runtime
  class maple_worldline_family
  class maple_mwl_conformance
  class maple_mwl_integration

  maple_mwl_types --> worldline_types
  maple_mwl_identity --> worldline_identity
  worldline_identity --> worldline_types
  worldline_core --> worldline_types
  worldline_core --> worldline_identity
  worldline_runtime --> maple_runtime
  worldline_runtime --> maple_kernel_family
  worldline_ledger --> maple_kernel_family
  worldline_governance --> maple_kernel_family
  worldline_substrate --> maple_worldline_family
  maple_mwl_conformance --> worldline_conformance
  worldline_conformance --> worldline_core
  worldline_conformance --> worldline_runtime
  worldline_conformance --> worldline_ledger
  maple_mwl_integration --> worldline_integration
  worldline_integration --> worldline_core
  worldline_integration --> worldline_runtime
  worldline_integration --> worldline_ledger
```

## 2.4 Naming policy

- Use `worldline-*` crates for all new integration code and docs.
- Keep `maple-*` and `palm-*` crates as compatibility surfaces during migration.
- Treat compatibility crates as stable aliases, not as the long-term primary API.

## 2.5 Why this layout

- Reduces cognitive overhead: one canonical namespace for architecture-level concepts.
- Preserves backward compatibility for existing deployments.
- Keeps control-plane and data-plane boundaries explicit while enabling gradual migration.

## 2.6 Compatibility relationships

- `worldline-types` and `worldline-identity` now hold the canonical implementations.
- `maple-mwl-types` and `maple-mwl-identity` are compatibility wrappers.
- `worldline-core` composes `worldline-types` and `worldline-identity`.
- `worldline-conformance` and `worldline-integration` are canonical suite crates.
- `maple-mwl-conformance` and `maple-mwl-integration` are compatibility wrappers.
- Legacy `maple-kernel-*` and `maple-worldline-*` crates remain the implementation
  substrate for one compatibility cycle.
