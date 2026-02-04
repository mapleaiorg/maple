# Tutorial 03: gRPC Integration with grpcurl

## Objective

Use the typed gRPC API and reflection to call iBank.

## Step 1: Start Service with gRPC Enabled

```bash
cargo run --manifest-path ibank/Cargo.toml -p ibank-service -- --listen 127.0.0.1:8091 --grpc-listen 127.0.0.1:50051
```

## Step 2: Verify Reflection

```bash
grpcurl -plaintext 127.0.0.1:50051 list
```

You should see `ibank.v1.IBankService`.

## Step 3: Call Health

```bash
grpcurl -plaintext -d '{}' 127.0.0.1:50051 ibank.v1.IBankService/Health
```

## Step 4: Call Handle

```bash
grpcurl -plaintext -d '{
  "origin_actor":"issuer-a",
  "counterparty_actor":"merchant-b",
  "transaction_type":"transfer",
  "amount_minor":"50000",
  "currency":"USD",
  "rail":"ach",
  "destination":"acct-123",
  "jurisdiction":"US",
  "user_intent":"pay invoice 889",
  "ambiguity_hint":0.1,
  "counterparty_risk":10,
  "anomaly_score":8,
  "model_uncertainty":0.08,
  "compliance_flags":[],
  "metadata":{}
}' 127.0.0.1:50051 ibank.v1.IBankService/Handle
```

## Proto Contract and Generated Artifacts

- Proto: `ibank/crates/ibank-service/proto/ibank/v1/ibank.proto`
- Generated Rust stubs: `ibank/crates/ibank-service/src/generated/ibank.v1.rs`
- Descriptor set: `ibank/crates/ibank-service/src/generated/ibank_descriptor.bin`

Regenerate descriptor after proto changes:

```bash
ibank/crates/ibank-service/scripts/regenerate_descriptor.sh
```
