# Universal Agent Language (UAL)

UAL defines a common message format for communication between MAPLE agents and services. It lives in the `ual` crate and currently supports three modes.

## Modes

- **Json** – Payload is encoded as JSON. Useful for quick prototyping and interoperability.
- **Grpc** – Structured binary format built with `prost` (not yet implemented).
- **ByteLevel** – Raw byte payload for maximum efficiency or custom encodings.

Each `UalMessage` contains an action string (e.g., `"move"`) and a payload. Encoding and decoding helpers ensure the payload matches the selected mode.

```rust
use maple_ual::{UalMessage, Mode};

let msg = UalMessage::new("move", Mode::Json)
    .with_json_payload(&serde_json::json!({"x": 10, "y": 20}))?;
let payload: serde_json::Value = msg.decode()?;
```

## Design Goals

1. **Flexibility** – Support multiple encoding schemes so agents written in different languages can interoperate.
2. **Transport Agnostic** – UAL messages can be sent over MAP, HTTP, gRPC or embedded in `.map` files.
3. **Extensibility** – Additional modes (e.g., Cap'n Proto) can be added without breaking existing agents.

Future development will implement full gRPC mode and richer schemas for complex agent interactions.

