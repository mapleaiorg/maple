# WorldLine Quickstart

This tutorial is the shortest end-to-end path through the current MAPLE implementation: build the repo, start the PALM daemon, create worldlines, submit a commitment, and inspect provenance.

## 1. Build the workspace

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple
cargo build
```

## 2. Start the daemon

```bash
cargo run -p palm-daemon
```

Default API base: `http://localhost:8080/api/v1`

## 3. Open a second terminal for CLI commands

```bash
cargo run -p maple-cli -- --help
cargo run -p maple-cli -- worldline create --profile financial --label treasury-a
cargo run -p maple-cli -- worldline create --profile financial --label treasury-b
cargo run -p maple-cli -- worldline list
```

## 4. Submit a commitment

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

Capture the returned `commitment_id` and `decision_receipt_id`.

## 5. Execute a financial consequence

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
```

## 6. Inspect provenance and kernel state

```bash
cargo run -p maple-cli -- kernel status
cargo run -p maple-cli -- kernel metrics
cargo run -p maple-cli -- provenance worldline-history REPLACE_WORLDLINE_ID
cargo run -p maple-cli -- gov list
```

## 7. What you just exercised

- worldline identity creation
- commitment submission and decision receipts
- a consequence that required explicit authorization
- kernel and provenance visibility through the current CLI

## Next

- [Operations Tutorial](operations.md)
- [iBank Commitment Boundary](ibank-commitment-boundary.md)
- [REST API](../api/rest-api.md)
