# MAP Protocol

The Multi-Agent Protocol (MAP) provides decentralized peer-to-peer communication between MAPLE nodes. It is implemented in the `map` crate using [`libp2p`](https://libp2p.io) and supports discovery and message broadcast.

## Features

- **Peer Discovery** – Uses mDNS so nodes on a local network can automatically find each other.
- **Encrypted Transport** – Connections use the `noise` protocol for authentication and encryption.
- **Multiplexing** – The `yamux` multiplexer allows multiple logical streams over a single TCP connection.
- **Command Channel** – Internally a Tokio `mpsc` channel drives the swarm event loop.
- **Broadcast Support** – Nodes can broadcast text or raw `.map` files to all peers.

## Example

```rust
use maple_map::{MapConfig, MapProtocol};

let config = MapConfig { listen_addr: "/ip4/0.0.0.0/tcp/0".to_string() };
let map = MapProtocol::new(config).await.unwrap();
map.broadcast("Hello, Mapleverse!".to_string()).await.unwrap();
```

## Extending

The current implementation prints events to stdout and leaves message routing as a TODO. Future work can integrate gossip protocols, persistent peer stores and custom message types for higher level services such as the Registry Service.

