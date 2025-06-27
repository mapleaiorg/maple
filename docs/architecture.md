# MAPLE Architecture

MAPLE (Multi-Agent Protocol Learning Environment) is organized as a set of Rust crates forming a workspace. The platform enables creation, registration and evolution of AI agents that communicate over a peer-to-peer network. Major layers include networking, registry, storage, runtime and developer tooling.

## Workspace Layout

- **agents/** – Agent implementations with utilities to dump and spawn `.map` DNA files.
- **api/** – REST API exposing runtime capabilities with JWT based authentication.
- **cli/** – Command line interface built with `clap` for local interaction.
- **config/** – Common configuration types.
- **core/** – Governance and language models. Contains an internal Maple LLM and adapters for external models.
- **mall/** – Maple Agent Learning Lab for agent evolution. Includes `mpy/` for Python based agents.
- **map/** – MAP protocol built on `libp2p` for decentralized messaging.
- **mrs/** – MAPLE Registry Service managing agent DIDs and metadata.
- **runtime/** – Node runtime managing agents and networking (distributed or enterprise modes).
- **sdk/** – Rust and Python SDKs for integrating MAPLE into other projects.
- **storage/** – Data backends (MapleDB key-value store, PostgreSQL, Vector DB placeholder).
- **ual/** – Universal Agent Language used by agents for communication.
- **utils/** – Workspace utilities and helpers.

## Data Flow

1. **Agent Creation** – Agents are defined in `agents/` with a configuration and optional state. They can dump their data into a `.map` DNA file for portability.
2. **Registration (MRS)** – The `.map` files or agent configs are registered with the Registry Service (`mrs/`). Agents receive a decentralized identifier (DID) for lookup.
3. **Network (MAP)** – Nodes communicate via the MAP protocol (`map/`). It uses libp2p with mDNS discovery, noise encryption and yamux multiplexing. Messages or entire `.map` files can be broadcast.
4. **Runtime** – A runtime instance (`runtime/`) coordinates agents, the registry and storage. It uses MapleDB and optionally PostgreSQL or a vector database for persistence. Agents can be spawned from DIDs or `.map` files and interact over MAP.
5. **Learning (MALL)** – The learning lab (`mall/`) runs simulations or tasks where agents, including Python agents via `mpy/`, are trained using UAL messages.
6. **API & CLI** – The REST API (`api/`) exposes runtime functions such as spawning agents. The CLI (`cli/`) provides local commands for developers. Both rely on the SDK crate.
7. **SDK** – `sdk/` provides a programmatic interface in Rust and Python, wrapping MAP, MRS, and runtime interactions.

## Storage

- **MapleDB** (`storage/mapledb`) – Embedded key-value store backed by `sled` for agent state and metadata.
- **PostgreSQL** (`storage/pg`) – Structured storage for agent data using `sqlx` (requires external database).
- **VectorDB** (`storage/vectordb`) – Placeholder integration for vector databases (e.g., Qdrant) used for embeddings or memory retrieval.

## Core LLMs

The `core/` crate groups language model logic.

- **maple/llm/** – External LLM integration with a simple `generate` function (currently a stub). Designed to connect to systems like Llama.cpp or Mistral.
- **maple/maple/** – Internal Maple language model for governance and conflict resolution among agents.

## Extensibility

The workspace design allows new crates to be added easily. Each crate exposes a clear API and tests. Developers can build custom agents, integrate new storage backends or extend the runtime.

