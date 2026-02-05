# ADR 004: Temporal Anchors and Causal Ordering

**Status**: Accepted

**Date**: 2026-01-15

**Decision Makers**: MAPLE Architecture Team

---

## Context

Distributed systems face fundamental challenges with time:

1. **No global clock**: Synchronized clocks are impossible to achieve
2. **Clock drift**: Local clocks diverge constantly
3. **Network delays**: Messages arrive at unpredictable times
4. **Ordering ambiguity**: Hard to determine event order across nodes
5. **Consensus overhead**: Agreeing on time is expensive

Traditional solutions:
- **NTP/PTP**: Clock synchronization (never perfect, ongoing overhead)
- **Logical clocks**: Lamport timestamps (limited expressiveness)
- **Consensus protocols**: Paxos, Raft (expensive, centralized)
- **Trusted time sources**: GPS, atomic clocks (not always available)

For MAPLE's goal of 100M+ Resonators across distributed nodes, we need an approach that:
- Scales without central coordination
- Works with only local clocks
- Expresses causal relationships clearly
- Supports audit trail construction
- Enables happened-before reasoning

## Decision

**We will use TEMPORAL ANCHORS with CAUSAL ORDERING instead of synchronized global clocks.**

### What are Temporal Anchors?

**Temporal anchors** are reference points in a Resonator's local timeline:

```rust
pub struct TemporalAnchor {
    pub id: AnchorId,
    pub event: Event,
    pub local_timestamp: i64,           // Monotonic local clock
    pub dependencies: Vec<AnchorId>,    // Causal dependencies
    pub resonator: ResonatorId,
    pub metadata: HashMap<String, Value>,
}
```

### Key Principles

1. **Causal Over Absolute**: What matters is "A happened before B", not exact timestamps
2. **Local Over Global**: Each Resonator has its own timeline
3. **Explicit Dependencies**: Causal relationships expressed explicitly
4. **Happened-Before**: Partial ordering based on causality

### Causal Ordering

```
a → b  (a happened-before b) if:
  1. a and b in same timeline AND a.timestamp < b.timestamp
  OR
  2. b depends on a (explicit dependency)
  OR
  3. ∃c such that a → c AND c → b (transitivity)

a ∥ b  (concurrent) if:
  NOT (a → b) AND NOT (b → a)
```

## Rationale

### Why Not Global Clocks?

**Problems with global clocks**:
- Require expensive synchronization
- Never perfectly synchronized
- Don't scale to massive distributed systems
- Add latency to every operation
- Single point of failure (time source)

**Benefits of causal ordering**:
- No synchronization required
- Purely local operations
- Natural scalability
- No coordination bottleneck
- More distributed-friendly

### Why Causality Matters

**What we care about**:
- Did commitment A complete before B started?
- Can action B have been caused by action A?
- What's the causal chain leading to this consequence?

**What we don't care about**:
- Exact wall-clock time of each event
- Precise time differences
- Synchronized timestamps

**Causality is sufficient** for:
- Audit trail construction
- Commitment ordering
- Consequence attribution
- Event sequencing

### Why Temporal Anchors?

**Advantages**:
- Locally generated (no coordination)
- Causally ordered (dependencies explicit)
- Portable (can be transmitted)
- Composable (can reference others)
- Immutable (once created)

**Vs. Alternatives**:
- **Logical clocks**: Less expressive, harder to query
- **Vector clocks**: Good but hidden in implementation
- **Hybrid logical clocks**: Complex, still needs sync
- **Temporal anchors**: Explicit, expressive, distributed-friendly

## Consequences

### Positive

1. **Scales Naturally**
   - No global coordination needed
   - Purely local operations
   - Horizontal scaling unlimited
   - No coordination bottleneck

2. **Causality Explicit**
   - Dependencies clearly expressed
   - Happened-before computable
   - Causal chains traceable
   - Audit trails constructible

3. **Distributed-Friendly**
   - Works with network partitions
   - No consensus required
   - Local clocks sufficient
   - Natural parallelism

4. **Immutable History**
   - Anchors never change
   - History preserved
   - Audit trails reliable
   - No time travel paradoxes

5. **Query Flexibility**
   - Find causal history
   - Detect concurrent events
   - Topological sorting
   - Dependency analysis

### Negative

1. **Partial Ordering**
   - Can't totally order all events
   - Concurrent events ambiguous
   - May need conflict resolution
   - Not all queries possible

2. **Storage Overhead**
   - Anchors stored indefinitely (audit)
   - Dependencies take space
   - Growth over time
   - Compaction needed

3. **Query Complexity**
   - Happened-before requires graph traversal
   - Causal history can be large
   - More complex than timestamp comparison
   - Optimization needed

4. **No Absolute Time**
   - Can't answer "what happened at 3pm"
   - No wall-clock ordering
   - Requires different mental model
   - Learning curve

### Mitigations

For partial ordering:
- Conflict resolution strategies
- Concurrent event handling
- CRDT-style merging
- Application-level policies

For storage:
- Anchor compaction (old anchors)
- Retention policies (7 years for iBank)
- Efficient storage formats
- Archive old anchors

For query complexity:
- Caching of causal relationships
- Indexed dependency graphs
- Query optimization
- Materialized views

For mental model:
- Clear documentation
- Many examples
- Visual diagrams
- Conceptual guides

## Alternatives Considered

### Alternative 1: Global Clock Synchronization (NTP)

**Why Rejected**: Expensive, never perfect, doesn't scale, adds latency, single point of failure

### Alternative 2: Consensus-Based Time (Paxos/Raft)

**Why Rejected**: Too expensive, centralized, doesn't scale to 100M+ agents, adds latency

### Alternative 3: Trusted Time Sources (GPS, Atomic Clocks)

**Why Rejected**: Not always available, doesn't solve fundamental problem, expensive infrastructure

### Alternative 4: Hybrid Logical Clocks (HLC)

**Why Rejected**: More complex than needed, still requires some synchronization, less explicit

### Alternative 5: Pure Vector Clocks

**Pros**: Mathematically clean, well-understood
**Cons**: Less expressive than anchors, hidden in implementation, harder to query

**Why Temporal Anchors Chosen**: More expressive (events named), explicit (dependencies visible), easier to query, better for audit trails

## Implementation

### TemporalCoordinator

```rust
pub struct TemporalCoordinator {
    anchors: DashMap<ResonatorId, Vec<TemporalAnchor>>,
    causal_graph: DashMap<AnchorId, Vec<AnchorId>>,
    config: TemporalConfig,
}

impl TemporalCoordinator {
    pub async fn create_anchor(
        &self,
        resonator: ResonatorId,
        event: Event,
        dependencies: Vec<AnchorId>
    ) -> Result<TemporalAnchor> {
        // Validate dependencies exist
        // Create anchor with monotonic timestamp
        // Add to causal graph
        // Return anchor
    }

    pub async fn happened_before(
        &self,
        a: AnchorId,
        b: AnchorId
    ) -> Result<bool> {
        // Traverse causal graph
        // Check if a is ancestor of b
    }

    pub async fn causal_history(
        &self,
        anchor: AnchorId
    ) -> Result<Vec<TemporalAnchor>> {
        // Get all causal predecessors
        // Return in topological order
    }
}
```

### Vector Clock Implementation (Internal)

Internally, we use vector clocks for efficient happened-before checks:

```rust
pub struct VectorClock {
    clocks: HashMap<ResonatorId, u64>,
}

impl VectorClock {
    pub fn happened_before(&self, other: &VectorClock) -> bool {
        // Check if self < other
    }

    pub fn concurrent(&self, other: &VectorClock) -> bool {
        // Check if neither < the other
    }
}
```

### Creating Anchors with Dependencies

```rust
// Commitment B depends on Commitment A being fulfilled
let anchor_a = commitment_a.fulfillment_anchor().await?;

let anchor_b = resonator.create_anchor_with_deps(
    Event::CommitmentActivated,
    vec![anchor_a.id]  // Explicit dependency
).await?;

// System ensures A completed before B started
```

## Integration with Commitments

**Critical for audit trails**:

```rust
// Every commitment creates anchors
let commitment = resonator.create_commitment(content).await?;
// Creates anchor for commitment creation

commitment.activate().await?;
// Creates anchor for activation (depends on creation anchor)

commitment.fulfill(result).await?;
// Creates anchor for fulfillment (depends on activation anchor)

// Full causal chain automatically tracked
let chain = coordinator.causal_history(
    commitment.fulfillment_anchor_id
).await?;

// Chain shows: creation → activation → fulfillment
```

## Platform-Specific Configuration

**Mapleverse** (Pure AI):
- Anchor retention: 30 days
- Compact old anchors: true
- Max dependencies: 10

**Finalverse** (Human-AI):
- Anchor retention: 90 days (longer for humans)
- Compact old anchors: true
- Human anchor priority: true

**iBank** (Finance):
- Anchor retention: 7 years (regulatory)
- Compact old anchors: false (immutable)
- Max dependencies: 20

## Monitoring

### Metrics

- Anchor creation rate
- Average causal depth
- Concurrent event ratio
- Causal query latency
- Storage growth

### Insights

- High concurrency → Good parallelism
- Deep causal chains → Long dependencies
- Query latency → Need optimization
- Storage growth → Time for compaction

## Future Enhancements

1. **Persistent temporal store**: Durable anchor storage
2. **Temporal queries**: Rich query language for causal relationships
3. **Time travel debugging**: Replay causal history
4. **Causal consistency**: Distributed consistency guarantees
5. **Temporal analytics**: Analyze causal patterns

## References

- Temporal Coordination Documentation: `docs/concepts/temporal.md`
- Architecture Overview: `docs/architecture.md`
- Implementation: `crates/maple-runtime/src/temporal/`
- Lamport's Time, Clocks paper
- Vector Clocks papers

## Approval

**Approved by**: MAPLE Architecture Team

**Date**: 2026-01-15

---

**This decision enables MAPLE's scalability and audit capabilities without global clock synchronization.**
