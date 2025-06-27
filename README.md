# Maple

Python-based AI Multi-Agent Framework

Maple is a modular, Python-based platform for building, registering, orchestrating, and evolving autonomous agents over a decentralized, event-driven network.

## Modules Overview

### core.map (MAP Protocol)
- Messaging models, routing engine, transport layer (HTTP/WebSocket), workflow orchestration, protocol server, security, and middleware.

### core.ars (Agent Registry Service)
- REST & gRPC servers, client SDK, service discovery, capability indexing, storage backends, and health monitoring.

### core.ual (Universal Agent Language)
- Lexer, parser, AST, semantic analyzer, code generator, compiler, and CLI for defining and compiling agent behaviors.

### llm (LLM Integration)
- Unified base interface and factory, integrations for cloud providers (OpenAI, Anthropic, Google) and local models (LLaMA, Mistral), with fallback, caching, streaming, and multimodal support.

### mall (Maple Agent Learning Lab)
- Federated learning, reinforcement learning, auto-spawning, privacy/security, strategy generation, and client/server SDK.

## Examples

- UAL agent definition example: `examples/ual/research_agent.ual`

## Documentation

- Architecture overview: `docs/architecture.md`
- Detailed design: `docs/MAPLE-AIFramework-Design.md`

## Contributing

Contributions welcome! Please open issues or pull requests to discuss improvements.
