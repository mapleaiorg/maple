# WorldLine Quickstart

This tutorial is the fastest path to run Maple WorldLine end-to-end from CLI, REST API, and demos.

## 1. Build

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple
cargo build
```

## 2. Validate WorldLine Crates

```bash
cargo test -p worldline-types -p worldline-identity -p worldline-core -p worldline-runtime -p worldline-ledger -p worldline-governance -p worldline-substrate
```

## 3. Run Core WorldLine Verification

```bash
cargo test -p maple-mwl-conformance -p maple-mwl-integration -p maple-worldline-conformance
```

## 4. Start the Daemon (Terminal A)

```bash
cargo run -p palm-daemon
```

WorldLine API is available at `http://localhost:8080/api/v1`.

## 5. Use WorldLine CLI Commands (Terminal B)

List available command groups:

```bash
cargo run -p maple-cli -- --help
```

Create two worldlines:

```bash
cargo run -p maple-cli -- worldline create --profile financial --label treasury-a
cargo run -p maple-cli -- worldline create --profile financial --label treasury-b
cargo run -p maple-cli -- worldline list
```

Check kernel status and metrics:

```bash
cargo run -p maple-cli -- kernel status
cargo run -p maple-cli -- kernel metrics
```

## 6. Submit a Commitment

```bash
cat >/tmp/worldline-commitment.json <<'JSON'
{
  "declaring_identity": "REPLACE_WITH_WORLDLINE_ID",
  "effect_domain": "financial",
  "targets": ["counterparty-1"],
  "capabilities": ["CAP-FIN"],
  "evidence": ["operator-approved"]
}
JSON
```

```bash
cargo run -p maple-cli -- commit submit --file /tmp/worldline-commitment.json
```

## 7. Submit a Settlement + Check Projection

```bash
cat >/tmp/worldline-settlement.json <<'JSON'
{
  "settlement_type": "dvp",
  "legs": [
    { "from": "REPLACE_WL_A", "to": "REPLACE_WL_B", "asset": "USD", "amount_minor": 150000 },
    { "from": "REPLACE_WL_B", "to": "REPLACE_WL_A", "asset": "BTC", "amount_minor": 1200 }
  ]
}
JSON
```

```bash
cargo run -p maple-cli -- financial settle --file /tmp/worldline-settlement.json
cargo run -p maple-cli -- financial projection REPLACE_WL_B USD
```

## 8. Query Provenance and Policies

```bash
cargo run -p maple-cli -- policy list
```

```bash
cat >/tmp/worldline-policy.json <<'JSON'
{
  "effect_domain": "financial"
}
JSON
```

```bash
cargo run -p maple-cli -- policy simulate --file /tmp/worldline-policy.json
```

If you have an event ID from commitment/settlement activity:

```bash
cargo run -p maple-cli -- provenance ancestors EVENT_ID --depth 5
cargo run -p maple-cli -- provenance worldline-history REPLACE_WORLDLINE_ID
```

## 9. Run Demonstration Programs

```bash
cargo run --manifest-path examples/mwl-worldline-lifecycle/Cargo.toml
cargo run --manifest-path examples/mwl-commitment-gate/Cargo.toml
cargo run --manifest-path examples/mwl-provenance-audit/Cargo.toml
cargo run --manifest-path examples/mwl-human-agency/Cargo.toml
cargo run --manifest-path examples/mwl-financial-settlement/Cargo.toml
```

All five demos are wired to canonical `worldline-*` crates while preserving
backward compatibility for legacy `maple-*` crates.

## 10. Use WorldLine Crates in Your Own App

```toml
[dependencies]
worldline-core = "0.1.2"
worldline-runtime = "0.1.2"
worldline-ledger = "0.1.2"
```

```rust
use worldline_core::types::WorldlineId;
use worldline_runtime::gate::CommitmentGate;
use worldline_ledger::provenance::ProvenanceIndex;
```

## 11. Optional: Direct REST Calls

```bash
curl -s http://localhost:8080/api/v1/worldlines
curl -s http://localhost:8080/api/v1/kernel/status
curl -s http://localhost:8080/api/v1/kernel/metrics
```

## 12. Next

- Framework map: [WorldLine Framework Guide](../worldline-framework.md)
- Demo catalog: [Examples and Demos](../../examples/README.md)
