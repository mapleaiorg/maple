# Tutorial 04: Add a Custom Settlement Connector

## Objective

Add a new rail adapter while preserving iBank invariants.

## Step 1: Implement Connector

Create a connector in your crate (or extend `ibank-adapters`):

```rust
use chrono::Utc;
use ibank_core::{IBankError, SettlementConnector};
use ibank_core::types::{AccountableWireMessage, ConnectorReceipt};
use std::collections::BTreeMap;

pub struct WireConnector;

impl SettlementConnector for WireConnector {
    fn rail(&self) -> &'static str {
        "wire"
    }

    fn execute(&self, message: &AccountableWireMessage) -> Result<ConnectorReceipt, IBankError> {
        let mut metadata = BTreeMap::new();
        metadata.insert("trace_id".to_string(), message.trace_id.clone());

        Ok(ConnectorReceipt {
            settlement_id: format!("wire-{}", message.message_id),
            rail: "wire".to_string(),
            settled_at: Utc::now(),
            metadata,
        })
    }
}
```

## Step 2: Register Connector at Bootstrap

```rust
use std::sync::Arc;

engine.register_connector(Arc::new(WireConnector))?;
```

## Step 3: Route Requests to New Rail

Set `"rail": "wire"` in `HandleRequest` payload.

## Step 4: Validate Invariants Still Hold

Even with a custom connector, iBank still requires:

- commitment before consequential route
- accountable wire verification
- deterministic risk pass
- explicit outcome record on success/failure

The connector only handles side-effect execution, not policy/invariant decisions.
