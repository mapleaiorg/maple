# ADR 003: Gradient Presence

**Status**: Accepted

**Date**: 2024-01-15

**Decision Makers**: MAPLE Architecture Team

---

## Context

Traditional systems model agent presence as **binary**: an agent is either "online" or "offline". This oversimplification creates problems:

1. **All-or-nothing**: Can't represent partial availability
2. **No nuance**: Can't distinguish "busy" from "available"
3. **Forces interaction**: Being online implies willingness to interact
4. **Binary transitions**: Sharp edges between states
5. **Privacy issues**: Can't observe without being noticed

Reality is more nuanced:
- Agents may be partially available
- Responsiveness varies
- Stability fluctuates
- Willingness to couple changes
- Silent observation should be possible

We need a richer model of presence.

## Decision

**We will use GRADIENT PRESENCE instead of binary online/offline.**

### Presence as Multidimensional Gradient

```rust
pub struct PresenceState {
    pub discoverability: f64,      // How findable (0.0-1.0)
    pub responsiveness: f64,        // How quick to respond (0.0-1.0)
    pub stability: f64,             // How consistently available (0.0-1.0)
    pub coupling_readiness: f64,    // Willing to couple (0.0-1.0)
    pub silent_mode: bool,          // Present but not signaling
    pub last_signal: TemporalAnchor,
}
```

### Dimensions Explained

**Discoverability** (0.0-1.0):
- 0.0: Completely hidden
- 0.5: Moderately discoverable
- 1.0: Highly visible

**Responsiveness** (0.0-1.0):
- 0.0: Not responding
- 0.5: Slow responses
- 1.0: Immediate responses

**Stability** (0.0-1.0):
- 0.0: Frequently dropping
- 0.5: Intermittent
- 1.0: Rock solid

**Coupling Readiness** (0.0-1.0):
- 0.0: Not open to new couplings
- 0.5: Selective about couplings
- 1.0: Eager to couple

**Silent Mode** (bool):
- false: Actively signaling
- true: Present but quiet (observer mode)

## Rationale

### Why Gradients?

1. **More Expressive**: Captures nuance of availability
2. **Reflects Reality**: Actual availability is not binary
3. **Enables Graceful Degradation**: Can reduce presence under load
4. **Privacy Control**: Silent mode for observation
5. **Human-Friendly**: Matches how humans experience availability

### Why These Dimensions?

**Discoverability**: Needed for privacy control and selective visibility

**Responsiveness**: Needed to set expectations about interaction speed

**Stability**: Needed to decide if coupling is worth the attention cost

**Coupling Readiness**: Needed to distinguish presence from willingness (Invariant #7)

**Silent Mode**: Needed for observation without participation (especially for humans)

### Why Not Binary?

Binary presence:
- ❌ Forces all-or-nothing decisions
- ❌ Can't represent partial availability
- ❌ Implies presence = willingness (violates human agency)
- ❌ No gradual degradation path
- ❌ No silent observation

Gradient presence:
- ✅ Expressive and nuanced
- ✅ Reflects actual availability
- ✅ Separates presence from willingness
- ✅ Enables graceful degradation
- ✅ Supports silent observation

## Consequences

### Positive

1. **Rich Expression**
   - Can represent complex availability states
   - Nuanced communication of intent
   - Better matches reality

2. **Privacy Control**
   - Silent mode for observation
   - Adjustable discoverability
   - Can be present without being "seen"

3. **Graceful Degradation**
   - Reduce responsiveness under load
   - Lower coupling readiness when busy
   - System adapts naturally

4. **Human Agency Protection**
   - Presence ≠ willingness (coupling_readiness dimension)
   - Can observe without participating (silent mode)
   - Respects human autonomy

5. **Load Signaling**
   - Reduced dimensions signal overload
   - Other agents adapt behavior
   - Natural coordination emerges

### Negative

1. **Increased Complexity**
   - More state to manage
   - More nuanced than binary
   - Requires understanding dimensions

2. **Interpretation Ambiguity**
   - What does 0.5 responsiveness mean?
   - How to interpret combinations?
   - Platform-specific meanings?

3. **Query Complexity**
   - Can't just query "who's online"
   - Multi-dimensional queries needed
   - Threshold decisions required

### Mitigations

For complexity:
- Clear documentation of each dimension
- Helper methods for common patterns
- Sensible defaults
- Examples showing usage

For ambiguity:
- Document dimension semantics
- Platform-specific guidelines
- Common patterns documented
- Validation of ranges

For queries:
- Helper methods for common queries
- Multi-dimensional query API
- Threshold recommendations
- Query examples

## Alternatives Considered

### Alternative 1: Binary Presence

**Why Rejected**: Insufficient expressiveness, forces all-or-nothing, violates human agency requirements

### Alternative 2: Status Enum

```rust
enum Status {
    Online,
    Away,
    Busy,
    DoNotDisturb,
    Offline,
}
```

**Why Rejected**: Still discrete, not composable, can't represent partial states, no silent mode

### Alternative 3: Binary + Metadata

Keep binary but add metadata fields

**Why Rejected**: Doesn't solve fundamental problems, creates inconsistencies, metadata easily ignored

## Implementation

### PresenceFabric

```rust
pub struct PresenceFabric {
    presence_states: DashMap<ResonatorId, PresenceState>,
    rate_limiter: RateLimiter,
    config: PresenceConfig,
}

impl PresenceFabric {
    pub async fn signal_presence(
        &self,
        id: ResonatorId,
        state: PresenceState
    ) -> Result<()> {
        // Validate ranges
        self.validate_presence(&state)?;

        // Rate limit
        self.rate_limiter.check(id).await?;

        // Update state
        self.presence_states.insert(id, state);

        Ok(())
    }

    pub async fn query_by_discoverability(
        &self,
        min: f64
    ) -> Vec<ResonatorId> {
        self.presence_states.iter()
            .filter(|entry| {
                entry.value().discoverability >= min &&
                !entry.value().silent_mode
            })
            .map(|entry| entry.key().clone())
            .collect()
    }
}
```

### Common Patterns

**Fully available**:
```rust
PresenceState {
    discoverability: 1.0,
    responsiveness: 1.0,
    stability: 1.0,
    coupling_readiness: 1.0,
    silent_mode: false,
    ..Default::default()
}
```

**Under load**:
```rust
PresenceState {
    discoverability: 0.8,
    responsiveness: 0.5,  // Slower responses
    stability: 1.0,
    coupling_readiness: 0.3,  // Not very open to new couplings
    silent_mode: false,
    ..Default::default()
}
```

**Silent observer**:
```rust
PresenceState {
    discoverability: 0.0,  // Hidden
    responsiveness: 0.0,  // Not responding
    stability: 1.0,  // Still present
    coupling_readiness: 0.0,  // Not coupling
    silent_mode: true,  // Explicitly silent
    ..Default::default()
}
```

**Human wanting privacy**:
```rust
PresenceState {
    discoverability: 0.1,  // Hard to find
    responsiveness: 0.0,
    stability: 1.0,
    coupling_readiness: 0.0,  // Not willing
    silent_mode: true,
    ..Default::default()
}
```

## Integration with Human Agency

**Critical for Finalverse**: Gradient presence enables human agency protection.

**Presence ≠ Willingness**:
```rust
// Human is present but not willing to couple
human.signal_presence(PresenceState {
    discoverability: 0.5,  // Visible
    coupling_readiness: 0.0,  // But not willing
    // ...
});

// AI agents must respect coupling_readiness
if target.presence().coupling_readiness < 0.3 {
    // Don't attempt coupling
}
```

**Silent Mode for Humans**:
```rust
// Human observing without participating
human.signal_presence(PresenceState {
    silent_mode: true,
    // ...
});

// Cannot be discovered or coupled with
assert!(cannot_discover(human.id));
assert!(cannot_couple_with(human.id));
```

## Monitoring

### Metrics

- Average presence dimensions per platform
- Silent mode usage
- Presence state distribution
- Dimension correlations
- Update frequency

### Insights

- Low responsiveness → Overloaded agents
- Low coupling readiness → System saturation
- High silent mode usage → Privacy concerns
- Dimension patterns → Behavior analysis

## Future Enhancements

1. **Dynamic dimensions**: Add platform-specific dimensions
2. **Presence forecasting**: Predict future availability
3. **Presence contracts**: Commit to minimum presence levels
4. **Presence aggregation**: Group presence views
5. **Emotional presence**: Add emotional state dimension (Finalverse)

## References

- Presence Fabric Implementation: `crates/maple-runtime/src/fabrics/presence.rs`
- Architecture Overview: `docs/architecture.md`
- Human Agency: `docs/concepts/profiles.md`
- Finalverse Platform: `docs/platforms/finalverse.md`

## Approval

**Approved by**: MAPLE Architecture Team

**Date**: 2024-01-15

---

**This decision is fundamental to MAPLE's expressiveness and human agency protection.**
