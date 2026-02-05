# 🎭 Resonators: Persistent Intelligent Entities

## What is a Resonator?

A **Resonator** is the fundamental unit in MAPLE - a persistent intelligent entity that participates in resonance (stateful relationships) with other Resonators.

**Resonators are NOT:**
- ❌ Ephemeral processes
- ❌ Stateless services
- ❌ Message endpoints
- ❌ Function calls

**Resonators ARE:**
- ✅ Persistent entities with continuity
- ✅ Stateful participants in relationships
- ✅ Resource-bounded (finite attention)
- ✅ Profile-governed (different modes)
- ✅ Identity-preserving across restarts

## Resonators as Entities (Individuals, Teams, Organizations)

A Resonator can represent more than a single process. It can model:

- An individual AI agent
- A human participant (Human profile)
- A team or organization composed of multiple agents
- A human + AI hybrid operating as one identity
- A service role such as an issuer or authority

In the AAS model, an **issuer** is a Resonator identity that grants capabilities. That issuer is usually an organization-level or system-level Resonator, not just a single worker. This lets policies, commitments, and accountability attach to a stable, persistent identity.

## Core Properties

### 1. Persistent Identity

Every Resonator has a **unique, persistent identity** that survives restarts:

```rust
pub struct ResonatorId(Uuid);

// First session
let spec = ResonatorSpec::default();
let resonator = runtime.register_resonator(spec).await?;
let id = resonator.id.clone();  // Save this

// Later session - SAME identity
let resumed = runtime.resume_resonator(id).await?;
assert_eq!(resumed.id, id);  // Identity preserved
```

**Why persistent identity matters:**
- Relationships can be rebuilt
- Reputation can accumulate
- Learning is preserved
- Accountability is maintained

### 2. Gradient Presence

Presence is **NOT binary** (online/offline). It's multidimensional:

```rust
pub struct PresenceState {
    pub discoverability: f64,      // How findable (0.0-1.0)
    pub responsiveness: f64,        // How quick to respond (0.0-1.0)
    pub stability: f64,             // How consistently available (0.0-1.0)
    pub coupling_readiness: f64,    // Willing to form couplings (0.0-1.0)
    pub last_signal: TemporalAnchor,
    pub silent_mode: bool,          // Present but quiet
}
```

**Example presence states:**

**Active and Open:**
```rust
PresenceState {
    discoverability: 1.0,      // Easy to find
    responsiveness: 0.9,        // Quick replies
    stability: 0.8,             // Mostly available
    coupling_readiness: 0.9,    // Open to new relationships
    silent_mode: false,
    ..Default::default()
}
```

**Silent Observer:**
```rust
PresenceState {
    discoverability: 0.1,      // Hard to find
    responsiveness: 0.3,        // Slow replies
    stability: 0.7,             // Consistently present
    coupling_readiness: 0.0,    // Not forming couplings
    silent_mode: true,          // Observing quietly
    ..Default::default()
}
```

**Going Offline:**
```rust
PresenceState {
    discoverability: 0.0,      // Not findable
    responsiveness: 0.0,        // Not responding
    stability: 0.0,             // Not available
    coupling_readiness: 0.0,    // Not ready
    silent_mode: false,
    ..Default::default()
}
```

### 3. Finite Attention Budget

Every Resonator has a **finite attention capacity**:

```rust
pub struct AttentionBudget {
    pub total_capacity: f64,    // Total attention (e.g., 1000.0)
    pub allocated: f64,         // Currently allocated to couplings
    pub available: f64,         // Still available (capacity - allocated)
    pub safety_reserve: f64,    // Cannot be allocated (last 10%)
}
```

**Why finite attention?**
- Prevents unlimited coupling
- Creates natural bounds
- Enables graceful degradation
- Reflects cognitive limitations

**Example:**
```rust
let budget = resonator.attention_status().await.unwrap();

println!("Total: {}", budget.total_capacity);      // 1000.0
println!("Allocated: {}", budget.allocated);       // 700.0
println!("Available: {}", budget.available);       // 300.0
println!("Reserve: {}", budget.safety_reserve);    // 100.0 (10%)

// Can allocate up to 200.0 more (300.0 - 100.0)
```

### 4. Profile Governance

Resonators operate under a **profile** that determines safety constraints:

```rust
pub enum ResonatorProfile {
    Human,        // Human users (strongest protections)
    World,        // AI agents in human environments
    Coordination, // Pure AI agents
    IBank,        // Financial AI agents
}
```

**Profile characteristics:**

| Profile | Human Protection | Coercion Detection | Audit Trails | Risk Assessment |
|---------|------------------|--------------------|--------------| ----------------|
| Human | ✅ Architectural | ✅ Enabled | Optional | Optional |
| World | ✅ Architectural | ✅ Enabled | Recommended | Recommended |
| Coordination | ❌ N/A | ❌ Disabled | Required | Optional |
| IBank | ❌ N/A | ❌ Disabled | ✅ Mandatory | ✅ Mandatory |

**Profile restrictions:**

```rust
// Mapleverse: Only Coordination allowed
let mut spec = ResonatorSpec::default();
spec.profile = ResonatorProfile::Coordination;  // ✅
spec.profile = ResonatorProfile::Human;         // ❌ Rejected

// Finalverse: Human and World allowed
spec.profile = ResonatorProfile::Human;   // ✅
spec.profile = ResonatorProfile::World;   // ✅
spec.profile = ResonatorProfile::IBank;   // ❌ Rejected

// iBank: Only IBank allowed
spec.profile = ResonatorProfile::IBank;   // ✅
spec.profile = ResonatorProfile::Human;   // ❌ Rejected
```

### 5. Continuity Across Restarts

Resonators maintain **continuity** across sessions:

```rust
pub struct ContinuityProof {
    pub resonator_id: ResonatorId,
    pub previous_anchor: TemporalAnchor,
    pub recovery_data: serde_json::Value,
    pub signature: Vec<u8>,
}
```

**Registration vs. Resumption:**

```rust
// FIRST TIME: Register
let spec = ResonatorSpec {
    profile: ResonatorProfile::Coordination,
    initial_attention: 1000.0,
    metadata: json!({ "name": "Alice" }),
};
let alice = runtime.register_resonator(spec).await?;

// Save ID for later
save_to_disk(&alice.id)?;

// LATER: Resume
let id = load_from_disk()?;
let alice = runtime.resume_resonator(id).await?;

// Same identity, relationships can be rebuilt
```

**What's preserved:**
- ✅ Identity (ResonatorId)
- ✅ Profile
- ✅ Attention budget
- ✅ Metadata
- ✅ Previous temporal anchors

**What's NOT automatically restored:**
- ❌ Active couplings (must be re-established)
- ❌ Current presence state (must be re-signaled)
- ❌ In-flight commitments (must be reconciled)

---

## Resonator Lifecycle

### 1. Registration (Bootstrap)

```rust
let spec = ResonatorSpec {
    profile: ResonatorProfile::Coordination,
    initial_attention: 1000.0,
    metadata: json!({
        "name": "Trading Agent",
        "version": "1.0.0"
    }),
};

let resonator = runtime.register_resonator(spec).await?;
```

**What happens:**
1. Unique identity (ResonatorId) generated
2. Attention budget allocated
3. Profile validated
4. Presence initialized (all dimensions 0.0)
5. Handle returned for interaction

### 2. Presence Signaling

```rust
let presence = PresenceState {
    discoverability: 1.0,
    responsiveness: 0.9,
    stability: 0.8,
    coupling_readiness: 0.8,
    silent_mode: false,
    ..Default::default()
};

resonator.signal_presence(presence).await?;
```

**Rate limiting:**
- Minimum interval between signals (default: 1 second)
- Prevents presence spam
- Enforces Invariant #6 (Safety overrides optimization)

### 3. Active Participation

```rust
// Establish couplings
let coupling = runtime.establish_coupling(CouplingParams {
    source: resonator.id.clone(),
    target: other_id,
    initial_strength: 0.3,
    initial_attention_cost: 100.0,
    ..Default::default()
}).await?;

// Form meaning, stabilize intent, make commitments
// ...

// Produce consequences
// ...
```

### 4. Graceful Degradation

When attention is low:

```rust
if let Some(budget) = resonator.attention_status().await {
    if budget.available < 100.0 {
        // Low attention - reduce coupling or decouple
        for coupling_id in resonator.couplings().await? {
            coupling.weaken(0.1).await?;  // Reduce load
        }
    }
}
```

### 5. Decoupling

```rust
// End relationships
for coupling_id in resonator.couplings().await? {
    let coupling = runtime.get_coupling(coupling_id)?;
    coupling.decouple().await?;  // Releases attention
}
```

### 6. Suspension

```rust
// Signal low presence (going away)
let presence = PresenceState {
    discoverability: 0.0,
    responsiveness: 0.0,
    stability: 0.0,
    coupling_readiness: 0.0,
    silent_mode: false,
    ..Default::default()
};

resonator.signal_presence(presence).await?;

// Identity preserved for later resumption
```

### 7. Resumption

```rust
// Later session
let resonator = runtime.resume_resonator(saved_id).await?;

// Restore presence
resonator.signal_presence(active_presence).await?;

// Rebuild couplings as needed
// ...
```

---

## Advanced Resonator Patterns

### Pattern 1: Silent Observer

Present but not actively participating:

```rust
let observer_presence = PresenceState {
    discoverability: 0.1,      // Hard to find
    responsiveness: 0.0,        // Not responding
    stability: 0.9,             // Consistently present
    coupling_readiness: 0.0,    // Not forming couplings
    silent_mode: true,          // Quiet observation
    ..Default::default()
};

observer.signal_presence(observer_presence).await?;

// Can observe but won't be discovered or coupled with
```

**Use cases:**
- Monitoring/auditing
- Passive learning
- Data collection
- Analysis without participation

### Pattern 2: Burst Responder

Low presence until needed, then highly responsive:

```rust
// Normal state: low presence
let low_presence = PresenceState {
    discoverability: 0.3,
    responsiveness: 0.2,
    stability: 0.9,
    coupling_readiness: 0.2,
    silent_mode: false,
    ..Default::default()
};

// When activated: high presence
let high_presence = PresenceState {
    discoverability: 1.0,
    responsiveness: 1.0,
    stability: 1.0,
    coupling_readiness: 0.9,
    silent_mode: false,
    ..Default::default()
};

// Switch based on events
if event_triggered {
    resonator.signal_presence(high_presence).await?;
} else {
    resonator.signal_presence(low_presence).await?;
}
```

**Use cases:**
- Event-driven agents
- Alarm systems
- Emergency responders
- On-demand services

### Pattern 3: Attention-Aware Coupling

Adapt coupling based on attention availability:

```rust
loop {
    let budget = resonator.attention_status().await.unwrap();

    if budget.available > 500.0 {
        // High attention: strengthen existing or form new couplings
        for coupling_id in resonator.couplings().await? {
            coupling.strengthen(0.1).await?;
        }
    } else if budget.available < 200.0 {
        // Low attention: weaken or decouple
        for coupling_id in resonator.couplings().await? {
            coupling.weaken(0.1).await?;
        }
    }

    tokio::time::sleep(Duration::from_secs(10)).await;
}
```

**Use cases:**
- Dynamic load balancing
- Resource-constrained agents
- Graceful degradation
- Adaptive coordination

### Pattern 4: Profile-Based Behavior

Different behavior based on profile:

```rust
match resonator.profile() {
    ResonatorProfile::Human => {
        // Human: Be patient, explain clearly, never coerce
        respond_with_patience().await?;
    }
    ResonatorProfile::World => {
        // World: Provide rich experiences, detect coercion
        provide_experience().await?;
    }
    ResonatorProfile::Coordination => {
        // Coordination: Efficient, explicit, accountable
        coordinate_efficiently().await?;
    }
    ResonatorProfile::IBank => {
        // IBank: Strict audit, risk assessment, bounded decisions
        execute_with_audit().await?;
    }
}
```

### Pattern 5: Continuity-Aware Startup

Handle both fresh starts and resumptions:

```rust
async fn get_or_create_resonator(
    runtime: &MapleRuntime,
    saved_id: Option<ResonatorId>,
) -> Result<ResonatorHandle> {

    match saved_id {
        Some(id) => {
            // Try to resume
            match runtime.resume_resonator(id).await {
                Ok(resonator) => {
                    println!("✅ Resumed: {}", id);
                    Ok(resonator)
                }
                Err(_) => {
                    // Resume failed, register new
                    println!("⚠️ Resume failed, creating new");
                    let spec = ResonatorSpec::default();
                    runtime.register_resonator(spec).await
                }
            }
        }
        None => {
            // Fresh start
            println!("🆕 Creating new Resonator");
            let spec = ResonatorSpec::default();
            runtime.register_resonator(spec).await
        }
    }
}
```

---

## Resonator Metadata

Attach arbitrary metadata to Resonators:

```rust
let spec = ResonatorSpec {
    profile: ResonatorProfile::Coordination,
    initial_attention: 1000.0,
    metadata: json!({
        "name": "AlphaTrader",
        "version": "2.1.0",
        "strategy": "momentum",
        "risk_tolerance": 0.7,
        "capabilities": ["trading", "analysis", "reporting"],
        "created_at": "2026-01-15T10:30:00Z"
    }),
};
```

**Use cases:**
- Agent discovery (find by capability)
- Configuration storage
- Versioning
- Debugging/monitoring
- Reputation tracking

---

## Best Practices

### ✅ DO

1. **Save identity for resumption**
   ```rust
   save_to_disk(&resonator.id)?;
   ```

2. **Monitor attention usage**
   ```rust
   if budget.available < threshold {
       decouple_or_weaken().await?;
   }
   ```

3. **Signal presence changes**
   ```rust
   when_going_offline(|| {
       signal_low_presence().await?;
   });
   ```

4. **Use appropriate profiles**
   ```rust
   // Financial agent = IBank profile
   spec.profile = ResonatorProfile::IBank;
   ```

5. **Handle resumption gracefully**
   ```rust
   match runtime.resume_resonator(id).await {
       Ok(r) => use_resumed(r),
       Err(_) => register_new(),
   }
   ```

### ❌ DON'T

1. **Don't assume unlimited attention**
   ```rust
   // ❌ BAD: Infinite coupling without checking
   loop {
       establish_coupling().await?;  // Will eventually fail
   }
   ```

2. **Don't signal presence too frequently**
   ```rust
   // ❌ BAD: Rate limit will block
   loop {
       signal_presence().await?;  // Every loop iteration
   }
   ```

3. **Don't ignore profile constraints**
   ```rust
   // ❌ BAD: Wrong profile for platform
   let spec = ResonatorSpec {
       profile: ResonatorProfile::Human,  // Not allowed in iBank
   };
   ```

4. **Don't lose identity**
   ```rust
   // ❌ BAD: Identity not saved
   let resonator = register().await?;
   // ... app exits, identity lost forever
   ```

5. **Don't mix profiles inappropriately**
   ```rust
   // ❌ BAD: Financial logic with Human profile
   if profile == Human {
       execute_million_dollar_trade()?;  // Wrong!
   }
   ```

---

## Related Concepts

- **[Coupling](coupling.md)** - Stateful relationships between Resonators
- **[Attention](attention.md)** - Resource economics and finite budgets
- **[Profiles](profiles.md)** - Different modes of operation
- **[Temporal Anchors](temporal.md)** - Causal ordering and time

---

**Next**: [Coupling →](coupling.md)
