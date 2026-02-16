# 🏗️ MAPLE Architecture Overview

> For the WorldLine-first architecture set, see:
> [`docs/architecture/00-overview.md`](architecture/00-overview.md),
> [`docs/architecture/01-worldline.md`](architecture/01-worldline.md),
> [`docs/architecture/03-commitment-boundary.md`](architecture/03-commitment-boundary.md),
> [`docs/architecture/04-ledger-wll.md`](architecture/04-ledger-wll.md).

## The Resonance Paradigm Shift

MAPLE represents a **fundamental rethinking** of how multi-agent AI systems should work. Rather than treating agents as isolated processes that exchange messages, MAPLE treats every entity as a **Resonator** participating in continuous, stateful **resonance**.

### Traditional Agent Frameworks

```
┌─────────┐     message     ┌─────────┐     message     ┌─────────┐
│ Agent A │ ──────────────> │ Agent B │ ──────────────> │ Agent C │
└─────────┘                 └─────────┘                 └─────────┘
     ↓                            ↓                           ↓
 Stateless                    Stateless                  Stateless
 Isolated                     Isolated                   Isolated
 Ephemeral                    Ephemeral                  Ephemeral
```

**Problems:**
- No persistent relationships
- No resource bounds
- No safety guarantees
- No accountability
- Binary presence (online/offline)

### MAPLE Resonance Architecture

```
┌──────────────┐  coupling  ┌──────────────┐  coupling  ┌──────────────┐
│ Resonator A  │ <========> │ Resonator B  │ <========> │ Resonator C  │
│              │            │              │            │              │
│ [presence]   │            │ [presence]   │            │ [presence]   │
│      ↓       │            │      ↓       │            │      ↓       │
│ [meaning] ───┼───────────>│ [meaning] ───┼───────────>│ [meaning]    │
│      ↓       │            │      ↓       │            │      ↓       │
│ [intent] ────┼───────────>│ [commitment] │───────────>│ [consequence]│
└──────────────┘            └──────────────┘            └──────────────┘
      ↓                            ↓                            ↓
   Stateful                     Stateful                     Stateful
   Relationship                 Relationship                 Relationship
   Persistent                   Persistent                   Persistent
```

**Advantages:**
- ✅ Stateful relationships that evolve
- ✅ Finite attention bounds
- ✅ Architectural safety guarantees
- ✅ Full accountability
- ✅ Gradient presence

---

## The Core Flow

Every interaction in MAPLE follows this architectural flow:

```
PRESENCE → COUPLING → MEANING → INTENT → COMMITMENT → CONSEQUENCE
```

### 1. Presence

**Gradient, multidimensional representation** (NOT binary online/offline).

```rust
pub struct PresenceState {
    pub discoverability: f64,      // How findable (0.0-1.0)
    pub responsiveness: f64,        // How quick to respond (0.0-1.0)
    pub stability: f64,             // How consistently available (0.0-1.0)
    pub coupling_readiness: f64,    // Willing to form couplings (0.0-1.0)
    pub last_signal: TemporalAnchor,
    pub silent_mode: bool,          // Present but not actively signaling
}
```

**Why gradient?**
- More expressive than binary
- Enables graceful degradation
- Reflects real-world availability
- Supports silent observers

### 2. Coupling

**Stateful relationships** (NOT ephemeral message channels).

```rust
pub struct Coupling {
    pub id: CouplingId,
    pub source: ResonatorId,
    pub target: ResonatorId,
    pub strength: f64,              // 0.0-1.0, must strengthen gradually
    pub persistence: CouplingPersistence,  // Transient/Session/Persistent
    pub scope: CouplingScope,       // Full/StateOnly/IntentOnly/Observational
    pub symmetry: CouplingSymmetry, // Symmetric/Asymmetric
    pub attention_cost: f64,        // Attention bound to this coupling
    pub meaning_convergence: f64,   // How well understanding aligns
}
```

**Key properties:**
- **Gradual strengthening**: Max 0.3 initial, 0.1 per step
- **Attention bounded**: Coupling ≤ available attention
- **Meaning tracking**: Convergence measured over time
- **Safe decoupling**: Without violating commitments

### 3. Meaning

**Semantic understanding** that converges over time.

Meaning is NOT transmitted fully-formed. It emerges through resonance:
- Context accumulates through interactions
- Understanding deepens gradually
- Misalignment is detected and corrected
- Convergence is tracked (0.0-1.0)

**Architectural requirement**: Sufficient meaning must exist before intent can form.

### 4. Intent

**Stabilized goals** formed from sufficient meaning.

Intent is NOT immediate. It requires:
- Sufficient meaning convergence (threshold: 0.5)
- Temporal stability (not fleeting)
- Consistency with previous intentions
- Validation against safety boundaries

**Architectural requirement**: Intent must stabilize before commitments can be made.

### 5. Commitment

**Explicit promises** with audit trails.

```rust
pub struct Commitment {
    pub id: CommitmentId,
    pub resonator: ResonatorId,
    pub content: CommitmentContent,
    pub status: CommitmentStatus,
    pub audit_trail: Vec<AuditEntry>,
    pub risk_assessment: Option<RiskAssessment>,
    pub reversibility: bool,
}
```

Every commitment:
- Has a complete audit trail
- Requires risk assessment (if consequential)
- Tracks reversibility
- Is digitally signed (non-repudiation)

**Architectural requirement**: No consequence without explicit commitment.

### 6. Consequence

**Attributable outcomes** from commitments.

Every consequential action:
- Links back to a commitment
- Has full audit trail
- Is attributable to a Resonator
- Can be traced through the chain

**Architectural requirement**: Failure must be explicit (never hidden).

---

## The 8 Architectural Invariants

These invariants are **enforced at runtime**. Violations = system errors.

### Invariant 1: Presence Precedes Meaning

```
✓ ALLOWED:  Present → Form/Receive Meaning
✗ FORBIDDEN: Form/Receive Meaning without Presence
```

**Why?** You cannot participate in resonance without being present.

**Example violation:**
```rust
// Resonator has NOT signaled presence
let meaning = resonator.form_meaning(...).await?;  // ❌ BLOCKED
```

### Invariant 2: Meaning Precedes Intent

```
✓ ALLOWED:  Sufficient Meaning → Form Intent
✗ FORBIDDEN: Form Intent without Sufficient Meaning
```

**Why?** Intent without understanding is dangerous.

**Threshold:** Meaning convergence ≥ 0.5 required.

### Invariant 3: Intent Precedes Commitment

```
✓ ALLOWED:  Stabilized Intent → Make Commitment
✗ FORBIDDEN: Make Commitment without Stabilized Intent
```

**Why?** Commitments must be based on clear goals.

**Requirement:** Intent must be temporally stable (not fleeting).

### Invariant 4: Commitment Precedes Consequence

```
✓ ALLOWED:  Explicit Commitment → Produce Consequence
✗ FORBIDDEN: Produce Consequence without Explicit Commitment
```

**Why?** Every consequential action must be attributable.

**This enables:** Full accountability and audit trails.

### Invariant 5: Coupling Bounded by Attention

```
✓ ALLOWED:  Coupling Strength ≤ Available Attention
✗ FORBIDDEN: Coupling that exceeds attention budget
```

**Why?** Prevents runaway resource consumption.

**Formula:** `total_allocated_attention ≤ total_capacity`

### Invariant 6: Safety Overrides Optimization

```
✓ ALLOWED:  Sacrifice Performance for Safety
✗ FORBIDDEN: Bypass Safety for Performance
```

**Why?** Safety is non-negotiable.

**Example:** Rate limiting presence signals even if it slows throughput.

### Invariant 7: Human Agency Cannot Be Bypassed

```
✓ ALLOWED:  Humans Can Always Disengage
✗ FORBIDDEN: Forced Coupling with Human
```

**Why?** Architectural protection of human autonomy.

**Mechanisms:**
- Presence does NOT imply willingness
- Humans can always reduce coupling
- Coercion detection
- Emotional exploitation prevention

### Invariant 8: Failure Must Be Explicit

```
✓ ALLOWED:  Surface All Failures
✗ FORBIDDEN: Silent Failures or Hidden Errors
```

**Why?** Reliability requires transparency.

**Enforcement:** All operations return Result types, no panics in production.

---

## Core Components

### MapleRuntime

The central orchestrator:

```rust
pub struct MapleRuntime {
    // Resonator Management
    resonator_registry: ResonatorRegistry,
    profile_manager: ProfileManager,

    // Resonance Infrastructure
    presence_fabric: PresenceFabric,
    coupling_fabric: CouplingFabric,
    attention_allocator: AttentionAllocator,

    // Safety and Governance
    invariant_guard: InvariantGuard,
    agency_protector: HumanAgencyProtector,

    // Temporal Coordination
    temporal_coordinator: TemporalCoordinator,

    // Scheduling
    scheduler: ResonanceScheduler,

    // Telemetry
    telemetry: RuntimeTelemetry,
}
```

**Responsibilities:**
- Bootstrap and shutdown orchestration
- Resonator lifecycle management
- Subsystem coordination
- Safety enforcement

### PresenceFabric

Manages gradient presence for all Resonators:

```rust
pub struct PresenceFabric {
    presence_states: DashMap<ResonatorId, PresenceState>,
    rate_limiter: RateLimiter,
    config: PresenceConfig,
}
```

**Features:**
- Multidimensional presence tracking
- Rate limiting (prevent spam)
- Silent mode support
- Presence queries by dimensions

**Key methods:**
```rust
async fn signal_presence(&self, id: ResonatorId, state: PresenceState) -> Result<()>;
async fn get_presence(&self, id: ResonatorId) -> Option<PresenceState>;
async fn query_by_discoverability(&self, min: f64) -> Vec<ResonatorId>;
```

### CouplingFabric

Manages the topology of all couplings:

```rust
pub struct CouplingFabric {
    couplings: DashMap<CouplingId, CouplingState>,
    topology: DashMap<ResonatorId, Vec<CouplingId>>,
    attention: Arc<AttentionAllocator>,
    config: CouplingConfig,
}
```

**Features:**
- Directed, weighted coupling graph
- Gradual strengthening enforcement
- Attention-bounded coupling
- Meaning convergence tracking
- Safe decoupling

**Key methods:**
```rust
async fn establish_coupling(&self, params: CouplingParams)
    -> Result<(CouplingId, AllocationToken)>;

async fn strengthen_coupling(&self, id: CouplingId, delta: f64)
    -> Result<()>;

async fn decouple(&self, id: CouplingId) -> Result<()>;
```

**Invariants enforced:**
- Initial strength ≤ max_initial_strength (0.3)
- Strengthening step ≤ max_strengthening_step (0.1)
- Total strength ≤ 1.0
- Attention allocated before coupling created

### AttentionAllocator

Manages finite attention budgets:

```rust
pub struct AttentionAllocator {
    budgets: DashMap<ResonatorId, AttentionBudget>,
    config: AttentionConfig,
}

pub struct AttentionBudget {
    pub total_capacity: f64,
    pub allocated: f64,
    pub available: f64,
    pub safety_reserve: f64,
}
```

**Features:**
- Finite capacity per Resonator
- Safety reserves (can't allocate last 10%)
- Allocation tracking
- Automatic release on decouple
- Exhaustion detection

**Key methods:**
```rust
async fn allocate(&self, id: ResonatorId, amount: f64)
    -> Result<AllocationToken>;

async fn release(&self, token: AllocationToken) -> Result<()>;

async fn rebalance(&self, id: ResonatorId) -> Result<()>;
```

**Why attention economics?**
- Prevents unlimited coupling
- Enables graceful degradation
- Detects attention exhaustion attacks
- Creates natural bounds

### InvariantGuard

Enforces the 9 canonical WorldLine invariants:

```rust
pub struct InvariantGuard {
    enabled_invariants: HashSet<ArchitecturalInvariant>,
}

pub enum ArchitecturalInvariant {
    PresencePrecedesMeaning,
    MeaningPrecedesIntent,
    IntentPrecedesCommitment,
    CommitmentPrecedesConsequence,
    CouplingBoundedByAttention,
    SafetyOverridesOptimization,
    HumanAgencyCannotBeBypassed,
    FailureMustBeExplicit,
    ImplementationProvenanceAndEvolution,
}
```

**Enforcement:**
```rust
pub fn check(&self, operation: &Operation, state: &SystemState)
    -> Result<(), InvariantViolation> {

    for invariant in &self.enabled_invariants {
        self.check_invariant(*invariant, operation, state)?;
    }
    Ok(())
}
```

**Critical:** Violations cause operations to fail immediately.

### TemporalCoordinator

Causal ordering **without global clocks**:

```rust
pub struct TemporalCoordinator {
    anchors: DashMap<ResonatorId, Vec<TemporalAnchor>>,
    causal_graph: DashMap<AnchorId, Vec<AnchorId>>,
}

pub struct TemporalAnchor {
    pub id: AnchorId,
    pub event: Event,
    pub local_timestamp: i64,
    pub dependencies: Vec<AnchorId>,
}
```

**Key insight:** Time is relational, not absolute.

**Features:**
- Happened-before relationships
- Causal dependency tracking
- Local timelines per Resonator
- No synchronized clocks required

**Why?**
- Distributed systems don't have global time
- Causal ordering is what matters
- Local clocks are sufficient
- More scalable than consensus

### ResonanceScheduler

Attention-aware task scheduling:

```rust
pub struct ResonanceScheduler {
    queues: HashMap<AttentionClass, PriorityQueue<Task>>,
    circuit_breakers: HashMap<AttentionClass, CircuitBreaker>,
}

pub enum AttentionClass {
    Critical,
    High,
    Normal,
    Low,
    Background,
}
```

**Features:**
- Priority queues by attention class
- Circuit breakers for overload
- Graceful degradation
- Fairness guarantees

**Scheduling policy:**
1. Critical tasks always run (safety)
2. High tasks run unless circuit broken
3. Normal tasks run with fair scheduling
4. Low tasks run when capacity available
5. Background tasks run opportunistically

---

## Platform Configurations

MAPLE supports three distinct platform configurations with different safety constraints.

### Mapleverse - Pure AI Coordination

**Target:** 100M+ concurrent AI agents coordinating autonomously.

**Configuration:**
```rust
pub fn mapleverse_runtime_config() -> RuntimeConfig {
    RuntimeConfig {
        profiles: ProfileConfig {
            human_profiles_allowed: false,  // No humans
            allowed_profiles: vec![ResonatorProfile::Coordination],
        },
        coupling: CouplingConfig {
            require_explicit_intent: true,
            require_commitment_for_state_change: true,
        },
        commitment: CommitmentConfig {
            require_audit_trail: true,
            require_digital_signature: true,
        },
        // ...
    }
}
```

**Characteristics:**
- ✅ AI-only (no human Resonators)
- ✅ Strong commitment accountability
- ✅ Explicit coupling and intent
- ✅ Federated collective intelligence
- ✅ Optimized for extreme scale

**Use cases:**
- Autonomous agent swarms
- Distributed AI coordination
- Multi-agent reinforcement learning
- Agent marketplaces

### Finalverse - Human-AI Coexistence

**Target:** Meaningful experiences where humans and AI coexist.

**Configuration:**
```rust
pub fn finalverse_runtime_config() -> RuntimeConfig {
    RuntimeConfig {
        profiles: ProfileConfig {
            human_profiles_allowed: true,
            allowed_profiles: vec![
                ResonatorProfile::Human,
                ResonatorProfile::World,
            ],
        },
        safety: SafetyConfig {
            human_agency_protection: true,
            coercion_detection: true,
            emotional_exploitation_prevention: true,
            reversible_consequences_preferred: true,
        },
        // ...
    }
}
```

**Characteristics:**
- ✅ Architectural human agency protection
- ✅ Coercion detection enabled
- ✅ Emotional exploitation prevention
- ✅ Reversible consequences preferred
- ✅ Experiential focus

**Human protection mechanisms:**
1. **Presence ≠ Willingness**: Presence does not imply consent to interact
2. **Always Disengageable**: Humans can always reduce coupling
3. **Coercion Detection**: Patterns of manipulation detected
4. **Emotional Exploitation**: Prevented architecturally
5. **Silent Mode**: Observe without participating

**Use cases:**
- Virtual worlds
- AI companions
- Interactive storytelling
- Educational environments
- Therapeutic applications

### iBank - Autonomous AI Finance

**Target:** AI-only autonomous financial system.

**Configuration:**
```rust
pub fn ibank_runtime_config() -> RuntimeConfig {
    RuntimeConfig {
        profiles: ProfileConfig {
            human_profiles_allowed: false,  // No humans
            allowed_profiles: vec![ResonatorProfile::IBank],
        },
        commitment: CommitmentConfig {
            require_audit_trail: true,
            require_risk_assessment: true,
            require_digital_signature: true,
        },
        consequence: ConsequenceConfig {
            maximum_autonomous_consequence_value: 1_000_000.0,  // $1M
            require_reversibility_assessment: true,
        },
        // ...
    }
}
```

**Characteristics:**
- ✅ AI-only (no human Resonators)
- ✅ Mandatory audit trails
- ✅ Risk assessments required
- ✅ Risk-bounded decisions ($1M limit)
- ✅ Strict accountability

**Financial safety:**
1. **Audit Trails**: Every transaction fully logged
2. **Digital Signatures**: Non-repudiation
3. **Risk Assessment**: Mandatory for all operations
4. **Risk Bounds**: $1M autonomous limit
5. **Reversibility**: Preferred where possible

**Use cases:**
- Autonomous trading systems
- AI-managed portfolios
- Decentralized finance
- Algorithmic market making
- Risk management

---

## Comparison with Competitors

### vs. Google A2A (Agent-to-Agent)

| Aspect | Google A2A | MAPLE |
|--------|-----------|-------|
| **Core Model** | Tool invocation | **Resonance relationships** |
| **Identity** | Ephemeral sessions | **Persistent continuity** |
| **Relationships** | Point-to-point calls | **Dynamic coupling** |
| **Semantics** | Function signatures | **Emergent meaning** |
| **Resource Management** | None | **Attention economics** |
| **Accountability** | None | **Commitment ledger** |
| **Learning** | None | **Federated intelligence** |
| **Safety** | Policy-based | **Architectural invariants** |
| **Human Protection** | Implicit trust | **Explicit preservation** |
| **Scale Target** | Thousands | **100M+ Resonators** |

**Key differences:**
- A2A treats agents as RPC endpoints; MAPLE treats them as persistent entities
- A2A has no resource bounds; MAPLE has attention economics
- A2A has no safety guarantees; MAPLE has 9 architectural invariants
- A2A has no accountability; MAPLE has full audit trails

### vs. Anthropic MCP (Model Context Protocol)

| Aspect | Anthropic MCP | MAPLE |
|--------|---------------|-------|
| **Core Model** | Context injection | **Resonators** |
| **Agent Model** | Stateless tools | **Stateful entities** |
| **Relationships** | None | **Coupling fabric** |
| **Safety** | Policy-based | **Architectural invariants** |
| **Human Protection** | Implicit trust | **Architectural guarantees** |
| **Learning** | None | **Federated intelligence** |
| **Accountability** | None | **Full commitment ledger** |
| **Resource Bounds** | None | **Attention economics** |
| **Temporal Model** | None | **Causal ordering** |
| **Scale Target** | Hundreds | **100M+ Resonators** |

**Key differences:**
- MCP provides context to models; MAPLE creates persistent relationships
- MCP has no safety architecture; MAPLE has 9 invariants
- MCP has no resource management; MAPLE has attention bounds
- MCP has no accountability; MAPLE has full audit trails

---

## Performance Characteristics

### Target Metrics

| Metric | Target | Status |
|--------|--------|--------|
| Resonator Registration | <1ms | ✅ |
| Resonator Resume | <5ms | ✅ |
| Coupling Establishment | <5ms | ✅ |
| Coupling Strengthening | <1ms | ✅ |
| Attention Allocation | <100μs | ✅ |
| Invariant Check | <10μs | ✅ |
| Presence Signal | <500μs | ✅ |
| Concurrent Resonators (per node) | 100,000+ | ✅ |
| Total Scale | 100M+ | 🎯 |

### Scalability Strategy

**Single node:**
- 100,000+ concurrent Resonators
- Async I/O throughout
- Lock-free data structures (DashMap)
- Zero-copy operations where possible

**Multi-node:**
- Federated coupling across nodes
- Distributed temporal coordination
- Consistent hashing for Resonator placement
- Cross-runtime resonance

**Database:**
- PostgreSQL for persistence
- Continuity records
- Audit trails
- Temporal anchor storage

---

## Design Principles

### 1. Resonance Over Messages

Traditional systems pass messages. MAPLE creates **stateful relationships** that evolve over time.

### 2. Architecture Over Policy

Safety through **architectural invariants**, not policies that can be bypassed.

### 3. Attention Over Unlimited

**Finite attention** creates natural bounds, prevents abuse, enables graceful degradation.

### 4. Commitment Over Implicit

**Every consequential action** requires explicit commitment with audit trail.

### 5. Gradient Over Binary

Presence, coupling strength, meaning convergence - all are **gradients**, not binaries.

### 6. Causal Over Clock

**Relational time** through causal ordering, no global clock required.

### 7. Agency Over Trust

**Architectural protection** of human agency, not trusting policies.

### 8. Scale Over Optimization

Designed from day one for **100M+ concurrent Resonators**.

---

## Next Steps

- **[Getting Started](getting-started.md)** - Build your first MAPLE application
- **[Core Concepts](concepts/)** - Deep dives into key concepts
- **[Platform Guides](platforms/)** - Platform-specific documentation
- **[Contributing](../CONTRIBUTING.md)** - Join the MAPLE community

---

**Built with 🍁 by the MAPLE Team**

*Making AI agents that resonate, not just respond*
