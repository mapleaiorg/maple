# Attention Economics and Resource Management

## Overview

Attention is MAPLE's fundamental resource for managing coordination capacity. Unlike traditional agent frameworks that allow unlimited relationships and interactions, MAPLE implements **finite attention budgets** that create natural bounds, prevent abuse, and enable graceful degradation.

## Core Concept

In MAPLE, **attention** represents the finite cognitive capacity a Resonator has to participate in couplings (relationships) and process resonance. Every coupling consumes attention, and when attention is exhausted, no new couplings can be formed.

```
Traditional Frameworks:    Unlimited relationships → Resource exhaustion → System failure

MAPLE:                     Finite attention → Bounded relationships → Graceful degradation
```

## Why Attention Economics?

### 1. Prevents Runaway Resource Consumption

Without attention bounds, a Resonator could:
- Form unlimited couplings
- Exhaust memory and CPU
- Create cascading failures
- Be targeted by denial-of-service attacks

With attention economics:
- Maximum couplings naturally bounded
- Resource usage predictable
- System remains stable
- Attacks have limited impact

### 2. Enables Graceful Degradation

When attention pressure is high:
- Lower priority couplings can be weakened
- Non-essential relationships temporarily suspended
- Critical couplings maintained
- System continues operating

### 3. Detects Coercion and Manipulation

Attention exhaustion patterns can indicate:
- Aggressive coupling attempts
- Manipulation or coercion
- Unbalanced relationships
- Attention exhaustion attacks

### 4. Creates Natural Coordination Bounds

Finite attention:
- Forces prioritization of relationships
- Encourages meaningful over superficial couplings
- Reflects real-world cognitive limits
- Prevents coordination overhead explosion

## Attention Budget Structure

```rust
pub struct AttentionBudget {
    /// Total capacity (typically 1000.0)
    pub total_capacity: f64,

    /// Currently allocated to active couplings
    pub allocated: f64,

    /// Available for new allocations
    pub available: f64,

    /// Safety reserve (cannot be allocated)
    pub safety_reserve: f64,
}
```

### Key Properties

- **Total Capacity**: Fixed per Resonator (default: 1000.0)
- **Allocated**: Bound to active couplings
- **Available**: `total_capacity - allocated - safety_reserve`
- **Safety Reserve**: 10% reserved for critical operations

## Attention Allocation

### Establishing a Coupling

When a coupling is established, attention is allocated:

```rust
let coupling_params = CouplingParams {
    source: resonator_a.id,
    target: resonator_b.id,
    initial_strength: 0.3,
    initial_attention_cost: 100.0,  // Attention allocated
    ..Default::default()
};

let coupling = runtime.establish_coupling(coupling_params).await?;
```

**Allocation process:**

1. Check if sufficient attention available
2. Check safety reserve not violated
3. Allocate attention to coupling
4. Update available attention
5. Return allocation token

### Allocation Token

```rust
pub struct AllocationToken {
    pub id: AllocationId,
    pub resonator: ResonatorId,
    pub amount: f64,
    pub allocated_at: TemporalAnchor,
}
```

The token:
- Proves attention was allocated
- Enables release on decouple
- Tracks allocation lifetime
- Prevents double-allocation

## Attention Classes

MAPLE categorizes operations by attention priority:

```rust
pub enum AttentionClass {
    Critical,    // Safety-critical operations
    High,        // Important but not critical
    Normal,      // Standard operations
    Low,         // Optional enhancements
    Background,  // Opportunistic work
}
```

### Priority Handling

- **Critical**: Always execute (safety overrides optimization)
- **High**: Execute unless circuit breaker triggered
- **Normal**: Fair scheduling with other operations
- **Low**: Execute when capacity available
- **Background**: Execute only when system idle

## Attention States

### Healthy State

```
Available Attention: > 30%
Status: Healthy
Action: Accept new couplings normally
```

### Pressure State

```
Available Attention: 10-30%
Status: Under pressure
Action:
  - Prioritize high-strength couplings
  - Warn about low attention
  - Consider weakening low-priority couplings
```

### Exhaustion State

```
Available Attention: < 10%
Status: Exhausted
Action:
  - Reject new coupling requests
  - Maintain only critical couplings
  - Trigger rebalancing
  - Alert about potential attack
```

### Safety Reserve Violation

```
Available Attention: Would go negative
Status: Safety reserve protection
Action:
  - Hard rejection of request
  - System error (invariant violation)
  - Potential security incident
```

## Attention Operations

### Check Available Attention

```rust
if let Some(budget) = resonator.attention_status().await {
    println!("Total: {}", budget.total_capacity);
    println!("Allocated: {}", budget.allocated);
    println!("Available: {}", budget.available);

    let utilization = budget.allocated / budget.total_capacity;
    println!("Utilization: {:.1}%", utilization * 100.0);
}
```

### Allocate Attention

```rust
let token = attention_allocator.allocate(
    resonator_id,
    amount
).await?;

// Token must be kept to release later
```

### Release Attention

```rust
attention_allocator.release(token).await?;

// Attention returned to available pool
```

### Rebalance Attention

```rust
attention_allocator.rebalance(resonator_id).await?;

// Optimizes allocation across couplings
// May weaken low-priority couplings
// Strengthens high-value relationships
```

## Coupling Attention Cost

The attention cost of a coupling depends on:

### 1. Coupling Strength

Stronger couplings require more attention:

```
Attention Cost = Base Cost × Strength Multiplier

Example:
- Strength 0.3 → 100 attention
- Strength 0.5 → 150 attention
- Strength 1.0 → 200 attention
```

### 2. Coupling Scope

Different scopes have different costs:

```rust
pub enum CouplingScope {
    Full,              // Highest cost (full resonance)
    StateOnly,         // Medium cost (state sharing only)
    IntentOnly,        // Medium cost (intent sharing only)
    ObservationalOnly, // Lowest cost (observe only)
}
```

### 3. Interaction Frequency

Frequently used couplings may cost more:
- High interaction rate → Higher attention cost
- Infrequent interaction → Lower attention cost
- Adaptive cost adjustment over time

### 4. Meaning Convergence

Better understanding reduces overhead:
- Low convergence (<0.3) → Higher cost
- Medium convergence (0.3-0.7) → Normal cost
- High convergence (>0.7) → Lower cost

## Attention Rebalancing

### Automatic Rebalancing

The `AttentionAllocator` automatically rebalances when:

1. **Exhaustion detected**: Available < 10%
2. **New high-priority coupling**: Needs space
3. **Coupling strengthens**: Requires more attention
4. **Periodic maintenance**: Regular optimization

### Rebalancing Strategy

```
1. Identify low-value couplings:
   - Low meaning convergence
   - Infrequent interaction
   - Observational-only scope

2. Calculate rebalancing budget:
   - Target available attention: 30%
   - Current available: X%
   - Need to free: (30 - X)%

3. Weaken or suspend couplings:
   - Start with lowest value
   - Weaken gradually (not abruptly)
   - Preserve commitments

4. Update allocations:
   - Release freed attention
   - Re-allocate as needed
   - Update coupling states
```

### Manual Rebalancing

Resonators can request rebalancing:

```rust
// Explicit rebalancing request
runtime.rebalance_attention(resonator_id).await?;

// With specific strategy
runtime.rebalance_attention_with_strategy(
    resonator_id,
    RebalanceStrategy::PreferHighConvergence
).await?;
```

## Attention Attacks and Defenses

### Attention Exhaustion Attack

**Attack pattern:**
- Attacker creates many weak couplings
- Exhausts victim's attention
- Prevents legitimate couplings
- Denial of service

**Defense mechanisms:**

1. **Gradual strengthening requirement**: Can't instantly create strong couplings
2. **Safety reserves**: Last 10% cannot be allocated
3. **Rate limiting**: Coupling establishment rate limited
4. **Coercion detection**: Patterns trigger warnings
5. **Circuit breakers**: Auto-reject when under attack

### Attention Drain Attack

**Attack pattern:**
- Attacker strengthens coupling aggressively
- Consumes increasing attention
- Victim loses capacity for others
- Resource monopolization

**Defense mechanisms:**

1. **Symmetric coupling rules**: Both parties must agree to strengthen
2. **Strengthening limits**: Max 0.1 increase per step
3. **Asymmetry detection**: Flags one-sided relationships
4. **Always disengageable**: Victims can always decouple
5. **Attention monitoring**: Alerts on abnormal patterns

### Distributed Attention Attack

**Attack pattern:**
- Multiple attackers coordinate
- Each takes small attention slice
- Combined effect exhausts victim
- Harder to detect

**Defense mechanisms:**

1. **Global attention tracking**: Monitor total allocation
2. **Pattern analysis**: Detect coordinated behavior
3. **Temporal correlation**: Identify simultaneous attacks
4. **Reputation systems**: Track historical behavior
5. **Adaptive thresholds**: Tighten limits under pressure

## Attention Monitoring

### Telemetry

```rust
pub struct AttentionTelemetry {
    pub total_capacity: f64,
    pub total_allocated: f64,
    pub total_available: f64,
    pub coupling_count: usize,
    pub average_coupling_cost: f64,
    pub utilization: f64,
    pub exhaustion_events: Vec<ExhaustionEvent>,
    pub rebalance_events: Vec<RebalanceEvent>,
}
```

### Metrics to Monitor

**Health metrics:**
- Utilization percentage
- Available attention trend
- Exhaustion event frequency
- Rebalance frequency

**Performance metrics:**
- Allocation latency
- Release latency
- Rebalance duration
- Coupling creation rate

**Security metrics:**
- Failed allocation attempts
- Exhaustion attack patterns
- Suspicious coupling patterns
- Coercion indicators

## Attention Configuration

### Per-Resonator Configuration

```rust
pub struct AttentionConfig {
    /// Total capacity per Resonator
    pub default_capacity: f64,  // 1000.0

    /// Safety reserve percentage
    pub safety_reserve_pct: f64,  // 0.1 (10%)

    /// Exhaustion threshold
    pub exhaustion_threshold: f64,  // 0.1 (10% available)

    /// Rebalance trigger threshold
    pub rebalance_trigger: f64,  // 0.2 (20% available)

    /// Enable automatic rebalancing
    pub auto_rebalance: bool,  // true

    /// Rebalance interval
    pub rebalance_interval: Duration,  // 60 seconds
}
```

### Platform-Specific Configuration

#### Mapleverse (Pure AI)

```rust
AttentionConfig {
    default_capacity: 1000.0,
    safety_reserve_pct: 0.1,
    exhaustion_threshold: 0.05,  // Tighter for AI
    rebalance_trigger: 0.15,
    auto_rebalance: true,
    rebalance_interval: Duration::from_secs(30),
}
```

#### Finalverse (Human-AI)

```rust
AttentionConfig {
    default_capacity: 1500.0,  // Humans get more
    safety_reserve_pct: 0.15,  // Larger safety margin
    exhaustion_threshold: 0.2,  // More lenient
    rebalance_trigger: 0.3,
    auto_rebalance: true,
    rebalance_interval: Duration::from_secs(60),
}
```

#### iBank (Finance)

```rust
AttentionConfig {
    default_capacity: 2000.0,  // High capacity for finance
    safety_reserve_pct: 0.2,   // Large safety margin
    exhaustion_threshold: 0.1,
    rebalance_trigger: 0.25,
    auto_rebalance: true,
    rebalance_interval: Duration::from_secs(10),
}
```

## Architectural Invariant

**Invariant #5: Coupling Bounded by Attention**

```
✓ ALLOWED:  Coupling Strength ≤ Available Attention
✗ FORBIDDEN: Coupling that exceeds attention budget
```

This invariant is **enforced at runtime**. Any attempt to:
- Allocate more attention than available
- Violate safety reserve
- Create coupling without allocation

Results in immediate failure.

## Best Practices

### For Resonator Developers

1. **Monitor attention regularly**
   ```rust
   if let Some(budget) = resonator.attention_status().await {
       if budget.available < 100.0 {
           // Low attention - take action
       }
   }
   ```

2. **Prioritize high-value couplings**
   - Allocate more to high-convergence relationships
   - Weaken low-value couplings proactively
   - Don't create unnecessary couplings

3. **Handle exhaustion gracefully**
   ```rust
   match resonator.couple_with(target, params).await {
       Err(CouplingError::InsufficientAttention) => {
           // Rebalance or wait
           runtime.rebalance_attention(resonator.id).await?;
       }
       Ok(coupling) => { /* proceed */ }
       Err(e) => return Err(e.into()),
   }
   ```

4. **Decouple when done**
   ```rust
   // Always decouple when relationship ends
   coupling.decouple().await?;
   // Attention is released
   ```

### For Platform Operators

1. **Configure appropriately**: Adjust capacity based on expected load
2. **Monitor telemetry**: Track utilization and exhaustion events
3. **Set alerts**: Warn on high utilization or frequent exhaustion
4. **Tune thresholds**: Adjust based on observed patterns
5. **Investigate anomalies**: Exhaustion spikes may indicate attacks

## Comparison with Competitors

### Google A2A

**A2A approach:**
- No resource management
- Unlimited tool invocations
- No bounds on relationships
- Vulnerable to resource exhaustion

**MAPLE advantage:**
- Finite attention bounds
- Predictable resource usage
- Graceful degradation
- Attack resistance

### Anthropic MCP

**MCP approach:**
- No resource model
- Context is unbounded
- No relationship limits
- No protection mechanisms

**MAPLE advantage:**
- Explicit resource economics
- Architectural bounds
- Natural coordination limits
- Built-in protections

## Future Enhancements

### Planned Features

1. **Dynamic capacity adjustment**: Adjust total capacity based on load
2. **Attention marketplace**: Trade attention between Resonators
3. **Predictive allocation**: ML-based attention forecasting
4. **Attention debt**: Temporarily exceed capacity (with penalties)
5. **Group attention pools**: Shared attention for collectives

### Research Directions

1. **Optimal allocation strategies**: Game-theoretic approaches
2. **Attention-aware routing**: Route based on availability
3. **Federated attention**: Cross-runtime attention coordination
4. **Attention economics**: Pricing models for attention

## Summary

Attention economics is a **fundamental innovation** in MAPLE that:

- ✅ Prevents runaway resource consumption
- ✅ Enables graceful degradation under pressure
- ✅ Detects and prevents manipulation
- ✅ Creates natural coordination bounds
- ✅ Enforces architectural safety (Invariant #5)
- ✅ Scales to 100M+ Resonators

By treating attention as a finite, valuable resource, MAPLE creates a robust, scalable, and abuse-resistant multi-agent system that far surpasses traditional frameworks.

## Related Documentation

- [Coupling](coupling.md) - How relationships work
- [Commitments](commitments.md) - Accountability system
- [Temporal Anchors](temporal.md) - Causal time
- [Architecture Overview](../architecture.md) - System design

---

**Built with 🍁 by the MAPLE Team**
