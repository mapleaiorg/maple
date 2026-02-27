# WorldLine Quickstart

This is the fastest end-to-end path for the canonical WorldLine stack (`worldline-*` crates, PALM daemon, and `maple` CLI).

## 1. Build

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple
cargo build
```

## 2. Validate Core WorldLine Crates

```bash
cargo test -p worldline-types -p worldline-identity -p worldline-core -p worldline-runtime -p worldline-ledger -p worldline-governance -p worldline-substrate
cargo test -p worldline-operator-bot -p worldline-promptkit
```

## 3. Run Conformance + Integration

```bash
cargo test -p worldline-conformance -p worldline-integration -p maple-worldline-conformance-suite -p maple-worldline-conformance
```

## 4. Start Daemon (Terminal A)

```bash
cargo run -p palm-daemon
```

API base: `http://localhost:8080/api/v1`

## 5. Run CLI (Terminal B)

```bash
cargo run -p maple-cli -- --help
cargo run -p maple-cli -- worldline create --profile financial --label treasury-a
cargo run -p maple-cli -- worldline create --profile financial --label treasury-b
cargo run -p maple-cli -- worldline list
cargo run -p maple-cli -- kernel status
cargo run -p maple-cli -- kernel metrics
```

## 6. Submit Commitment

```bash
cat >/tmp/worldline-commitment.json <<'JSON'
{
  "declaring_identity": "REPLACE_WITH_WORLDLINE_ID",
  "effect_domain": "financial",
  "targets": ["counterparty-1"],
  "capabilities": ["cap-financial-settle"],
  "evidence": ["operator-approved"]
}
JSON

cargo run -p maple-cli -- commit submit --file /tmp/worldline-commitment.json
```

Keep returned `commitment_id` and `decision_receipt_id`.

## 7. Settle + Query Projection

```bash
cat >/tmp/worldline-settlement.json <<'JSON'
{
  "commitment_id": "REPLACE_WITH_COMMITMENT_ID",
  "decision_receipt_id": "REPLACE_WITH_DECISION_RECEIPT_ID",
  "settlement_type": "dvp",
  "legs": [
    { "from": "REPLACE_WL_A", "to": "REPLACE_WL_B", "asset": "USD", "amount_minor": 150000 },
    { "from": "REPLACE_WL_B", "to": "REPLACE_WL_A", "asset": "BTC", "amount_minor": 1200 }
  ]
}
JSON

cargo run -p maple-cli -- financial settle --file /tmp/worldline-settlement.json
cargo run -p maple-cli -- financial projection REPLACE_WL_B USD
```

## 8. Provenance + Governance

```bash
cargo run -p maple-cli -- gov list
cargo run -p maple-cli -- provenance ancestors EVENT_ID --depth 5
cargo run -p maple-cli -- provenance worldline-history REPLACE_WORLDLINE_ID
```

## 9. Demo Programs

```bash
cargo run --manifest-path examples/mwl-worldline-lifecycle/Cargo.toml
cargo run --manifest-path examples/mwl-commitment-gate/Cargo.toml
cargo run --manifest-path examples/mwl-provenance-audit/Cargo.toml
cargo run --manifest-path examples/mwl-human-agency/Cargo.toml
cargo run --manifest-path examples/mwl-financial-settlement/Cargo.toml
```

## 10. Next

- [WorldLine Framework Guide](../worldline-framework.md)
- [Operations Tutorial](operations.md)
- [Maple Runtime Standalone Tutorial](maple-runtime-standalone.md)
