# ADR 002: Attention Economics

**Status**: Accepted

**Date**: 2026-01-15

**Decision Makers**: MAPLE Architecture Team

---

## Context

Multi-agent systems face resource exhaustion problems:

1. **Unlimited relationships**: Agents can form unbounded connections, exhausting memory and CPU
2. **No degradation strategy**: Systems crash when overloaded rather than degrading gracefully
3. **Denial of service**: Malicious agents can exhaust resources through connection spam
4. **No prioritization**: All relationships treated equally regardless of value
5. **Hidden costs**: Resource consumption not explicit or manageable

Traditional solutions (rate limiting, connection limits) are:
- **Arbitrary**: Limits chosen without principled basis
- **Inflexible**: Can't adapt to varying conditions
- **Incomplete**: Don't address root cause
- **Fragile**: Break down under unusual patterns

We need a **principled, flexible, and complete** resource management system.

## Decision

**We will implement ATTENTION ECONOMICS as the core resource management mechanism.**

### What is Attention?

**Attention** represents the finite cognitive capacity a Resonator has to maintain couplings and process resonance.

### Key Properties

1. **Finite**: Every Resonator has limited attention (default: 1000.0 units)
2. **Allocated**: Attention is explicitly allocated to couplings
3. **Bounded**: Coupling strength ≤ available attention (Invariant #5)
4. **Recoverable**: Attention released when couplings end
5. **Protected**: Safety reserve (10%) cannot be allocated

### Attention Budget Structure

```rust
pub struct AttentionBudget {
    pub total_capacity: f64,        // Fixed capacity
    pub allocated: f64,              // Bound to couplings
    pub available: f64,              // Free for allocation
    pub safety_reserve: f64,         // Protected reserve
}
```

### Allocation Rules

1. **Before coupling**: Attention must be allocated before coupling created
2. **Proportional cost**: Stronger couplings cost more attention
3. **Cannot exceed capacity**: Total allocated ≤ (capacity - safety_reserve)
4. **Released on decouple**: Attention returned when coupling ends
5. **Rebalanceable**: Can redistribute across couplings

## Rationale

### Why "Attention"?

1. **Intuitive metaphor**: Matches human cognitive limits
2. **Biologically inspired**: Real brains have finite attention
3. **Naturally bounded**: Reflects actual resource constraints
4. **Prioritization**: Forces choices about what matters
5. **Graceful degradation**: Low attention signals need to reduce load

### Why Not Alternatives?

**Connection limits**: Arbitrary, inflexible, no prioritization

**Rate limiting**: Addresses symptoms not cause, brittle

**Memory limits**: Too low-level, doesn't capture cognitive cost

**Computational budgets**: Hard to estimate, platform-dependent

**Attention economics**: Principled, flexible, complete

## Consequences

### Positive

1. **Prevents Resource Exhaustion**
   - Natural bounds on couplings
   - Cannot create unlimited relationships
   - System remains stable

2. **Enables Graceful Degradation**
   - Low attention signals overload
   - Can weaken low-priority couplings
   - System continues operating

3. **Detects Attacks**
   - Attention exhaustion patterns visible
   - Coercion through attention drain detectable
   - Early warning of manipulation

4. **Encourages Meaningful Relationships**
   - Forces prioritization
   - Quality over quantity
   - Strengthens valuable couplings

5. **Predictable Resource Usage**
   - Attention budget known upfront
   - Resource consumption trackable
   - Capacity planning possible

### Negative

1. **Additional Complexity**
   - Developers must manage attention
   - Extra bookkeeping required
   - More state to track

2. **Performance Overhead**
   - Allocation/release operations
   - Budget tracking
   - Rebalancing computation

3. **Tuning Required**
   - Optimal capacity varies by use case
   - Attention costs need calibration
   - Platform-specific tuning needed

### Mitigations

For complexity:
- Clear documentation
- Helper methods
- Automatic rebalancing
- Sensible defaults

For overhead:
- Efficient data structures
- Minimal locking
- Batch operations
- Lazy rebalancing

For tuning:
- Platform-specific defaults
- Monitoring tools
- Tuning guides
- Auto-tuning (future)

## Alternatives Considered

### Alternative 1: No Resource Management

**Why Rejected**: Vulnerable to exhaustion, no graceful degradation, unpredictable behavior

### Alternative 2: Simple Connection Limits

**Why Rejected**: Arbitrary limits, no prioritization, inflexible, doesn't reflect actual resource costs

### Alternative 3: Memory-Based Limits

**Why Rejected**: Too low-level, platform-dependent, doesn't capture cognitive/coordination cost

### Alternative 4: Computational Budgets

**Why Rejected**: Hard to estimate costs, platform-dependent, doesn't map to cognitive limits

## Implementation

### AttentionAllocator

```rust
pub struct AttentionAllocator {
    budgets: DashMap<ResonatorId, AttentionBudget>,
    config: AttentionConfig,
}

impl AttentionAllocator {
    pub async fn allocate(
        &self,
        id: ResonatorId,
        amount: f64
    ) -> Result<AllocationToken> {
        // Check availability
        // Allocate attention
        // Return token
    }

    pub async fn release(&self, token: AllocationToken) -> Result<()> {
        // Return attention to pool
    }

    pub async fn rebalance(&self, id: ResonatorId) -> Result<()> {
        // Optimize allocation across couplings
    }
}
```

### Integration with Coupling

```rust
// Attention allocated before coupling created
let token = attention_allocator.allocate(
    resonator_id,
    coupling_cost
).await?;

// Coupling creation
let coupling = coupling_fabric.establish_coupling(
    params,
    token  // Proves attention allocated
).await?;

// Attention released on decouple
coupling.decouple().await?;
// Token automatically released
```

### Platform-Specific Configuration

**Mapleverse** (Pure AI):
- Default capacity: 1000.0
- Safety reserve: 10%
- Tight thresholds for AI

**Finalverse** (Human-AI):
- Human capacity: 1500.0 (larger)
- AI capacity: 1000.0
- More lenient thresholds

**iBank** (Finance):
- Capacity: 2000.0 (complex operations)
- Safety reserve: 20% (larger margin)
- Strict monitoring

## Enforcement

**Architectural Invariant #5**: Coupling bounded by attention

```rust
fn check_invariant_5(coupling: &Coupling, budget: &AttentionBudget) -> Result<()> {
    if coupling.strength > budget.available {
        return Err(InvariantViolation::CouplingExceedsAttention);
    }
    Ok(())
}
```

This is enforced at runtime. Violations cause system errors.

## Monitoring

### Metrics

- Attention utilization per Resonator
- Allocation/release rates
- Exhaustion events
- Rebalancing frequency
- Average coupling cost

### Alerts

- Utilization >80%: Warning
- Utilization >90%: Critical
- Frequent exhaustion: Potential attack
- Rapid allocation: Suspicious pattern

## Future Enhancements

1. **Dynamic capacity**: Adjust based on system load
2. **Attention marketplace**: Trade attention between Resonators
3. **Predictive allocation**: ML-based forecasting
4. **Attention debt**: Temporarily exceed capacity with penalties
5. **Group pools**: Shared attention for collectives

## References

- Attention Economics Documentation: `docs/concepts/attention.md`
- Coupling Documentation: `docs/concepts/coupling.md`
- Architecture Overview: `docs/architecture.md`
- Implementation: `crates/maple-runtime/src/allocator/`

## Approval

**Approved by**: MAPLE Architecture Team

**Date**: 2026-01-15

---

**This decision is fundamental to MAPLE's resource management and safety properties.**
