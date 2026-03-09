# Rust SDK

Install:

```bash
cargo add maple-sdk
```

The Rust SDK is the closest fit for teams building deeply integrated runtimes or strongly typed operator services around MAPLE.

## Example

```rust
use maple_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<(), maple_sdk::Error> {
    let client = MapleClient::connect("http://localhost:8080").await?;

    let agent = client.worldline()
        .create(Profile::Agent, "my-support-agent")
        .await?;

    let result = client.commit()
        .declare(agent.id())
        .with_obligation("resolve customer ticket #1234")
        .with_capability("zendesk.ticket.reply")
        .submit()
        .await?;

    println!("{result:?}");
    Ok(())
}
```

## Typical flow

1. Connect a client to the daemon or gateway.
2. Create or load a worldline identity.
3. Draft and submit a commitment.
4. Handle authorization, denial, or hold.
5. Query provenance and receipts.
