# Resonator CLI (`crates/resonator/cli`)

Command-line interface for MAPLE Resonator system management.

## Overview

The Resonator CLI provides human-friendly tools for managing commitments, consequences, memory, and conversations in the MAPLE Resonance Architecture.

## Installation

```bash
# Build and install
cargo install --path crates/resonator/cli

# Or run directly
cargo run -p resonator-cli -- <command>
```

## Commands

### Commitment Management

```bash
# List all commitments
resonator commitment list
resonator commitment list --status active
resonator commitment list --limit 10

# Inspect a specific commitment
resonator commitment inspect <id>

# Show lifecycle states
resonator commitment lifecycle

# Show valid transitions from a state
resonator commitment transitions active

# Validate a commitment file (dry-run)
resonator commitment validate ./my-commitment.json

# Show commitment statistics
resonator commitment stats
```

### Consequence Tracking

```bash
# List consequences
resonator consequence list
resonator consequence list --commitment-id <id>

# Show consequence details
resonator consequence show <id>

# Trace consequence chain back to commitment
resonator consequence trace <id>

# Show consequence statistics
resonator consequence stats
```

### Memory Management

```bash
# List memory entries
resonator memory list
resonator memory list --tier working
resonator memory list --tier long-term

# Show memory details
resonator memory show <id>

# Show memory tier information
resonator memory tiers

# Show memory statistics
resonator memory stats
```

### Conversation Management

```bash
# List active conversations
resonator conversation list

# Show conversation details
resonator conversation show <id>

# List turns in a conversation
resonator conversation turns <id>

# Show conversation statistics
resonator conversation stats
```

### System Information

```bash
# Show the 8 architectural invariants
resonator invariants

# Show the resonance pipeline stages
resonator pipeline
```

## Output Formats

All commands support multiple output formats:

```bash
# Human-readable table (default)
resonator commitment list

# JSON output
resonator commitment list --format json

# YAML output
resonator commitment list --format yaml
```

## Examples

### Inspect Commitment Lifecycle

```bash
$ resonator commitment lifecycle

Commitment Lifecycle States
============================================================

  Draft → Proposed → Accepted → Active → Executing → Completed

  Alternative endings:
    Failed (execution error)
    Disputed (conflict raised)
    Expired (time limit exceeded)
    Revoked (revoked by authority)

  Draft: Initial state, commitment being formed
  Proposed: Submitted for approval
  Accepted: Approved by authority
  Active: Ready for execution
  Executing: Currently being executed
  Completed: Successfully completed
  Failed: Execution failed
  Disputed: Under dispute resolution
  Expired: Time limit exceeded
  Revoked: Revoked by authority
```

### Show Architectural Invariants

```bash
$ resonator invariants

The 8 MAPLE Runtime Invariants
============================================

  1. Presence precedes Coupling
     A Resonator must establish presence before forming couplings

  2. Coupling precedes Meaning
     Meaning can only form within established coupling relationships

  3. Meaning precedes Intent
     Intent requires sufficient meaning convergence

  4. Commitment precedes Consequence
     No consequence without explicit, auditable commitment

  5. Receipts are Immutable
     Once created, commitment receipts cannot be modified

  6. Audit trail is Append-Only
     Audit entries can only be added, never removed or modified

  7. Capabilities gate Actions
     Actions require explicit capability grants

  8. Time anchors are Monotonic
     Temporal anchors always increase within a timeline
```

### Show Resonance Pipeline

```bash
$ resonator pipeline

Resonance Pipeline
============================================

  ┌──────────┐    ┌──────────┐    ┌──────────┐
  │ Presence │ ─→ │ Coupling │ ─→ │ Meaning  │
  └──────────┘    └──────────┘    └──────────┘
                                        │
  ┌──────────────┐    ┌────────────┐    ▼
  │ Consequence  │ ←─ │ Commitment │ ←─ ┌──────────┐
  └──────────────┘    └────────────┘    │  Intent  │
                                        └──────────┘

  Stage Descriptions:

    Presence:    Establish gradient presence (discoverability, stability)
    Coupling:    Form stateful relationships with other Resonators
    Meaning:     Build semantic understanding through coupling
    Intent:      Stabilize goals from sufficient meaning
    Commitment:  Create explicit, auditable promises
    Consequence: Execute and track outcomes from commitments
```

## Integration with Resonator Service

The CLI can operate in two modes:

1. **Standalone Mode** (default): Uses in-memory storage for demonstrations
2. **Connected Mode**: Connects to a running Resonator service for live data

```bash
# Connect to a running service
resonator --endpoint http://localhost:8080 commitment list
```

## Configuration

Create `~/.resonator/config.toml` for persistent settings:

```toml
[service]
endpoint = "http://localhost:8080"
timeout_ms = 5000

[output]
default_format = "table"
color = true
```

## Exit Codes

- `0`: Success
- `1`: General error
- `2`: Connection error
- `3`: Invalid argument
- `4`: Resource not found

## See Also

- [Resonator Architecture](../README.md)
- [Commitment System](../commitment/README.md)
- [Consequence Tracking](../consequence/README.md)
- [Memory System](../memory/README.md)
