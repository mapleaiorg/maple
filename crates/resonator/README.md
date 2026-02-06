# Resonator Layer (`crates/resonator/*`)

Resonator is the cognition/lifecycle layer for MAPLE entities, implementing the full Resonance Architecture pipeline from presence through consequence.

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                        RESONATOR LAYER                                │
├──────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐          │
│  │ Identity │──→│ Presence │──→│ Coupling │──→│ Meaning  │          │
│  └──────────┘   └──────────┘   └──────────┘   └──────────┘          │
│                                                      │                │
│                                                      ▼                │
│  ┌──────────────┐   ┌────────────┐   ┌──────────┐   ┌──────────┐    │
│  │ Consequence  │←──│ Commitment │←──│  Intent  │←──│          │    │
│  └──────────────┘   └────────────┘   └──────────┘   └──────────┘    │
│         │                 │                                          │
│         ▼                 ▼                                          │
│  ┌──────────────┐   ┌────────────┐                                  │
│  │    Memory    │   │   Audit    │                                  │
│  └──────────────┘   └────────────┘                                  │
│                                                                       │
├──────────────────────────────────────────────────────────────────────┤
│  CLI │ Observability │ Conformance │ Profiles │ Conversation        │
└──────────────────────────────────────────────────────────────────────┘
```

## Components

### Core Pipeline

| Crate | Description |
|-------|-------------|
| **[types](types/)** | Core Resonator types: identity, presence, coupling |
| **[identity](identity/)** | Persistent identity and continuity primitives |
| **[meaning](meaning/)** | Meaning formation engine with semantic understanding |
| **[intent](intent/)** | Intent stabilization from converged meaning |
| **[commitment](commitment/)** | Contract lifecycle and commitment management |
| **[consequence](consequence/)** | Consequence tracking and attribution |

### Memory & Conversation

| Crate | Description |
|-------|-------------|
| **[memory](memory/)** | Multi-tier memory system (short-term, working, long-term, episodic) |
| **[conversation](conversation/)** | Multi-turn conversation management |

### Operations

| Crate | Description |
|-------|-------------|
| **[cli](cli/)** | Command-line interface for Resonator management |
| **[observability](observability/)** | Metrics, tracing, and alerting infrastructure |
| **[conformance](conformance/)** | Test suite for runtime invariant verification |
| **[profiles](profiles/)** | Profile constraints (Human, World, Coordination, IBank) |
| **[runtime](runtime/)** | Runtime coordination for Resonator flows |
| **[client](client/)** | Client-facing helpers and SDKs |

## The 8 Runtime Invariants

Every Resonator implementation must enforce these architectural invariants:

1. **Presence precedes Coupling** - Must establish presence before forming relationships
2. **Coupling precedes Meaning** - Meaning only forms within established couplings
3. **Meaning precedes Intent** - Intent requires sufficient meaning convergence
4. **Commitment precedes Consequence** - No consequence without explicit commitment
5. **Receipts are Immutable** - Commitment receipts cannot be modified
6. **Audit trail is Append-Only** - Audit entries can only be added
7. **Capabilities gate Actions** - Actions require explicit capability grants
8. **Time anchors are Monotonic** - Temporal anchors always increase

## Quick Start

### Basic Resonator Usage

```rust
use maple_runtime::{MapleRuntime, ResonatorSpec, config::RuntimeConfig};
use resonator_meaning::MeaningFormationEngine;
use resonator_intent::IntentStabilizationEngine;
use resonator_commitment::ContractEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bootstrap runtime
    let config = RuntimeConfig::default();
    let runtime = MapleRuntime::bootstrap(config).await?;

    // Register a Resonator
    let resonator = runtime.register_resonator(ResonatorSpec::default()).await?;

    // Use the cognitive pipeline
    let meaning_engine = MeaningFormationEngine::new();
    let intent_engine = IntentStabilizationEngine::new();
    let contract_engine = ContractEngine::new();

    // Form meaning from input
    let meaning = meaning_engine.form_meaning(&input).await?;

    // Stabilize intent from meaning
    let intent = intent_engine.stabilize(&meaning).await?;

    // Create commitment from intent
    let commitment = contract_engine.create_commitment(intent).await?;

    Ok(())
}
```

### Using the CLI

```bash
# Show architectural invariants
resonator invariants

# Show resonance pipeline
resonator pipeline

# List commitments
resonator commitment list

# Show commitment lifecycle
resonator commitment lifecycle

# Track consequences
resonator consequence list
```

### Observability

```rust
use resonator_observability::{MetricsCollector, SpanTracker, AlertEngine};

let metrics = MetricsCollector::new();
let spans = SpanTracker::new();
let alerts = AlertEngine::new();

// Track pipeline metrics
metrics.increment_counter("pipeline.commitment.created");
metrics.record_histogram("pipeline.meaning.formation_ms", 45.0);

// Create spans for tracing
let span = spans.start_span("commitment.validate", None);
// ... do work ...
spans.end_span(&span);
```

### Conformance Testing

```rust
use resonator_conformance::{ConformanceSuite, Invariant};

let suite = ConformanceSuite::new();

// Test specific invariant
let result = suite.test_invariant(Invariant::CommitmentPrecedesConsequence).await?;
assert!(result.passed);

// Run all tests
let report = suite.run_all().await?;
assert!(report.all_passed());
```

## Memory System

The multi-tier memory system provides:

- **Short-term Memory**: Quick access, limited capacity, auto-expiring
- **Working Memory**: Active processing, moderate capacity
- **Long-term Memory**: Persistent storage, large capacity
- **Episodic Memory**: Experience sequences with emotional context

```rust
use resonator_memory::{MemorySystem, MemoryEntry, MemoryTier};

let memory = MemorySystem::new();

// Store in appropriate tier
memory.store(MemoryEntry::new(
    "interaction_123",
    content,
    MemoryTier::Working,
)).await?;

// Retrieve with context
let memories = memory.retrieve_relevant(&context, 10).await?;

// Consolidate to long-term
memory.consolidate().await?;
```

## Conversation Management

Handle multi-turn conversations with state management:

```rust
use resonator_conversation::{ConversationManager, Turn};

let manager = ConversationManager::new();

// Start conversation
let conversation = manager.start_conversation(participant_ids).await?;

// Add turns
conversation.add_turn(Turn::new(
    speaker_id,
    "Hello, I'd like to discuss the project",
)).await?;

// Get conversation state
let state = conversation.get_state();
println!("Turns: {}, Active: {}", state.turn_count, state.is_active);
```

## Protocol Adapters

### MCP (Model Context Protocol)

```rust
use maple_protocol_mcp::{McpAdapter, McpTool, McpResource};

let adapter = McpAdapter::new();

// Register tools with commitment tracking
adapter.register_tool(McpTool::new(
    "search",
    "Search the knowledge base",
    |params| async { /* implementation */ },
));

// All tool invocations create auditable commitments
let result = adapter.invoke_tool("search", params).await?;
```

### A2A (Agent-to-Agent)

```rust
use maple_protocol_a2a::{A2aAdapter, A2aTask, A2aMessage};

let adapter = A2aAdapter::new();

// Handle tasks with consequence tracking
adapter.on_task(|task| async {
    // Tasks automatically create commitments
    // Completions are tracked as consequences
    task.complete(result).await
});
```

## Best Practices

### Invariant Compliance

Always follow the pipeline order:
1. Establish presence first
2. Form couplings before exchanging meaning
3. Wait for sufficient meaning before stabilizing intent
4. Never create consequences without commitments

### Memory Management

- Use working memory for active processing
- Consolidate to long-term memory periodically
- Attach emotional context to episodic memories
- Set appropriate retention policies

### Observability

- Track all pipeline stage transitions
- Create spans for significant operations
- Set up alerts for invariant violations
- Export telemetry for monitoring

## See Also

- [MAPLE Runtime](../maple-runtime/README.md)
- [Resonance Commitment Format (RCF)](../rcf-types/README.md)
- [Authority & Accountability Service (AAS)](../aas-types/README.md)
- [CLI Documentation](cli/README.md)
- [Observability Guide](observability/README.md)
- [Conformance Tests](conformance/README.md)
