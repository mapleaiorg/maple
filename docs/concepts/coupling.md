# 🔗 Coupling: Stateful Relationships

## What is Coupling?

**Coupling** is a stateful relationship between two Resonators. It is NOT a message channel or RPC endpoint - it's a persistent, evolving connection that has strength, scope, persistence, and attention cost.

```
Traditional:  Agent A --[message]--> Agent B
MAPLE:        Resonator A <==[coupling]==> Resonator B
                              ↑
                         [strength: 0.7]
                         [attention: 150.0]
                         [meaning: 0.85]
```

## Core Properties

### 1. Coupling Strength (0.0 - 1.0)

Strength represents how strongly two Resonators are coupled:

```rust
pub struct Coupling {
    pub strength: f64,  // 0.0 (no coupling) to 1.0 (maximum coupling)
    // ...
}
```

**Strength semantics:**
- `0.0` - No coupling (disconnected)
- `0.1-0.3` - Weak coupling (occasional interaction)
- `0.4-0.6` - Moderate coupling (regular interaction)
- `0.7-0.9` - Strong coupling (frequent interaction)
- `1.0` - Maximum coupling (continuous resonance)

**Architectural requirement:** Strength must increase **gradually**:
- Maximum initial strength: `0.3`
- Maximum strengthening per step: `0.1`
- Minimum weakening per step: `0.05`

```rust
// ✅ ALLOWED
let coupling = establish_coupling(params.with_strength(0.3)).await?;
coupling.strengthen(0.1).await?;  // Now 0.4
coupling.strengthen(0.1).await?;  // Now 0.5

// ❌ FORBIDDEN
let coupling = establish_coupling(params.with_strength(0.8)).await?;  // Too strong!
coupling.strengthen(0.5).await?;  // Too aggressive!
```

**Why gradual strengthening?**
- Prevents sudden resource consumption
- Allows time for meaning convergence
- Enables graceful relationship formation
- Reflects natural relationship building

### 2. Attention Cost

Every coupling **consumes attention** from the source Resonator:

```rust
let params = CouplingParams {
    source: alice.id,
    target: bob.id,
    initial_strength: 0.3,
    initial_attention_cost: 100.0,  // Attention bound to this coupling
    ..Default::default()
};

let coupling = runtime.establish_coupling(params).await?;

// Alice's attention reduced by 100.0
```

**Attention formula:**
```
total_attention_allocated = Σ(coupling.attention_cost for all couplings)
available_attention = total_capacity - total_allocated - safety_reserve
```

**Architectural Invariant #5:** `coupling.strength ≤ available_attention`

**What happens on decouple:**
```rust
coupling.decouple().await?;
// Attention released back to Alice (100.0 returned)
```

### 3. Persistence

Couplings have different lifetime semantics:

```rust
pub enum CouplingPersistence {
    Transient,    // Exists only in memory, lost on restart
    Session,      // Persists for the session, lost on runtime shutdown
    Persistent,   // Survives restarts, stored in database
}
```

**Examples:**

```rust
// Transient: Quick coordination
CouplingParams {
    persistence: CouplingPersistence::Transient,
    ..Default::default()
}

// Session: Multi-step task
CouplingParams {
    persistence: CouplingPersistence::Session,
    ..Default::default()
}

// Persistent: Long-term relationship
CouplingParams {
    persistence: CouplingPersistence::Persistent,
    ..Default::default()
}
```

### 4. Scope

What aspects of resonance are shared:

```rust
pub enum CouplingScope {
    Full,              // Full resonance (presence, meaning, intent, commitment)
    StateOnly,         // Only state shared
    IntentOnly,        // Only intent shared
    ObservationalOnly, // One-way observation (no participation)
}
```

**Scope semantics:**

| Scope | Presence | Meaning | Intent | Commitment |
|-------|----------|---------|--------|------------|
| Full | ✅ | ✅ | ✅ | ✅ |
| StateOnly | ✅ | ❌ | ❌ | ❌ |
| IntentOnly | ✅ | ✅ | ✅ | ❌ |
| ObservationalOnly | ✅ (one-way) | ❌ | ❌ | ❌ |

**Examples:**

```rust
// Full resonance: Close collaboration
CouplingParams {
    scope: CouplingScope::Full,
    ..Default::default()
}

// State monitoring: Just track state
CouplingParams {
    scope: CouplingScope::StateOnly,
    ..Default::default()
}

// Intent coordination: Align goals without commitment
CouplingParams {
    scope: CouplingScope::IntentOnly,
    ..Default::default()
}

// Passive observation: Watch without participation
CouplingParams {
    scope: CouplingScope::ObservationalOnly,
    ..Default::default()
}
```

### 5. Symmetry

Is the coupling bidirectional?

```rust
pub enum CouplingSymmetry {
    Symmetric,    // Both Resonators participate equally
    Asymmetric,   // Source initiates, target responds
}
```

**Symmetric coupling:**
```
Alice <===========> Bob
     (both active)
```

**Asymmetric coupling:**
```
Alice ============> Bob
     (Alice drives)
```

**Example:**

```rust
// Symmetric: Peer collaboration
CouplingParams {
    symmetry: CouplingSymmetry::Symmetric,
    ..Default::default()
}

// Asymmetric: Leader-follower
CouplingParams {
    symmetry: CouplingSymmetry::Asymmetric,
    ..Default::default()
}
```

### 6. Meaning Convergence

How well the two Resonators understand each other:

```rust
pub struct Coupling {
    pub meaning_convergence: f64,  // 0.0 (no understanding) to 1.0 (perfect alignment)
    // ...
}
```

**Convergence over time:**
```
Initial:     meaning_convergence = 0.1  (little shared context)
After 10m:   meaning_convergence = 0.4  (developing understanding)
After 1h:    meaning_convergence = 0.7  (good alignment)
After 1d:    meaning_convergence = 0.9  (strong understanding)
```

**Architectural requirement:** Intent requires `meaning_convergence ≥ 0.5`

```rust
if coupling.meaning_convergence < 0.5 {
    // Cannot form intent yet
    return Err(InvariantViolation::MeaningPrecedesIntent);
}
```

---

## Coupling Lifecycle

### 1. Establishment

```rust
// Both Resonators must be present first (Invariant #1)
alice.signal_presence(PresenceState::new()).await?;
bob.signal_presence(PresenceState::new()).await?;

// Establish coupling
let params = CouplingParams {
    source: alice.id,
    target: bob.id,
    initial_strength: 0.3,          // Max allowed initially
    initial_attention_cost: 150.0,  // Attention allocated
    persistence: CouplingPersistence::Session,
    scope: CouplingScope::Full,
    symmetry: CouplingSymmetry::Symmetric,
};

let coupling = runtime.establish_coupling(params).await?;
```

**What happens:**
1. Check presence (both must be present)
2. Allocate attention (check availability)
3. Create coupling in fabric
4. Update topology graph
5. Return coupling handle

### 2. Gradual Strengthening

```rust
// Strengthen over time
coupling.strengthen(0.1).await?;  // 0.3 → 0.4
tokio::time::sleep(Duration::from_secs(60)).await;

coupling.strengthen(0.1).await?;  // 0.4 → 0.5
tokio::time::sleep(Duration::from_secs(60)).await;

coupling.strengthen(0.1).await?;  // 0.5 → 0.6

// Cannot jump directly
// coupling.strengthen(0.5).await?;  // ❌ BLOCKED
```

**Why wait between strengthening?**
- Allows meaning to converge
- Gives time for interaction patterns to stabilize
- Prevents aggressive coupling

### 3. Meaning Convergence

As Resonators interact, meaning converges:

```rust
loop {
    let state = coupling.state().await?;

    println!("Strength: {}", state.strength);
    println!("Meaning: {}", state.meaning_convergence);

    if state.meaning_convergence >= 0.5 {
        println!("✅ Sufficient meaning for intent");
        break;
    }

    // Continue interacting...
    tokio::time::sleep(Duration::from_secs(10)).await;
}
```

### 4. Intent Formation

Once meaning converges, intent can be formed:

```rust
if coupling.meaning_convergence() >= 0.5 {
    // Now we can form shared intent
    let intent = form_shared_intent(&coupling).await?;
}
```

### 5. Weakening

Reduce coupling strength when less interaction needed:

```rust
// Reduce strength
coupling.weaken(0.1).await?;  // Strength reduced
// Attention is NOT released (still bound)

// Weaken further
coupling.weaken(0.2).await?;
```

### 6. Decoupling

End the relationship:

```rust
// Check for active commitments first
if coupling.has_active_commitments().await? {
    println!("⚠️ Cannot decouple: active commitments");
    // Must fulfill or revoke commitments first
    return Err(CouplingError::ActiveCommitments);
}

// Safe to decouple
coupling.decouple().await?;

// Attention released back to source
```

---

## Coupling Patterns

### Pattern 1: Progressive Strengthening

Build relationships gradually:

```rust
async fn build_relationship(
    runtime: &MapleRuntime,
    source: ResonatorId,
    target: ResonatorId,
) -> Result<CouplingHandle> {

    // Start weak
    let params = CouplingParams {
        source,
        target,
        initial_strength: 0.2,
        initial_attention_cost: 100.0,
        ..Default::default()
    };

    let coupling = runtime.establish_coupling(params).await?;

    // Strengthen over time as meaning converges
    for _ in 0..5 {
        tokio::time::sleep(Duration::from_secs(60)).await;

        let state = coupling.state().await?;
        if state.meaning_convergence > 0.6 {
            coupling.strengthen(0.1).await?;
        }
    }

    Ok(coupling)
}
```

### Pattern 2: Attention-Aware Coupling

Adapt coupling based on attention availability:

```rust
async fn maintain_coupling_budget(
    resonator: &ResonatorHandle,
    max_couplings: usize,
) -> Result<()> {

    loop {
        let budget = resonator.attention_status().await.unwrap();
        let couplings = resonator.couplings().await?;

        if budget.available < 100.0 && couplings.len() > 0 {
            // Low attention: weaken or decouple weakest
            let weakest = find_weakest_coupling(&couplings).await?;
            weakest.weaken(0.1).await?;
        } else if budget.available > 500.0 && couplings.len() < max_couplings {
            // High attention: can form new couplings
            discover_and_couple().await?;
        }

        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
```

### Pattern 3: Observational Learning

Learn from others without full participation:

```rust
// Observe without participating
let observer_params = CouplingParams {
    source: observer.id,
    target: expert.id,
    initial_strength: 0.1,
    initial_attention_cost: 50.0,  // Low cost
    scope: CouplingScope::ObservationalOnly,  // Just watch
    symmetry: CouplingSymmetry::Asymmetric,   // One-way
    ..Default::default()
};

let observation = runtime.establish_coupling(observer_params).await?;

// Learn from observations
loop {
    let observations = observe_through_coupling(&observation).await?;
    learn_from(observations).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

### Pattern 4: Dynamic Topology

Adapt coupling topology based on needs:

```rust
async fn maintain_optimal_topology(
    resonator: &ResonatorHandle,
) -> Result<()> {

    loop {
        let couplings = resonator.couplings().await?;

        // Find high-value couplings
        for coupling in &couplings {
            let state = coupling.state().await?;

            if state.meaning_convergence > 0.8 && state.strength < 0.7 {
                // High meaning, low strength: strengthen
                coupling.strengthen(0.1).await?;
            } else if state.meaning_convergence < 0.3 && state.strength > 0.3 {
                // Low meaning, high strength: weaken or decouple
                coupling.weaken(0.1).await?;
            }
        }

        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}
```

### Pattern 5: Commitment-Safe Decoupling

Safely end relationships:

```rust
async fn safe_decouple(coupling: &CouplingHandle) -> Result<()> {
    // Check for active commitments
    let commitments = coupling.active_commitments().await?;

    if !commitments.is_empty() {
        println!("⚠️ Active commitments found: {}", commitments.len());

        // Fulfill or revoke commitments first
        for commitment in commitments {
            match commitment.status().await? {
                CommitmentStatus::Active => {
                    // Attempt to fulfill
                    fulfill_commitment(&commitment).await?;
                }
                CommitmentStatus::Pending => {
                    // Revoke pending commitments
                    commitment.revoke().await?;
                }
                _ => {}
            }
        }
    }

    // Now safe to decouple
    coupling.decouple().await?;
    Ok(())
}
```

---

## Profile-Specific Coupling Rules

### Human-to-World Coupling (Finalverse)

```rust
// Human can always disengage (Invariant #7)
if source_profile == Human {
    // Human must explicitly accept coupling
    require_human_consent(&coupling).await?;

    // Coercion detection enabled
    monitor_for_coercion(&coupling).await?;

    // Human can reduce coupling at any time
    allow_immediate_weakening(&coupling).await?;
}
```

### Coordination-to-Coordination (Mapleverse)

```rust
// Pure AI: Explicit intent required
if source_profile == Coordination && target_profile == Coordination {
    // Must have stabilized intent
    require_stabilized_intent(&coupling).await?;

    // Commitment accountability
    require_commitment_for_consequences(&coupling).await?;
}
```

### IBank-to-IBank (iBank)

```rust
// Financial: Strict audit
if source_profile == IBank && target_profile == IBank {
    // Mandatory audit trail
    enable_audit_trail(&coupling).await?;

    // Risk assessment for coupling
    assess_coupling_risk(&coupling).await?;

    // Digital signature required
    sign_coupling_establishment(&coupling).await?;
}
```

---

## Coupling Topology

MAPLE maintains a **directed, weighted graph** of all couplings:

```
      [0.5]
Alice -----> Bob
  |           |
  | [0.3]     | [0.7]
  |           |
  ↓           ↓
Carol <---- Dave
      [0.4]
```

**Query topology:**

```rust
// Get all couplings for a Resonator
let couplings = runtime.get_couplings_for(alice.id).await?;

// Find strongest coupling
let strongest = couplings.iter()
    .max_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap());

// Find couplings by strength threshold
let strong_couplings: Vec<_> = couplings.iter()
    .filter(|c| c.strength > 0.7)
    .collect();

// Get coupling path between two Resonators
let path = runtime.find_coupling_path(alice.id, dave.id).await?;
```

---

## Best Practices

### ✅ DO

1. **Strengthen gradually**
   ```rust
   coupling.strengthen(0.1).await?;  // Good
   ```

2. **Check attention before coupling**
   ```rust
   if budget.available >= attention_cost {
       establish_coupling().await?;
   }
   ```

3. **Wait for meaning convergence**
   ```rust
   if coupling.meaning_convergence() >= 0.5 {
       form_intent().await?;
   }
   ```

4. **Decouple safely**
   ```rust
   fulfill_commitments().await?;
   coupling.decouple().await?;
   ```

5. **Monitor coupling health**
   ```rust
   if coupling.meaning_convergence() < 0.3 {
       weaken_or_decouple().await?;
   }
   ```

### ❌ DON'T

1. **Don't strengthen too aggressively**
   ```rust
   // ❌ BAD
   coupling.strengthen(0.5).await?;  // Too much!
   ```

2. **Don't couple without attention**
   ```rust
   // ❌ BAD
   // Ignoring attention budget will cause failure
   ```

3. **Don't decouple with active commitments**
   ```rust
   // ❌ BAD
   coupling.decouple().await?;  // Commitments still active!
   ```

4. **Don't couple without presence**
   ```rust
   // ❌ BAD
   establish_coupling().await?;  // Neither is present!
   ```

5. **Don't forget to release attention**
   ```rust
   // ❌ BAD
   // Coupling exists but unused - attention still bound
   ```

---

## Related Concepts

- **[Resonators](resonators.md)** - Persistent intelligent entities
- **[Attention](attention.md)** - Resource economics
- **[Commitments](commitments.md)** - Accountability system
- **[Temporal Anchors](temporal.md)** - Causal ordering

---

**Next**: [Attention →](attention.md)
