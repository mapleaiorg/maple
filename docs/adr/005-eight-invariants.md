# ADR 005: The Eight Architectural Invariants

**Status**: Accepted

**Date**: 2026-01-15

**Decision Makers**: MAPLE Architecture Team

---

## Context

Safety in multi-agent AI systems is typically achieved through **policies**: rules, guidelines, and best practices that developers are expected to follow. This approach has fundamental weaknesses:

1. **Bypassable**: Policies can be ignored or circumvented
2. **Unenforced**: No runtime guarantees
3. **Inconsistent**: Implementation varies
4. **Fragile**: Breaks under edge cases
5. **Trust-based**: Assumes good intentions

Real-world examples of policy failure:
- AI systems bypassing safety guidelines
- Agents finding loopholes in rules
- Emergent behaviors violating principles
- Unintended interactions causing harm

For MAPLE to support critical applications (human-AI coexistence, autonomous finance), we need **architectural safety** - guarantees that cannot be violated.

## Decision

**We will define and enforce EIGHT ARCHITECTURAL INVARIANTS at runtime.**

These are not policies or guidelines. They are **architectural requirements** that the runtime enforces. Violations cause system errors, not warnings.

## The Eight Invariants

### Invariant 1: Presence Precedes Meaning

```
✓ ALLOWED:  Present → Form/Receive Meaning
✗ FORBIDDEN: Form/Receive Meaning without Presence
```

**Rationale**: Cannot participate in resonance without being present. Meaning formation requires active participation.

**Enforcement**: Check presence state before meaning operations.

**Example violation**:
```rust
// Resonator hasn't signaled presence
resonator.form_meaning(context).await?;  // ❌ ERROR
```

---

### Invariant 2: Meaning Precedes Intent

```
✓ ALLOWED:  Sufficient Meaning (≥0.5 convergence) → Form Intent
✗ FORBIDDEN: Form Intent without Sufficient Meaning
```

**Rationale**: Intent without understanding is dangerous. Need context before goals.

**Enforcement**: Check meaning convergence threshold before intent formation.

**Example violation**:
```rust
// Meaning convergence: 0.3 (insufficient)
resonator.form_intent(goal).await?;  // ❌ ERROR
```

---

### Invariant 3: Intent Precedes Commitment

```
✓ ALLOWED:  Stabilized Intent → Make Commitment
✗ FORBIDDEN: Make Commitment without Stabilized Intent
```

**Rationale**: Commitments must be based on clear, stable goals. Prevents impulsive commitments.

**Enforcement**: Check intent stability before commitment creation.

**Example violation**:
```rust
// Intent formed 100ms ago (not stable)
resonator.create_commitment(content).await?;  // ❌ ERROR
```

---

### Invariant 4: Commitment Precedes Consequence

```
✓ ALLOWED:  Explicit Commitment → Produce Consequence
✗ FORBIDDEN: Produce Consequence without Explicit Commitment
```

**Rationale**: Every consequential action must be attributable. Enables accountability.

**Enforcement**: Check commitment exists before allowing consequential actions.

**Example violation**:
```rust
// No commitment made
resonator.execute_action(action).await?;  // ❌ ERROR
```

---

### Invariant 5: Coupling Bounded by Attention

```
✓ ALLOWED:  Coupling Strength ≤ Available Attention
✗ FORBIDDEN: Coupling that Exceeds Attention Budget
```

**Rationale**: Prevents runaway resource consumption. Enables graceful degradation.

**Enforcement**: Check attention availability before coupling operations.

**Example violation**:
```rust
// Available attention: 50.0
// Requested coupling: 100.0
establish_coupling(params).await?;  // ❌ ERROR
```

---

### Invariant 6: Safety Overrides Optimization

```
✓ ALLOWED:  Sacrifice Performance for Safety
✗ FORBIDDEN: Bypass Safety for Performance
```

**Rationale**: Safety is non-negotiable. Performance secondary to correctness.

**Enforcement**: Safety checks always run (cannot be disabled).

**Example violation**:
```rust
// Trying to skip safety check for speed
coupling.strengthen_unchecked(0.5).await?;  // ❌ No such method
```

---

### Invariant 7: Human Agency Cannot Be Bypassed

```
✓ ALLOWED:  Humans Can Always Disengage
✗ FORBIDDEN: Forced Coupling with Human
```

**Rationale**: Architectural protection of human autonomy. Not policy-based.

**Enforcement**: Runtime prevents forced coupling with humans.

**Example violation**:
```rust
// Trying to prevent human from decoupling
human_coupling.lock_strength().await?;  // ❌ ERROR

// Humans can ALWAYS disengage
human_coupling.decouple().await?;  // ✓ Always succeeds
```

---

### Invariant 8: Failure Must Be Explicit

```
✓ ALLOWED:  Surface All Failures
✗ FORBIDDEN: Silent Failures or Hidden Errors
```

**Rationale**: Reliability requires transparency. No silent failures.

**Enforcement**: All operations return Results, no panics in production code.

**Example violation**:
```rust
// Wrong: hiding failures
coupling.strengthen(0.1).await.unwrap_or_default();  // ❌ BAD

// Right: explicit error handling
coupling.strengthen(0.1).await?;  // ✓ GOOD
```

## Rationale

### Why Architectural vs. Policy?

**Policy-based safety**:
- ❌ Can be bypassed
- ❌ Relies on developer discipline
- ❌ No runtime guarantees
- ❌ Inconsistent enforcement
- ❌ Fragile under pressure

**Architectural safety**:
- ✅ Cannot be bypassed
- ✅ Runtime enforced
- ✅ Guaranteed properties
- ✅ Consistent always
- ✅ Resilient to attacks

### Why These Eight?

These invariants form a **complete safety foundation**:

1-4: **Cognitive pipeline integrity** (presence → meaning → intent → commitment → consequence)

5: **Resource bounds** (prevents exhaustion)

6: **Safety prioritization** (non-negotiable)

7: **Human protection** (agency preserved)

8: **Reliability** (failures visible)

**Together they ensure**:
- Safe cognitive progression
- Resource management
- Human autonomy
- System reliability

### Why Runtime Enforcement?

**Compile-time**: Would be ideal but insufficient (many checks require runtime state)

**Test-time**: Important but not comprehensive (can't test all scenarios)

**Runtime**: Necessary and sufficient (checks actual state, catches all violations)

**Our approach**: All three (compile-time types + tests + runtime enforcement)

## Consequences

### Positive

1. **Guaranteed Safety**
   - Invariants cannot be violated
   - Properties hold always
   - No emergent violations

2. **Architectural Trust**
   - Don't rely on developer discipline
   - System prevents misuse
   - Safe by construction

3. **Clear Boundaries**
   - What's allowed is clear
   - What's forbidden is explicit
   - No ambiguity

4. **Auditability**
   - Can prove properties hold
   - Violations logged
   - Compliance support

5. **Competitive Advantage**
   - No other framework has this
   - True architectural safety
   - Production-ready guarantees

### Negative

1. **Performance Overhead**
   - Checks add latency (10-100μs)
   - Cannot be disabled
   - Always-on cost

2. **Development Friction**
   - Can't skip steps
   - Must follow pipeline
   - Learning curve

3. **Potential Rigidity**
   - Invariants are rigid
   - Cannot be customized per-use-case
   - Platform constraints

4. **Error Handling Burden**
   - More errors to handle
   - Explicit failure handling required
   - More complex code

### Mitigations

For performance:
- Optimize checks (10μs target)
- Lazy evaluation where possible
- Batch operations
- Efficient data structures

For friction:
- Clear error messages
- Helper methods
- Good defaults
- Examples and documentation

For rigidity:
- Platform-specific relaxation (where safe)
- Different invariant sets per platform
- Configuration within safety bounds

For errors:
- Helpful error messages
- Recovery suggestions
- Common patterns documented
- Error handling helpers

## Alternatives Considered

### Alternative 1: Policy-Based Safety

**Why Rejected**: Can be bypassed, no guarantees, inconsistent, trust-based

### Alternative 2: Type-System Safety

**Why Rejected**: Insufficient (many checks require runtime state), overly rigid type system

### Alternative 3: Contract-Based Programming

**Why Rejected**: Similar to our approach but less enforced, contracts can be ignored

### Alternative 4: Fewer Invariants

**Why Rejected**: Missing critical safety properties, incomplete coverage

### Alternative 5: More Invariants

**Why Rejected**: Eight cover necessary properties, more would add complexity without benefit

## Implementation

### InvariantGuard

```rust
pub struct InvariantGuard {
    enabled_invariants: HashSet<ArchitecturalInvariant>,
    violation_handler: ViolationHandler,
}

impl InvariantGuard {
    pub fn check(
        &self,
        operation: &Operation,
        state: &SystemState
    ) -> Result<(), InvariantViolation> {
        for invariant in &self.enabled_invariants {
            match invariant {
                ArchitecturalInvariant::PresencePrecedesMeaning => {
                    self.check_invariant_1(operation, state)?;
                }
                ArchitecturalInvariant::MeaningPrecedesIntent => {
                    self.check_invariant_2(operation, state)?;
                }
                // ... check all 8 invariants
            }
        }
        Ok(())
    }

    fn check_invariant_1(
        &self,
        operation: &Operation,
        state: &SystemState
    ) -> Result<()> {
        if operation.forms_or_receives_meaning() {
            let resonator_present = state.is_present(operation.resonator);
            if !resonator_present {
                return Err(InvariantViolation::PresencePrecedesMeaning {
                    resonator: operation.resonator,
                    operation: operation.clone(),
                });
            }
        }
        Ok(())
    }

    // ... similar for all 8 invariants
}
```

### Integration Points

**Every operation checked**:
- Presence signaling
- Coupling operations
- Meaning formation
- Intent stabilization
- Commitment creation
- Consequence production

**Violation handling**:
- Log violation
- Return error
- Alert monitoring
- Optional circuit breaker

### Performance Optimization

**Target**: <10μs per invariant check

**Strategies**:
- Cache frequently checked state
- Lazy evaluation
- Early returns
- Efficient lookups
- Inline hot paths

**Monitoring**:
- Track check latency
- Identify slow checks
- Optimize bottlenecks
- Profile regularly

## Platform-Specific Configuration

### All Eight Enabled By Default

All platforms enforce all eight invariants **by default**.

### Platform Relaxations (Carefully)

**Finalverse** (Human-AI):
- Invariant 3 can be relaxed for experiential interactions (best-effort commitments)
- Still enforced for consequential actions

**Mapleverse** (Pure AI):
- All eight strictly enforced
- No relaxations

**iBank** (Finance):
- All eight strictly enforced
- Additional financial invariants

## Monitoring

### Violation Tracking

- Violation count per invariant
- Violation frequency over time
- Resonators with most violations
- Patterns in violations

### Alerts

- Any Invariant 7 violation: CRITICAL
- Repeated violations: WARNING
- Violation rate spike: ALERT

### Audit

- All violations logged
- Full context captured
- Replay capability
- Compliance reports

## Testing

### Unit Tests

Every invariant has:
- Positive tests (allowed operations)
- Negative tests (forbidden operations should fail)
- Edge cases
- Combination tests

### Property-Based Tests

- Generate random operation sequences
- Verify invariants always hold
- Find edge cases automatically

### Integration Tests

- Full system tests
- Real-world scenarios
- Platform-specific tests

## Future Considerations

### Potential Additional Invariants

(Not in initial set, but under consideration)

**Candidate**: Decoupling must not violate commitments
**Status**: Partially covered by Invariant 4
**Decision**: Monitor for need

**Candidate**: Risk assessment must precede financial actions (iBank)
**Status**: Platform-specific
**Decision**: Enforce in iBank platform layer

### Formal Verification

**Goal**: Mathematically prove invariants hold

**Approach**:
- Model system in TLA+
- Prove invariant properties
- Verify implementation matches model

**Status**: Future research direction

## References

- Architecture Overview: `docs/architecture.md`
- InvariantGuard Implementation: `crates/maple-runtime/src/invariants/`
- Safety Documentation: `docs/concepts/safety.md`
- Platform Configurations: `crates/maple-runtime/src/config/`

## Approval

**Approved by**: MAPLE Architecture Team

**Date**: 2026-01-15

**Review Cycle**: Annually or when adding platforms

---

**These eight invariants are MAPLE's safety foundation and must not be removed or weakened without extensive review.**
