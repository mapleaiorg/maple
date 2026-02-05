# Commitments and Accountability

## Overview

In MAPLE, every consequential action requires an **explicit commitment** with a full audit trail. This architectural requirement ensures complete accountability, enables trust in multi-agent coordination, and provides the foundation for safe autonomous AI systems.

## Core Concept

A **commitment** is an explicit, digitally-signed promise made by a Resonator to take a specific action or maintain a specific state. Unlike traditional agent frameworks where actions are implicit or untracked, MAPLE makes **every consequential action attributable** through the commitment system.

```
Traditional Frameworks:    Action → Outcome (no record, no attribution)

MAPLE:                     Intent → Commitment → Action → Consequence
                                        ↓
                                  Audit Trail
```

## Why Commitments?

### 1. Full Accountability

Every action can be traced:
- Who made the commitment?
- When was it made?
- What was promised?
- Was it fulfilled?
- What was the outcome?

### 2. Trust Through Transparency

Commitments enable trust by:
- Making promises explicit
- Providing verifiable records
- Enabling audit and review
- Supporting dispute resolution

### 3. Safety Through Attribution

Attribution prevents:
- Unattributable harmful actions
- Denial of responsibility
- Hidden failures
- Shadow coordination

### 4. Regulatory Compliance

Audit trails support:
- Financial regulations (iBank)
- Safety certifications
- Legal requirements
- Industry standards

## Commitment Structure

```rust
pub struct Commitment {
    /// Unique identifier
    pub id: CommitmentId,

    /// Resonator making commitment
    pub resonator: ResonatorId,

    /// What is being committed to
    pub content: CommitmentContent,

    /// Current status
    pub status: CommitmentStatus,

    /// Complete audit trail
    pub audit_trail: Vec<AuditEntry>,

    /// Risk assessment (if required)
    pub risk_assessment: Option<RiskAssessment>,

    /// Can this be reversed?
    pub reversibility: bool,

    /// Couplings affected by this commitment
    pub affected_couplings: Vec<CouplingId>,

    /// Temporal anchors
    pub created_at: TemporalAnchor,
    pub updated_at: TemporalAnchor,
    pub expires_at: Option<TemporalAnchor>,

    /// Digital signature (non-repudiation)
    pub signature: DigitalSignature,
}
```

## Commitment Content Types

### Action Commitment

Promise to perform a specific action:

```rust
pub struct ActionCommitment {
    pub action: String,
    pub parameters: HashMap<String, Value>,
    pub preconditions: Vec<Condition>,
    pub postconditions: Vec<Condition>,
    pub deadline: Option<TemporalAnchor>,
}
```

**Example:**
```rust
ActionCommitment {
    action: "transfer_funds",
    parameters: {
        "from": "account_123",
        "to": "account_456",
        "amount": 1000.0,
        "currency": "USD"
    },
    preconditions: ["balance >= 1000"],
    postconditions: ["transfer_confirmed"],
    deadline: Some(now + 5_minutes),
}
```

### State Commitment

Promise to maintain a specific state:

```rust
pub struct StateCommitment {
    pub state: String,
    pub properties: HashMap<String, Value>,
    pub duration: Option<Duration>,
    pub monitoring_required: bool,
}
```

**Example:**
```rust
StateCommitment {
    state: "service_available",
    properties: {
        "uptime": ">= 99.9%",
        "response_time": "<= 100ms"
    },
    duration: Some(Duration::from_hours(24)),
    monitoring_required: true,
}
```

### Boundary Commitment

Promise not to exceed certain boundaries:

```rust
pub struct BoundaryCommitment {
    pub boundaries: Vec<Boundary>,
    pub enforcement: EnforcementLevel,
    pub violation_handling: ViolationPolicy,
}
```

**Example:**
```rust
BoundaryCommitment {
    boundaries: vec![
        Boundary::MaxCoupling(10),
        Boundary::MaxAttentionUsage(0.8),
        Boundary::NoHumanInteraction,
    ],
    enforcement: EnforcementLevel::Strict,
    violation_handling: ViolationPolicy::ImmediateRevoke,
}
```

### Result Commitment

Promise to achieve a specific result:

```rust
pub struct ResultCommitment {
    pub goal: String,
    pub success_criteria: Vec<Criterion>,
    pub fallback_plan: Option<String>,
    pub best_effort: bool,
}
```

**Example:**
```rust
ResultCommitment {
    goal: "optimize_portfolio_return",
    success_criteria: vec![
        Criterion::MinReturn(0.05),
        Criterion::MaxDrawdown(0.1),
    ],
    fallback_plan: Some("revert_to_conservative"),
    best_effort: false,
}
```

## Commitment Status

```rust
pub enum CommitmentStatus {
    /// Created but not yet active
    Pending,

    /// Currently active and being fulfilled
    Active,

    /// Successfully completed
    Fulfilled,

    /// Failed to fulfill (not violated)
    Failed,

    /// Deliberately violated
    Violated,

    /// Revoked by creator (before active)
    Revoked,

    /// Expired without fulfillment
    Expired,

    /// Under dispute
    Disputed,
}
```

### Status Transitions

```
Pending → Active → Fulfilled ✓
        ↓       → Failed
        ↓       → Violated
        ↓       → Expired
        ↓       → Disputed
        → Revoked
```

## Audit Trail

Every commitment maintains a complete audit trail:

```rust
pub struct AuditEntry {
    /// Unique entry ID
    pub id: AuditEntryId,

    /// Type of event
    pub event: AuditEvent,

    /// When it occurred
    pub timestamp: TemporalAnchor,

    /// Who caused it (if applicable)
    pub actor: Option<ResonatorId>,

    /// Event details
    pub details: HashMap<String, Value>,

    /// Digital signature
    pub signature: DigitalSignature,
}

pub enum AuditEvent {
    Created,
    Activated,
    ProgressUpdate,
    StatusChange,
    Fulfilled,
    Failed,
    Violated,
    Revoked,
    Disputed,
    Resolved,
}
```

### Audit Entry Example

```rust
AuditEntry {
    id: "audit_789",
    event: AuditEvent::StatusChange,
    timestamp: TemporalAnchor::now(),
    actor: Some(resonator_id),
    details: {
        "old_status": "Active",
        "new_status": "Fulfilled",
        "completion_time": "2026-01-15T10:30:00Z",
        "verification": "confirmed"
    },
    signature: signature,
}
```

## Risk Assessment

For consequential commitments (especially in iBank), risk assessment is mandatory:

```rust
pub struct RiskAssessment {
    /// Overall risk score (0.0-1.0)
    pub risk_score: f64,

    /// Risk breakdown by category
    pub risk_factors: Vec<RiskFactor>,

    /// Financial impact (if applicable)
    pub financial_impact: Option<FinancialImpact>,

    /// Risk mitigation strategies
    pub mitigations: Vec<Mitigation>,

    /// Who performed assessment
    pub assessed_by: ResonatorId,

    /// When assessment was done
    pub assessed_at: TemporalAnchor,
}

pub struct RiskFactor {
    pub category: RiskCategory,
    pub score: f64,
    pub description: String,
}

pub enum RiskCategory {
    Financial,
    Operational,
    Compliance,
    Reputation,
    Technical,
}
```

### Risk Assessment Example

```rust
RiskAssessment {
    risk_score: 0.35,  // Medium risk
    risk_factors: vec![
        RiskFactor {
            category: RiskCategory::Financial,
            score: 0.4,
            description: "Market volatility could impact outcome",
        },
        RiskFactor {
            category: RiskCategory::Operational,
            score: 0.3,
            description: "Depends on external API availability",
        },
    ],
    financial_impact: Some(FinancialImpact {
        potential_loss: 50000.0,
        potential_gain: 15000.0,
        currency: "USD",
    }),
    mitigations: vec![
        Mitigation {
            strategy: "Stop-loss at 2%",
            effectiveness: 0.9,
        },
    ],
    assessed_by: risk_assessor_id,
    assessed_at: TemporalAnchor::now(),
}
```

## Creating Commitments

### Basic Commitment Creation

```rust
use maple_runtime::{Commitment, CommitmentContent, ActionCommitment};

// Create commitment
let content = CommitmentContent::Action(ActionCommitment {
    action: "process_data".to_string(),
    parameters: HashMap::new(),
    preconditions: vec![],
    postconditions: vec![],
    deadline: None,
});

let commitment = resonator.create_commitment(content).await?;
println!("Commitment created: {}", commitment.id);
```

### Commitment with Risk Assessment

```rust
// For financial operations (iBank)
let content = CommitmentContent::Action(ActionCommitment {
    action: "execute_trade",
    parameters: {
        "symbol": "AAPL",
        "quantity": 100,
        "action": "BUY",
    },
    // ...
});

let risk = RiskAssessment {
    risk_score: 0.3,
    // ... risk details ...
};

let commitment = resonator.create_commitment_with_risk(
    content,
    risk
).await?;
```

### Commitment with Affected Couplings

```rust
// Specify couplings affected by this commitment
let commitment = resonator.create_commitment_ex(
    content,
    CommitmentOptions {
        affected_couplings: vec![coupling_id_1, coupling_id_2],
        reversibility: true,
        expires_at: Some(now + 1_hour),
        require_signature: true,
    }
).await?;
```

## Commitment Lifecycle

### 1. Creation (Pending)

```rust
let commitment = resonator.create_commitment(content).await?;
assert_eq!(commitment.status, CommitmentStatus::Pending);
```

### 2. Activation

```rust
commitment.activate().await?;
assert_eq!(commitment.status, CommitmentStatus::Active);
```

### 3. Progress Updates

```rust
commitment.update_progress(0.5, "Half complete").await?;

// Audit trail automatically updated
```

### 4. Fulfillment

```rust
commitment.fulfill(outcome_data).await?;
assert_eq!(commitment.status, CommitmentStatus::Fulfilled);

// Consequence created and linked
```

### 5. Failure Handling

```rust
match commitment.execute().await {
    Ok(outcome) => {
        commitment.fulfill(outcome).await?;
    }
    Err(e) => {
        commitment.fail(e.to_string()).await?;
        // Audit trail records failure
    }
}
```

## Architectural Invariants

### Invariant #3: Intent Precedes Commitment

```
✓ ALLOWED:  Stabilized Intent → Make Commitment
✗ FORBIDDEN: Make Commitment without Stabilized Intent
```

You cannot create a commitment without having:
- Sufficient meaning convergence (≥0.5)
- Stabilized intent
- Clear understanding of action

### Invariant #4: Commitment Precedes Consequence

```
✓ ALLOWED:  Explicit Commitment → Produce Consequence
✗ FORBIDDEN: Produce Consequence without Explicit Commitment
```

Every consequential action MUST:
- Have an explicit commitment
- Be recorded in audit trail
- Be attributable to a Resonator

### Invariant #8: Failure Must Be Explicit

```
✓ ALLOWED:  Surface All Failures
✗ FORBIDDEN: Silent Failures or Hidden Errors
```

Commitment failures:
- Must be recorded in audit trail
- Must update commitment status
- Must be surfaced to involved parties
- Cannot be hidden or ignored

## Commitment Validation

Before activation, commitments are validated:

```rust
pub struct CommitmentValidator {
    pub config: CommitmentConfig,
}

impl CommitmentValidator {
    pub fn validate(&self, commitment: &Commitment) -> Result<(), ValidationError> {
        // Check preconditions
        self.validate_preconditions(commitment)?;

        // Check risk assessment (if required)
        if self.config.require_risk_assessment {
            self.validate_risk(commitment)?;
        }

        // Check affected couplings still valid
        self.validate_couplings(commitment)?;

        // Check digital signature (if required)
        if self.config.require_digital_signature {
            self.validate_signature(commitment)?;
        }

        Ok(())
    }
}
```

## Platform-Specific Commitment Rules

### Mapleverse (Pure AI)

```rust
CommitmentConfig {
    require_audit_trail: true,
    require_digital_signature: true,
    require_risk_assessment: false,  // Not financial
    allow_best_effort: false,  // All commitments binding
    max_concurrent_commitments: 100,
}
```

**Characteristics:**
- All commitments binding (no best-effort)
- Digital signatures required
- Full audit trails
- High concurrency support

### Finalverse (Human-AI)

```rust
CommitmentConfig {
    require_audit_trail: true,
    require_digital_signature: false,  // Optional
    require_risk_assessment: false,
    allow_best_effort: true,  // Experiential context
    reversible_preferred: true,
    human_override_allowed: true,
}
```

**Characteristics:**
- Reversibility preferred
- Best-effort commitments allowed
- Human override capability
- Less strict for experiential interactions

### iBank (Finance)

```rust
CommitmentConfig {
    require_audit_trail: true,
    require_digital_signature: true,
    require_risk_assessment: true,  // Mandatory
    allow_best_effort: false,
    max_autonomous_value: 1_000_000.0,  // $1M limit
    require_two_party_confirmation: true,
}
```

**Characteristics:**
- Risk assessment mandatory
- Digital signatures required
- No best-effort (all binding)
- Dollar limits enforced
- Two-party confirmation for large transactions

## Commitment Queries

### Query by Status

```rust
let active_commitments = runtime.query_commitments(
    CommitmentQuery::ByStatus(CommitmentStatus::Active)
).await?;

println!("Active commitments: {}", active_commitments.len());
```

### Query by Resonator

```rust
let my_commitments = runtime.query_commitments(
    CommitmentQuery::ByResonator(resonator_id)
).await?;
```

### Query by Time Range

```rust
let recent = runtime.query_commitments(
    CommitmentQuery::ByTimeRange {
        start: yesterday,
        end: now,
    }
).await?;
```

### Query by Risk Score

```rust
let high_risk = runtime.query_commitments(
    CommitmentQuery::ByRiskScore {
        min: 0.7,
        max: 1.0,
    }
).await?;
```

## Dispute Resolution

When commitments are disputed:

```rust
// Mark as disputed
commitment.dispute(
    disputing_party,
    "Precondition was not met"
).await?;

// Adjudication process begins
let resolution = adjudicator.resolve_dispute(
    commitment.id
).await?;

match resolution {
    Resolution::UpholdCommitment => {
        commitment.resolve_dispute(DisputeOutcome::Upheld).await?;
    }
    Resolution::RevokeCommitment => {
        commitment.resolve_dispute(DisputeOutcome::Revoked).await?;
    }
    Resolution::ModifyCommitment(new_terms) => {
        commitment.modify(new_terms).await?;
        commitment.resolve_dispute(DisputeOutcome::Modified).await?;
    }
}
```

## Consequence Tracking

When commitments are fulfilled, consequences are created:

```rust
pub struct Consequence {
    pub id: ConsequenceId,
    pub commitment: CommitmentId,
    pub resonator: ResonatorId,
    pub outcome: Outcome,
    pub impact: Impact,
    pub timestamp: TemporalAnchor,
    pub reversible: bool,
}
```

**Linking commitments to consequences:**

```rust
// Fulfill commitment
commitment.fulfill(outcome).await?;

// Consequence automatically created
let consequence = runtime.get_consequence_for_commitment(
    commitment.id
).await?;

// Full traceability
println!("Commitment {} → Consequence {}",
    commitment.id, consequence.id);
```

## Audit and Compliance

### Generate Audit Report

```rust
let report = runtime.generate_audit_report(
    AuditReportRequest {
        resonator: Some(resonator_id),
        time_range: (start, end),
        include_fulfilled: true,
        include_failed: true,
        include_violated: true,
    }
).await?;

// Export to various formats
report.export_csv("audit_report.csv")?;
report.export_json("audit_report.json")?;
report.export_pdf("audit_report.pdf")?;
```

### Compliance Checking

```rust
let compliance = runtime.check_compliance(
    ComplianceCheck {
        standard: ComplianceStandard::SOC2,
        scope: ComplianceScope::AllCommitments,
        period: last_year,
    }
).await?;

if !compliance.compliant {
    println!("Violations: {:?}", compliance.violations);
}
```

## Best Practices

### For Resonator Developers

1. **Always create commitments for consequential actions**
   ```rust
   // WRONG: Direct action
   execute_financial_transaction().await?;

   // RIGHT: Commitment first
   let commitment = create_commitment(content).await?;
   commitment.activate().await?;
   execute_with_commitment(commitment).await?;
   ```

2. **Include comprehensive preconditions**
   ```rust
   ActionCommitment {
       action: "deploy_update",
       preconditions: vec![
           "all_tests_passing",
           "backup_completed",
           "maintenance_window_active",
       ],
       // ...
   }
   ```

3. **Handle failures explicitly**
   ```rust
   match commitment.execute().await {
       Ok(outcome) => commitment.fulfill(outcome).await?,
       Err(e) => {
           commitment.fail(e.to_string()).await?;
           // Don't hide failures
       }
   }
   ```

4. **Mark reversibility accurately**
   ```rust
   let commitment = Commitment {
       reversibility: true,  // Only if truly reversible
       // ...
   };
   ```

### For Platform Operators

1. **Monitor commitment metrics**: Track fulfillment rates, violation rates
2. **Review high-risk commitments**: Extra scrutiny for risk > 0.7
3. **Audit regularly**: Periodic compliance checks
4. **Investigate violations**: Understand and address root causes
5. **Tune risk thresholds**: Adjust based on observed outcomes

## Comparison with Competitors

### Google A2A

**A2A approach:**
- No commitment model
- Actions are untracked
- No audit trails
- No accountability mechanism

**MAPLE advantage:**
- Explicit commitments required
- Full audit trails
- Complete accountability
- Regulatory compliance support

### Anthropic MCP

**MCP approach:**
- No commitment concept
- Context injection only
- No attribution
- No audit capability

**MAPLE advantage:**
- Commitment-based architecture
- Every action attributable
- Built-in audit trails
- Non-repudiation through signatures

## Future Enhancements

### Planned Features

1. **Smart contract integration**: Deploy commitments as smart contracts
2. **Multi-party commitments**: Commitments requiring multiple Resonators
3. **Conditional commitments**: Activate based on conditions
4. **Commitment templates**: Reusable commitment patterns
5. **Automated dispute resolution**: ML-based adjudication

### Research Directions

1. **Formal verification**: Prove commitment properties
2. **Commitment languages**: DSL for expressing commitments
3. **Commitment markets**: Trade and transfer commitments
4. **Privacy-preserving commitments**: Zero-knowledge commitments

## Summary

Commitments are the **accountability backbone** of MAPLE:

- ✅ Every consequential action requires explicit commitment
- ✅ Full audit trails for transparency
- ✅ Risk assessment for financial operations
- ✅ Digital signatures for non-repudiation
- ✅ Architectural invariants enforced (#3, #4, #8)
- ✅ Platform-specific rules (Mapleverse, Finalverse, iBank)
- ✅ Dispute resolution support
- ✅ Regulatory compliance ready

By making commitments explicit and tracked, MAPLE enables trust and accountability at scale - something no other agent framework provides.

## Related Documentation

- [Architecture Overview](../architecture.md) - System design
- [Attention](attention.md) - Resource management
- [Temporal Anchors](temporal.md) - Causal time
- [iBank Platform](../platforms/ibank.md) - Financial commitments

---

**Built with 🍁 by the MAPLE Team**
