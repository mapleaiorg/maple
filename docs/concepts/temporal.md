# Temporal Coordination and Causal Ordering

## Overview

MAPLE uses **temporal anchors** and **causal ordering** instead of global synchronized clocks. This approach enables distributed coordination without consensus, supports natural causal relationships, and scales to massive distributed deployments.

## Core Concept

In MAPLE, time is **relational, not absolute**. Events are ordered by their causal dependencies (happened-before relationships) rather than global timestamps.

```
Traditional Systems:   Global Clock → Synchronized Timestamps → Total Ordering

MAPLE:                Local Clocks → Causal Dependencies → Partial Ordering
```

## Why No Global Clocks?

### 1. Distributed Systems Don't Have Global Time

**The problem:**
- Clock synchronization is expensive
- Clocks drift constantly
- Network delays introduce uncertainty
- No truly global "now" exists

**MAPLE's solution:**
- Local clocks only
- Causal dependencies explicit
- No synchronization required
- Natural distributed operation

### 2. Causality Is What Matters

**What we actually care about:**
- Did event A happen before event B?
- Can event B have been caused by event A?
- What is the causal chain of events?

**Not what we care about:**
- Exact timestamp of event A
- Absolute time difference
- Synchronized wall-clock time

### 3. Scalability

**Global clocks don't scale:**
- Require consensus protocols
- Create coordination bottlenecks
- Add latency to every operation
- Limit horizontal scaling

**Causal ordering scales:**
- No global coordination needed
- Purely local operations
- Natural parallelism
- Unlimited horizontal scaling

## Temporal Anchors

A **temporal anchor** is a reference point in a Resonator's local timeline:

```rust
pub struct TemporalAnchor {
    /// Unique identifier
    pub id: AnchorId,

    /// Event this anchor represents
    pub event: Event,

    /// Local timestamp (monotonic)
    pub local_timestamp: i64,

    /// Causal dependencies (anchors that must precede this)
    pub dependencies: Vec<AnchorId>,

    /// Resonator that created this anchor
    pub resonator: ResonatorId,

    /// Optional metadata
    pub metadata: HashMap<String, Value>,
}
```

### Key Properties

1. **Locally generated**: No coordination required
2. **Causally ordered**: Dependencies make happened-before explicit
3. **Portable**: Can be transmitted and understood elsewhere
4. **Composable**: Can reference anchors from other Resonators

## Creating Temporal Anchors

### Basic Anchor Creation

```rust
// Create anchor for an event
let anchor = resonator.create_anchor(
    Event::PresenceSignaled
).await?;

println!("Anchor created: {}", anchor.id);
println!("Local timestamp: {}", anchor.local_timestamp);
```

### Anchor with Dependencies

```rust
// This event depends on previous events
let anchor = resonator.create_anchor_with_deps(
    Event::CommitmentFulfilled,
    vec![anchor_1, anchor_2]  // Dependencies
).await?;

// Happened-before: anchor_1 → this_anchor
// Happened-before: anchor_2 → this_anchor
```

### Cross-Resonator Dependencies

```rust
// Alice's anchor
let alice_anchor = alice.create_anchor(
    Event::IntentFormed
).await?;

// Bob's anchor depends on Alice's
let bob_anchor = bob.create_anchor_with_deps(
    Event::CommitmentMade,
    vec![alice_anchor.id]  // Bob's event causally depends on Alice's
).await?;
```

## Causal Ordering

### Happened-Before Relation

The happened-before relation (→) is defined as:

```
a → b  if and only if:
  1. a and b are in the same Resonator timeline AND a.timestamp < b.timestamp
  OR
  2. b's dependencies include a
  OR
  3. There exists c such that a → c AND c → b (transitivity)
```

### Concurrent Events

Events are **concurrent** (a ∥ b) if:
```
NOT (a → b) AND NOT (b → a)
```

Concurrent events:
- Have no causal relationship
- Can be processed in any order
- Enable parallelism
- Are the common case in distributed systems

### Example Causal Graph

```
Resonator A:     a1 ──→ a2 ──→ a3 ──→ a4
                  ↓             ↓
Resonator B:     b1 ──→ b2     b3 ──→ b4
                        ↓       ↓
Resonator C:           c1 ──→ c2 ──→ c3

Causal dependencies:
  a1 → a2 → a3 → a4
  a1 → b1 → b2
  a3 → b3 → b4
  b2 → c1 → c2 → c3
  b3 → c2

Concurrent pairs:
  a2 ∥ b1
  a3 ∥ c1
  a4 ∥ b4
  b2 ∥ b3
```

## Temporal Coordinator

The `TemporalCoordinator` manages temporal anchors and causal ordering:

```rust
pub struct TemporalCoordinator {
    /// Anchors by Resonator
    anchors: DashMap<ResonatorId, Vec<TemporalAnchor>>,

    /// Causal dependency graph
    causal_graph: DashMap<AnchorId, Vec<AnchorId>>,

    /// Configuration
    config: TemporalConfig,
}
```

### Key Operations

#### Query Happened-Before

```rust
let happened_before = coordinator.happened_before(
    anchor_a,
    anchor_b
).await?;

if happened_before {
    println!("Event A happened before Event B");
} else if coordinator.happened_before(anchor_b, anchor_a).await? {
    println!("Event B happened before Event A");
} else {
    println!("Events A and B are concurrent");
}
```

#### Get Causal History

```rust
// Get all anchors that causally precede this one
let history = coordinator.causal_history(anchor_id).await?;

for ancestor in history {
    println!("Causal predecessor: {}", ancestor.id);
}
```

#### Find Common Ancestor

```rust
// Find most recent common ancestor
let lca = coordinator.lowest_common_ancestor(
    anchor_a,
    anchor_b
).await?;

println!("Common causal ancestor: {}", lca.id);
```

#### Topological Sort

```rust
// Get anchors in causal order
let sorted = coordinator.topological_sort(anchor_set).await?;

// Process in causal order
for anchor in sorted {
    process_event(anchor).await?;
}
```

## Event Types

```rust
pub enum Event {
    /// Presence events
    PresenceSignaled,
    PresenceUpdated,
    PresenceWithdrawn,

    /// Coupling events
    CouplingEstablished,
    CouplingStrengthened,
    CouplingWeakened,
    Decoupled,

    /// Meaning events
    MeaningFormed,
    MeaningConverged,
    MeaningDiverged,

    /// Intent events
    IntentFormed,
    IntentStabilized,
    IntentChanged,

    /// Commitment events
    CommitmentCreated,
    CommitmentActivated,
    CommitmentFulfilled,
    CommitmentViolated,

    /// Consequence events
    ConsequenceProduced,
    ConsequenceReversed,

    /// Custom events
    Custom(String),
}
```

## Temporal Invariants

### Invariant: Dependencies Must Exist

Before creating an anchor with dependencies:

```rust
// Check all dependencies exist
for dep_id in dependencies {
    if !coordinator.has_anchor(dep_id).await? {
        return Err(TemporalError::MissingDependency(dep_id));
    }
}
```

### Invariant: No Circular Dependencies

```rust
// Before adding dependency a → b, check that b ↛ a
if coordinator.happened_before(new_dep, current_anchor).await? {
    return Err(TemporalError::CircularDependency);
}
```

### Invariant: Local Timestamps Monotonic

```rust
// Within a Resonator's timeline
for i in 1..anchors.len() {
    assert!(anchors[i].local_timestamp >= anchors[i-1].local_timestamp);
}
```

## Use Cases

### 1. Ordering Commitment Execution

```rust
// Commitment B depends on Commitment A being fulfilled
let anchor_a = commitment_a.create_fulfillment_anchor().await?;
let anchor_b = commitment_b.create_activation_anchor_with_deps(
    vec![anchor_a.id]
).await?;

// System ensures A completes before B starts
```

### 2. Tracking Causal Chains

```rust
// Get full causal chain for an outcome
let consequence_anchor = consequence.anchor;
let chain = coordinator.causal_history(consequence_anchor).await?;

println!("Causal chain:");
for (i, anchor) in chain.iter().enumerate() {
    println!("  {}. {} at {}", i+1, anchor.event, anchor.local_timestamp);
}
```

### 3. Detecting Concurrent Modifications

```rust
// Two Resonators modify same state
let anchor_a = resonator_a.modify_state_anchor().await?;
let anchor_b = resonator_b.modify_state_anchor().await?;

if !coordinator.happened_before(anchor_a, anchor_b).await? &&
   !coordinator.happened_before(anchor_b, anchor_a).await? {
    // Concurrent modifications - need conflict resolution
    println!("⚠️ Concurrent modifications detected");
    resolve_conflict(anchor_a, anchor_b).await?;
}
```

### 4. Ensuring Precondition Satisfaction

```rust
// Action requires precondition to be satisfied
let precondition_anchor = check_precondition().await?;

let action_anchor = resonator.create_anchor_with_deps(
    Event::Custom("action_executed".to_string()),
    vec![precondition_anchor.id]
).await?;

// Causal dependency ensures precondition happened first
```

### 5. Audit Trail Construction

```rust
// Reconstruct what led to a consequence
let consequence_anchor = get_consequence_anchor(consequence_id).await?;
let audit_trail = coordinator.causal_history(consequence_anchor).await?;

// Full causal history for audit
for anchor in audit_trail {
    audit_log.append(AuditEntry {
        event: anchor.event,
        timestamp: anchor.local_timestamp,
        resonator: anchor.resonator,
    });
}
```

## Vector Clocks (Internal Implementation)

Internally, MAPLE uses vector clocks for efficient causal ordering:

```rust
pub struct VectorClock {
    /// Clock value per Resonator
    clocks: HashMap<ResonatorId, u64>,
}

impl VectorClock {
    /// Increment this Resonator's clock
    pub fn tick(&mut self, resonator: ResonatorId) {
        *self.clocks.entry(resonator).or_insert(0) += 1;
    }

    /// Merge with another vector clock
    pub fn merge(&mut self, other: &VectorClock) {
        for (resonator, other_value) in &other.clocks {
            let value = self.clocks.entry(*resonator).or_insert(0);
            *value = (*value).max(*other_value);
        }
    }

    /// Check if this happened before other
    pub fn happened_before(&self, other: &VectorClock) -> bool {
        let mut less_than_or_equal = true;
        let mut strictly_less = false;

        for (resonator, other_value) in &other.clocks {
            let self_value = self.clocks.get(resonator).copied().unwrap_or(0);
            if self_value > *other_value {
                return false;
            }
            if self_value < *other_value {
                strictly_less = true;
            }
        }

        less_than_or_equal && strictly_less
    }
}
```

### Vector Clock Example

```
Initial state:
  A: [A:0, B:0, C:0]
  B: [A:0, B:0, C:0]
  C: [A:0, B:0, C:0]

A performs event a1:
  A: [A:1, B:0, C:0]

B performs event b1:
  B: [A:0, B:1, C:0]

A sends message to B (includes A's vector clock):
  B receives and merges: [A:1, B:1, C:0]
  B performs b2: [A:1, B:2, C:0]

Now we can determine:
  a1 → b2 (because [A:1,B:0,C:0] < [A:1,B:2,C:0])
  a1 ∥ b1 (neither < the other)
```

## Temporal Queries

### Query Anchors in Time Range

```rust
let anchors = coordinator.query_anchors(
    TemporalQuery::TimeRange {
        resonator: Some(resonator_id),
        start: yesterday,
        end: now,
    }
).await?;

println!("Found {} anchors", anchors.len());
```

### Query by Event Type

```rust
let commitment_anchors = coordinator.query_anchors(
    TemporalQuery::EventType(Event::CommitmentFulfilled)
).await?;
```

### Query Causally Related

```rust
// Find all anchors causally related to this one
let related = coordinator.query_anchors(
    TemporalQuery::CausallyRelated {
        anchor: anchor_id,
        direction: CausalDirection::Both,  // Ancestors and descendants
    }
).await?;
```

## Temporal Metrics

### Causal Depth

```rust
// Maximum depth of causal chain
let depth = coordinator.causal_depth(anchor_id).await?;
println!("Causal depth: {}", depth);
```

### Concurrent Event Ratio

```rust
// What fraction of events are concurrent?
let stats = coordinator.concurrency_stats().await?;
println!("Concurrent events: {:.1}%",
    stats.concurrent_ratio * 100.0);
```

### Causal Latency

```rust
// Time between causally related events
let latency = coordinator.causal_latency(
    anchor_a,
    anchor_b
).await?;
println!("Causal latency: {:?}", latency);
```

## Platform-Specific Configuration

### Mapleverse (Pure AI)

```rust
TemporalConfig {
    anchor_retention: Duration::from_days(30),
    max_dependencies: 10,
    enable_vector_clocks: true,
    compact_old_anchors: true,
}
```

### Finalverse (Human-AI)

```rust
TemporalConfig {
    anchor_retention: Duration::from_days(90),  // Longer for humans
    max_dependencies: 5,
    enable_vector_clocks: true,
    human_anchor_priority: true,
}
```

### iBank (Finance)

```rust
TemporalConfig {
    anchor_retention: Duration::from_years(7),  // Regulatory requirement
    max_dependencies: 20,
    enable_vector_clocks: true,
    require_audit_trail: true,
    immutable_anchors: true,
}
```

## Best Practices

### For Resonator Developers

1. **Create anchors for important events**
   ```rust
   // Mark significant events with anchors
   let anchor = resonator.create_anchor(
       Event::CommitmentFulfilled
   ).await?;
   ```

2. **Use dependencies to express causality**
   ```rust
   // Make causal relationships explicit
   let action_anchor = resonator.create_anchor_with_deps(
       Event::Custom("action"),
       vec![precondition_anchor]
   ).await?;
   ```

3. **Don't create unnecessary dependencies**
   ```rust
   // WRONG: Too many dependencies
   create_anchor_with_deps(event, vec![
       every, single, previous, anchor, ...
   ]);

   // RIGHT: Only direct causal dependencies
   create_anchor_with_deps(event, vec![
       immediate_cause_anchor
   ]);
   ```

4. **Use local timestamps for ordering within timeline**
   ```rust
   // Query my own timeline
   let my_anchors = coordinator.query_anchors(
       TemporalQuery::Resonator(my_id)
   ).await?;

   // Already in causal order (local timestamps monotonic)
   for anchor in my_anchors {
       process(anchor);
   }
   ```

### For Platform Operators

1. **Monitor causal depth**: Deep chains may indicate issues
2. **Track concurrency ratio**: High concurrency is good (parallelism)
3. **Archive old anchors**: Retention policies
4. **Validate causal graphs**: Check for anomalies
5. **Backup temporal data**: Critical for audit trails

## Comparison with Competitors

### Google A2A

**A2A approach:**
- Assumes synchronized clocks
- Global timestamps
- No causal tracking
- No happened-before relation

**MAPLE advantage:**
- No clock synchronization required
- Causal ordering explicit
- Happened-before computable
- Scales naturally to distributed systems

### Anthropic MCP

**MCP approach:**
- No temporal model
- Stateless interactions
- No event ordering
- No causality tracking

**MAPLE advantage:**
- Complete temporal model
- Causal relationships tracked
- Event ordering guaranteed
- Full audit trail capability

## Advanced Topics

### Conflict-Free Replicated Data Types (CRDTs)

MAPLE's temporal model enables CRDT-style conflict resolution:

```rust
// Detect concurrent modifications
if concurrent(anchor_a, anchor_b) {
    // Apply CRDT merge rules
    let merged = crdt_merge(state_a, state_b);
    apply_merged_state(merged);
}
```

### Snapshot Isolation

Create consistent snapshots using causal cuts:

```rust
// Get consistent snapshot at this anchor
let snapshot = coordinator.causal_cut(anchor_id).await?;

// Snapshot includes all causally preceding events
// Excludes all concurrent and future events
```

### Distributed Transactions

Use causal ordering for distributed coordination:

```rust
// Transaction across multiple Resonators
let tx_start = coordinator.create_anchor(
    Event::Custom("tx_start".to_string())
).await?;

// All transaction events depend on start
let action_1 = create_anchor_with_deps(Event::Action1, vec![tx_start]);
let action_2 = create_anchor_with_deps(Event::Action2, vec![tx_start]);

// Commit depends on all actions
let tx_commit = create_anchor_with_deps(
    Event::Custom("tx_commit".to_string()),
    vec![action_1, action_2]
);
```

## Future Enhancements

### Planned Features

1. **Persistent temporal store**: Durable anchor storage
2. **Temporal queries**: Rich query language
3. **Causal consistency**: Distributed consistency guarantees
4. **Time travel debugging**: Replay causal history
5. **Temporal analytics**: Analyze event patterns

### Research Directions

1. **Formal verification**: Prove temporal properties
2. **Optimized causal structures**: Better data structures
3. **Probabilistic causality**: Handle uncertainty
4. **Temporal machine learning**: Learn from causal patterns

## Summary

Temporal coordination in MAPLE is a **fundamental innovation**:

- ✅ No global clocks required
- ✅ Causal ordering explicit and computable
- ✅ Scales naturally to distributed systems
- ✅ Supports happened-before reasoning
- ✅ Enables audit trail construction
- ✅ Concurrent events processed in parallel
- ✅ Vector clocks for efficiency
- ✅ Platform-specific retention policies

By using temporal anchors and causal ordering instead of synchronized clocks, MAPLE achieves scalability, correctness, and auditability that traditional frameworks cannot match.

## Related Documentation

- [Architecture Overview](../architecture.md) - System design
- [Commitments](commitments.md) - Accountability system
- [Attention](attention.md) - Resource management
- [Getting Started](../getting-started.md) - Basic usage

---

**Built with 🍁 by the MAPLE Team**
