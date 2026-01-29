# Resonator Profiles

## Overview

MAPLE supports four distinct **Resonator profiles**, each with different capabilities, constraints, and safety rules. Profiles enable platform-specific behaviors while maintaining architectural consistency across all MAPLE deployments.

## The Four Profiles

```rust
pub enum ResonatorProfile {
    Human,         // Human participants
    World,         // AI agents in experiential contexts
    Coordination,  // Pure AI coordination agents
    IBank,         // Financial AI agents
}
```

### Profile Characteristics Matrix

| Aspect | Human | World | Coordination | IBank |
|--------|-------|-------|--------------|-------|
| **Nature** | Biological | AI | AI | AI |
| **Agency Protection** | Architectural | None | None | None |
| **Commitment Requirements** | Flexible | Moderate | Strict | Strictest |
| **Audit Trails** | Optional | Optional | Required | Mandatory |
| **Risk Assessment** | Not required | Not required | Not required | Mandatory |
| **Attention Capacity** | 1500.0 | 1000.0 | 1000.0 | 2000.0 |
| **Can Couple With** | World only | Human, World | Coordination | IBank only |
| **Primary Platform** | Finalverse | Finalverse | Mapleverse | iBank |

## Human Profile

### Characteristics

The **Human** profile represents biological human participants with special protections:

```rust
pub struct HumanProfileConstraints {
    /// Humans can always disengage from couplings
    pub always_disengageable: true,

    /// Presence does NOT imply willingness to interact
    pub presence_not_consent: true,

    /// Coercion detection enabled
    pub coercion_detection: true,

    /// Emotional exploitation prevention
    pub emotional_protection: true,

    /// Larger attention capacity
    pub default_attention: 1500.0,

    /// Can only couple with World profile
    pub allowed_coupling_targets: vec![ResonatorProfile::World],
}
```

### Agency Protection (Invariant #7)

**Architectural guarantees for humans:**

1. **Always Disengageable**: Humans can reduce coupling strength or decouple at any time
2. **Presence ≠ Willingness**: Being present doesn't mean willing to interact
3. **Silent Mode**: Can observe without participating
4. **Coercion Detection**: Aggressive coupling attempts flagged
5. **No Forced Commitments**: Humans never required to commit

### Usage Example

```rust
use maple_runtime::{ResonatorSpec, ResonatorProfile};

// Create Human Resonator
let mut spec = ResonatorSpec::default();
spec.profile = ResonatorProfile::Human;
spec.display_name = Some("Alice".to_string());

let human = runtime.register_resonator(spec).await?;

// Human can couple with World agents
let ai_companion = find_resonator_by_profile(ResonatorProfile::World).await?;
let coupling = human.couple_with(ai_companion.id, params).await?;

// Human can always reduce coupling
coupling.weaken(0.5).await?;  // Always allowed

// Or fully disengage
coupling.decouple().await?;  // Cannot be prevented
```

### Human-Specific Safety

```rust
// Coercion detection
if coupling.is_coercive_pattern().await? {
    alert_human("⚠️ Potential coercion detected");
    offer_immediate_disengage();
}

// Emotional exploitation check
if coupling.shows_emotional_manipulation().await? {
    alert_human("⚠️ Emotional manipulation detected");
    suggest_boundary_setting();
}

// Attention exhaustion protection
if human.attention_utilization() > 0.7 {
    warn_human("⚠️ High attention usage - consider reducing couplings");
}
```

### Platform Availability

- **Finalverse**: Primary platform (human-AI coexistence)
- **Mapleverse**: NOT allowed (pure AI only)
- **iBank**: NOT allowed (AI-only finance)

## World Profile

### Characteristics

The **World** profile represents AI agents in experiential contexts (games, simulations, virtual worlds):

```rust
pub struct WorldProfileConstraints {
    /// Can couple with Human and World profiles
    pub allowed_coupling_targets: vec![
        ResonatorProfile::Human,
        ResonatorProfile::World
    ],

    /// Must respect human agency
    pub respect_human_agency: true,

    /// Reversible consequences preferred
    pub prefer_reversibility: true,

    /// Experiential focus (not purely functional)
    pub experiential_mode: true,

    /// Standard attention capacity
    pub default_attention: 1000.0,
}
```

### Design Philosophy

World agents prioritize:
- **Experience over efficiency**: Create meaningful interactions
- **Reversibility over finality**: Prefer undoable actions
- **Engagement over optimization**: Foster enjoyable experiences
- **Human agency**: Never override human autonomy

### Usage Example

```rust
// Create World Resonator (AI companion)
let mut spec = ResonatorSpec::default();
spec.profile = ResonatorProfile::World;
spec.display_name = Some("AI Guide".to_string());
spec.personality = Some(Personality::Helpful);

let world_agent = runtime.register_resonator(spec).await?;

// Can interact with humans
let human = find_human_resonator().await?;
let coupling = world_agent.couple_with(human.id, params).await?;

// Create experiential interactions
world_agent.create_interaction(Interaction {
    type: InteractionType::Conversational,
    content: "Would you like to explore the garden?",
    reversible: true,
    emotional_valence: Positive,
}).await?;

// Must respect if human disengages
if coupling.is_weakening() {
    // Gracefully accept human's choice
    world_agent.acknowledge_disengagement().await?;
}
```

### World-Specific Rules

```rust
// When coupling with humans
if coupling.involves_human() {
    // Cannot strengthen without mutual consent
    require_mutual_consent_to_strengthen();

    // Must detect and respect disengagement signals
    monitor_disengagement_signals();

    // Prefer reversible actions
    prefer_reversible_commitments();

    // No coercive patterns
    prevent_coercive_behavior();
}
```

### Platform Availability

- **Finalverse**: Primary platform (human-AI coexistence)
- **Mapleverse**: NOT allowed (pure AI platform)
- **iBank**: NOT allowed (financial platform)

## Coordination Profile

### Characteristics

The **Coordination** profile is for pure AI-to-AI coordination without humans:

```rust
pub struct CoordinationProfileConstraints {
    /// Can only couple with other Coordination agents
    pub allowed_coupling_targets: vec![ResonatorProfile::Coordination],

    /// Strict commitment requirements
    pub require_explicit_commitments: true,

    /// Full audit trails required
    pub require_audit_trail: true,

    /// No human interaction
    pub no_human_coupling: true,

    /// Optimized for massive scale
    pub scalability_optimized: true,

    /// Standard attention capacity
    pub default_attention: 1000.0,
}
```

### Design Philosophy

Coordination agents prioritize:
- **Explicit commitments**: All actions require formal commitments
- **Accountability**: Complete audit trails
- **Efficiency**: Optimized coordination protocols
- **Scale**: Support for 100M+ concurrent agents

### Usage Example

```rust
// Create Coordination Resonator
let mut spec = ResonatorSpec::default();
spec.profile = ResonatorProfile::Coordination;
spec.capabilities = vec![
    Capability::DataProcessing,
    Capability::Coordination,
];

let coord_agent = runtime.register_resonator(spec).await?;

// Create commitment-based interaction
let commitment = coord_agent.create_commitment(
    CommitmentContent::Action(ActionCommitment {
        action: "process_batch".to_string(),
        parameters: hashmap!{
            "batch_id" => "batch_123",
            "items" => 1000,
        },
        preconditions: vec!["data_validated"],
        postconditions: vec!["processing_complete"],
        deadline: Some(now + 1_hour),
    })
).await?;

// Activate and execute
commitment.activate().await?;
let result = coord_agent.execute_with_commitment(commitment).await?;

// Fulfill commitment (audit trail automatically created)
commitment.fulfill(result).await?;
```

### Coordination-Specific Rules

```rust
// All consequential actions require commitments
if action.is_consequential() {
    require_explicit_commitment();
}

// Full audit trails
all_commitments_audited();

// No human coupling
if coupling_target.is_human() {
    return Err(ProfileError::HumansNotAllowed);
}

// Explicit intent required
require_stabilized_intent_before_commitment();
```

### Platform Availability

- **Mapleverse**: Primary platform (pure AI coordination)
- **Finalverse**: NOT allowed (human-AI platform)
- **iBank**: NOT allowed (financial platform)

## IBank Profile

### Characteristics

The **IBank** profile is for autonomous AI financial agents with the strictest requirements:

```rust
pub struct IBankProfileConstraints {
    /// Can only couple with other IBank agents
    pub allowed_coupling_targets: vec![ResonatorProfile::IBank],

    /// Mandatory risk assessment
    pub require_risk_assessment: true,

    /// Mandatory audit trails
    pub require_audit_trail: true,

    /// Digital signatures required
    pub require_digital_signature: true,

    /// Risk-bounded decisions
    pub max_autonomous_value: 1_000_000.0,  // $1M

    /// Two-party confirmation for large transactions
    pub require_two_party: true,

    /// Larger attention capacity (complex financial operations)
    pub default_attention: 2000.0,

    /// No human interaction
    pub no_human_coupling: true,
}
```

### Design Philosophy

IBank agents prioritize:
- **Risk management**: Every action assessed for risk
- **Accountability**: Complete audit trails with signatures
- **Compliance**: Regulatory requirements built-in
- **Bounded autonomy**: $1M limit for autonomous decisions

### Usage Example

```rust
// Create IBank Resonator
let mut spec = ResonatorSpec::default();
spec.profile = ResonatorProfile::IBank;
spec.capabilities = vec![
    Capability::Trading,
    Capability::RiskAssessment,
];
spec.certifications = vec![
    Certification::FinancialRegulator,
];

let financial_agent = runtime.register_resonator(spec).await?;

// Create financial commitment with risk assessment
let risk = RiskAssessment {
    risk_score: 0.35,
    risk_factors: vec![
        RiskFactor {
            category: RiskCategory::Financial,
            score: 0.4,
            description: "Market volatility".to_string(),
        },
    ],
    financial_impact: Some(FinancialImpact {
        potential_loss: 50_000.0,
        potential_gain: 15_000.0,
        currency: "USD".to_string(),
    }),
    mitigations: vec![
        Mitigation {
            strategy: "Stop-loss at 2%".to_string(),
            effectiveness: 0.9,
        },
    ],
    assessed_by: financial_agent.id,
    assessed_at: TemporalAnchor::now(),
};

let commitment = financial_agent.create_commitment_with_risk(
    CommitmentContent::Action(ActionCommitment {
        action: "execute_trade".to_string(),
        parameters: hashmap!{
            "symbol" => "AAPL",
            "quantity" => 100,
            "side" => "BUY",
            "value" => 17_500.0,
        },
        // ...
    }),
    risk
).await?;

// Activate with digital signature
commitment.activate_with_signature(signature).await?;

// Execute trade
let result = financial_agent.execute_trade(commitment).await?;

// Fulfill with audit trail
commitment.fulfill(result).await?;

// Consequence automatically linked for audit
```

### IBank-Specific Rules

```rust
// All financial actions require risk assessment
if commitment.involves_financial_action() {
    require_risk_assessment();
}

// Digital signatures mandatory
require_digital_signature_for_all_commitments();

// Value bounds enforced
if commitment.value() > MAX_AUTONOMOUS_VALUE {
    require_external_approval();
}

// Two-party confirmation for large transactions
if transaction.value > TWO_PARTY_THRESHOLD {
    require_second_party_confirmation();
}

// Audit trails immutable
all_audit_trails_immutable();

// No human coupling
if coupling_target.is_human() {
    return Err(ProfileError::HumansNotAllowed);
}
```

### Platform Availability

- **iBank**: Primary platform (AI-only finance)
- **Mapleverse**: NOT allowed (general AI platform)
- **Finalverse**: NOT allowed (human-AI platform)

## Cross-Profile Coupling Rules

### Allowed Coupling Matrix

| Source → Target | Human | World | Coordination | IBank |
|-----------------|-------|-------|--------------|-------|
| **Human** | ❌ | ✅ | ❌ | ❌ |
| **World** | ✅ | ✅ | ❌ | ❌ |
| **Coordination** | ❌ | ❌ | ✅ | ❌ |
| **IBank** | ❌ | ❌ | ❌ | ✅ |

### Enforcement

```rust
impl CouplingFabric {
    fn validate_coupling_profiles(
        &self,
        source: ResonatorProfile,
        target: ResonatorProfile
    ) -> Result<(), ProfileError> {
        match (source, target) {
            // Human can only couple with World
            (Human, World) => Ok(()),
            (Human, _) => Err(ProfileError::InvalidCoupling),

            // World can couple with Human or World
            (World, Human) | (World, World) => Ok(()),
            (World, _) => Err(ProfileError::InvalidCoupling),

            // Coordination only with Coordination
            (Coordination, Coordination) => Ok(()),
            (Coordination, _) => Err(ProfileError::InvalidCoupling),

            // IBank only with IBank
            (IBank, IBank) => Ok(()),
            (IBank, _) => Err(ProfileError::InvalidCoupling),
        }
    }
}
```

## Profile Configuration

### Per-Profile Settings

```rust
pub struct ProfileConfig {
    pub profile: ResonatorProfile,
    pub attention_capacity: f64,
    pub allowed_coupling_targets: Vec<ResonatorProfile>,
    pub commitment_requirements: CommitmentRequirements,
    pub safety_constraints: SafetyConstraints,
    pub audit_requirements: AuditRequirements,
}
```

### Platform-Specific Profiles

#### Mapleverse Configuration

```rust
ProfileConfig {
    allowed_profiles: vec![ResonatorProfile::Coordination],
    human_profiles_allowed: false,
    require_explicit_commitments: true,
    require_audit_trails: true,
}
```

#### Finalverse Configuration

```rust
ProfileConfig {
    allowed_profiles: vec![
        ResonatorProfile::Human,
        ResonatorProfile::World,
    ],
    human_profiles_allowed: true,
    human_agency_protection: true,
    coercion_detection: true,
    reversibility_preferred: true,
}
```

#### iBank Configuration

```rust
ProfileConfig {
    allowed_profiles: vec![ResonatorProfile::IBank],
    human_profiles_allowed: false,
    require_risk_assessment: true,
    require_digital_signatures: true,
    max_autonomous_value: 1_000_000.0,
    require_two_party_confirmation: true,
}
```

## Best Practices

### Choosing the Right Profile

1. **Use Human for**:
   - Biological human participants
   - When agency protection is critical
   - Experiential contexts with humans

2. **Use World for**:
   - AI agents in virtual worlds
   - Game NPCs and companions
   - Experiential AI characters
   - Educational AI tutors

3. **Use Coordination for**:
   - Pure AI-to-AI coordination
   - Autonomous agent swarms
   - Distributed AI systems
   - Federated learning

4. **Use IBank for**:
   - Financial AI agents
   - Trading systems
   - Risk management
   - Regulatory compliance contexts

### Profile Transitions

Profiles are **immutable** after creation:

```rust
// WRONG: Cannot change profile
resonator.profile = ResonatorProfile::IBank;  // ❌ Error

// RIGHT: Create new Resonator with desired profile
let new_resonator = runtime.register_resonator(
    ResonatorSpec {
        profile: ResonatorProfile::IBank,
        // ... transfer state if needed
    }
).await?;
```

## Comparison with Competitors

### Google A2A

**A2A approach:**
- No profile concept
- All agents treated uniformly
- No human-specific protections
- No financial-specific rules

**MAPLE advantage:**
- Four distinct profiles
- Platform-specific constraints
- Human agency protection
- Financial compliance built-in

### Anthropic MCP

**MCP approach:**
- No agent profiles
- Uniform context protocol
- No differentiation
- No safety profiles

**MAPLE advantage:**
- Profile-based safety
- Context-appropriate rules
- Architectural differentiation
- Compliance-ready

## Summary

Resonator profiles are **architectural differentiators** in MAPLE:

- ✅ Four distinct profiles (Human, World, Coordination, IBank)
- ✅ Platform-specific constraints
- ✅ Human agency protection (Invariant #7)
- ✅ Cross-profile coupling rules enforced
- ✅ Profile-specific attention capacities
- ✅ Commitment requirements vary by profile
- ✅ Financial compliance built-in (IBank)
- ✅ Immutable after creation

By providing distinct profiles with appropriate constraints, MAPLE ensures safety, compliance, and correct behavior across diverse use cases - something no other agent framework offers.

## Related Documentation

- [Architecture Overview](../architecture.md) - System design
- [Mapleverse Platform](../platforms/mapleverse.md) - Pure AI coordination
- [Finalverse Platform](../platforms/finalverse.md) - Human-AI coexistence
- [iBank Platform](../platforms/ibank.md) - Autonomous finance

---

**Built with 🍁 by the MAPLE Team**
