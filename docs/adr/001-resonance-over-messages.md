# ADR 001: Resonance Over Messages

**Status**: Accepted

**Date**: 2026-01-15

**Decision Makers**: MAPLE Architecture Team

---

## Context

Traditional agent frameworks (including Google A2A and Anthropic MCP) use **message passing** as their fundamental communication primitive. Agents send messages to each other, process them, and send responses. This approach has several limitations:

1. **Stateless interactions**: Each message is independent, requiring full context transmission
2. **No relationship tracking**: Agents don't maintain awareness of ongoing relationships
3. **Ephemeral connections**: Connections exist only for message exchange
4. **No resource bounds**: Unlimited message sending can exhaust resources
5. **No accountability**: Messages are fire-and-forget with no attribution
6. **Binary presence**: Agents are either "online" or "offline"

These limitations become critical when building large-scale, long-running multi-agent systems that need:
- Meaningful, evolving relationships between agents
- Resource management and graceful degradation
- Accountability and audit trails
- Natural coordination at massive scale

## Decision

**We will use RESONANCE as the fundamental primitive instead of message passing.**

### What is Resonance?

Resonance is a **stateful, continuous relationship** between Resonators (intelligent entities). It encompasses:

1. **Presence**: Gradient, multidimensional availability (not binary)
2. **Coupling**: Stateful relationships that strengthen over time
3. **Meaning**: Semantic understanding that converges through interaction
4. **Intent**: Stabilized goals formed from sufficient meaning
5. **Commitment**: Explicit promises with audit trails
6. **Consequence**: Attributable outcomes from commitments

### Key Principles

**1. Relationships Over Transactions**

```
Traditional:
Agent A --[message]--> Agent B --[message]--> Agent C
(each message is independent)

MAPLE:
Resonator A <==[coupling]==> Resonator B <==[coupling]==> Resonator C
(relationships evolve over time)
```

**2. State Over Statelessness**

- Couplings maintain shared state
- Meaning convergence tracked over time
- Relationship history preserved
- Context accumulates naturally

**3. Gradual Over Instant**

- Coupling must strengthen gradually
- Meaning converges over time
- Intent stabilizes (not instant)
- Trust builds through interaction

**4. Bounded Over Unlimited**

- Finite attention limits relationships
- Graceful degradation under pressure
- Natural resource bounds
- Prevents exhaustion attacks

**5. Accountable Over Anonymous**

- Every consequential action requires commitment
- Full audit trails
- Attribution to specific Resonators
- Non-repudiation through signatures

## Consequences

### Positive

1. **Meaningful Relationships**
   - Agents develop rich, stateful relationships
   - Context accumulates naturally
   - Understanding deepens over time

2. **Natural Resource Management**
   - Attention economics bounds relationships
   - Graceful degradation built-in
   - Prevents resource exhaustion

3. **Complete Accountability**
   - Every action attributable
   - Full audit trails
   - Regulatory compliance support

4. **Scalability**
   - Causal ordering without global clocks
   - Natural parallelism (concurrent events)
   - Distributed by design

5. **Safety**
   - Architectural invariants enforced
   - Human agency protected
   - Coercion detection built-in

### Negative

1. **Higher Complexity**
   - More sophisticated than simple message passing
   - Requires understanding new concepts
   - Steeper learning curve

2. **Implementation Effort**
   - More complex runtime implementation
   - Requires careful design and testing
   - More state to manage

3. **Performance Overhead**
   - State management has overhead
   - More bookkeeping than stateless
   - Requires optimization

4. **Migration Difficulty**
   - Cannot easily migrate from message-based systems
   - Requires rethinking agent design
   - Breaking change from traditional approaches

### Mitigations

For complexity:
- Comprehensive documentation
- Many examples
- Clear conceptual guides
- Gradual learning path

For implementation effort:
- Modular design
- Comprehensive testing
- Clear abstractions
- Reusable components

For performance:
- Benchmark-driven optimization
- Lock-free data structures
- Async I/O throughout
- Careful profiling

For migration:
- Compatibility layers (future)
- Migration guides
- Gradual adoption path
- Interop with message systems

## Alternatives Considered

### Alternative 1: Enhanced Message Passing

**Description**: Keep message passing but add state tracking layers

**Pros**:
- Familiar to developers
- Easier migration
- Lower complexity

**Cons**:
- State management bolted on (not natural)
- Doesn't solve resource bounds problem
- No accountability built-in
- Scalability issues remain

**Why Rejected**: Doesn't address fundamental limitations; creates complexity without architectural benefits

### Alternative 2: Actor Model

**Description**: Use actor model (like Akka, Orleans)

**Pros**:
- Well-understood paradigm
- Mature implementations exist
- Good performance

**Cons**:
- Still message-based
- No relationship tracking
- No resource bounds
- No accountability
- Binary presence

**Why Rejected**: Same fundamental limitations as message passing; doesn't enable our vision

### Alternative 3: Hybrid Approach

**Description**: Support both messages and resonance

**Pros**:
- Flexibility
- Easier migration
- Backwards compatibility

**Cons**:
- Inconsistent programming model
- Complexity of supporting both
- Dilutes architectural vision
- Confusion for developers

**Why Rejected**: Architectural compromises undermine core benefits; better to commit fully

## Implementation

### Core Components

1. **PresenceFabric**: Manages gradient presence for all Resonators
2. **CouplingFabric**: Manages stateful relationships (topology)
3. **AttentionAllocator**: Enforces finite attention bounds
4. **TemporalCoordinator**: Causal ordering without clocks
5. **InvariantGuard**: Enforces architectural invariants

### API Design

```rust
// Resonator coupling (not message sending)
let coupling = resonator_a.couple_with(
    resonator_b.id,
    CouplingParams {
        initial_strength: 0.3,
        initial_attention_cost: 100.0,
        scope: CouplingScope::Full,
        // ...
    }
).await?;

// Coupling strengthens over time
coupling.strengthen(0.1).await?;

// Create commitment (not send message)
let commitment = resonator_a.create_commitment(
    CommitmentContent::Action(action)
).await?;

// Execute with accountability
commitment.activate().await?;
let result = resonator_a.execute_with_commitment(commitment).await?;
commitment.fulfill(result).await?;
```

### Migration Strategy

For projects moving from message-based systems:

1. **Identify stateful relationships**: Which "message channels" are really ongoing relationships?
2. **Model as couplings**: Convert to explicit couplings
3. **Add commitments**: Wrap consequential actions in commitments
4. **Track attention**: Identify resource bounds
5. **Implement gradually**: One subsystem at a time

## References

- MAPLE Architecture Overview: `docs/architecture.md`
- Coupling Documentation: `docs/concepts/coupling.md`
- Attention Economics: `docs/concepts/attention.md`
- Google A2A Comparison: `README.md`
- Anthropic MCP Comparison: `README.md`

## Approval

**Approved by**: MAPLE Architecture Team

**Date**: 2026-01-15

---

**This decision is foundational to MAPLE's architecture and should not be changed without extensive discussion.**
