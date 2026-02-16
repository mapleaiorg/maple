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

  # compatibility and legacy implementation crates
  maple-mwl-*
  maple-kernel-*
  maple-worldline-*
  maple-runtime
  palm/*
```

## 2.2 Component dependency graph (text)

```text
worldline-types     -> maple-mwl-types
worldline-identity  -> maple-mwl-identity + worldline-types
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

maple-mwl-{conformance,integration} -> worldline-{core,runtime,ledger}
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
  end

  subgraph Validation["Conformance and integration"]
    MC["maple-mwl-conformance"]
    MI["maple-mwl-integration"]
  end

  WT --> MMT
  WI --> MMI
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
  MI --> WC
  MI --> WR
  MI --> WL
```

### UML-style component view

```mermaid
classDiagram
  class worldline_types <<crate>>
  class worldline_identity <<crate>>
  class worldline_core <<crate>>
  class worldline_runtime <<crate>>
  class worldline_ledger <<crate>>
  class worldline_governance <<crate>>
  class worldline_substrate <<crate>>

  class maple_mwl_types <<crate>>
  class maple_mwl_identity <<crate>>
  class maple_kernel_family <<crate_group>>
  class maple_runtime <<crate>>
  class maple_worldline_family <<crate_group>>

  worldline_types --> maple_mwl_types
  worldline_identity --> maple_mwl_identity
  worldline_identity --> worldline_types
  worldline_core --> worldline_types
  worldline_core --> worldline_identity
  worldline_runtime --> maple_runtime
  worldline_runtime --> maple_kernel_family
  worldline_ledger --> maple_kernel_family
  worldline_governance --> maple_kernel_family
  worldline_substrate --> maple_worldline_family
```

## 2.4 Naming policy

- Use `worldline-*` crates for all new integration code and docs.
- Keep `maple-*` and `palm-*` crates as compatibility surfaces during migration.
- Treat compatibility crates as stable aliases, not as the long-term primary API.

## 2.5 Why this layout

- Reduces cognitive overhead: one canonical namespace for architecture-level concepts.
- Preserves backward compatibility for existing deployments.
- Keeps control-plane and data-plane boundaries explicit while enabling gradual migration.
