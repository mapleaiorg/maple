# MAPLE Product Specifications

This directory contains detailed specifications for MAPLE's flagship products.

## Products

| Product | Description | Primary Concern |
|---------|-------------|-----------------|
| [Mapleverse](mapleverse-spec.md) | Swarm orchestration platform | Throughput |
| [Finalverse](finalverse-spec.md) | Human-centric world simulation | Safety |
| [iBank](ibank-spec.md) | Autonomous financial operations | Accountability |

## Product Comparison

### Architectural Priorities
```
                    Throughput
                        ▲
                        │
                        │    ┌─────────────┐
                        │    │  Mapleverse │
                        │    └─────────────┘
                        │
                        │
                        │         ┌─────────────┐
                        │         │  Finalverse │
                        │         └─────────────┘
                        │
                        │
         ◄──────────────┼──────────────────────►
       Safety           │              Accountability
                        │
                        │                    ┌─────────────┐
                        │                    │    iBank    │
                        │                    └─────────────┘
                        │
                        ▼
```

### Feature Matrix

| Feature | Mapleverse | Finalverse | iBank |
|---------|------------|------------|-------|
| Max Scale | 10M agents | 5K agents | 100 agents |
| Human Approval | Not required | Required | Required (>$100k) |
| Accountability Proof | No | No | Always |
| Force Operations | Allowed | With approval | Never |
| Checkpoint Frequency | 10 min | 2 min | 1 min |
| Data Retention | 1 day | 30 days | 1 year |
| Audit Trail | Basic | Standard | Full chain |

### Use Case Alignment

| Use Case | Recommended Product |
|----------|---------------------|
| Game NPC coordination | Mapleverse |
| Swarm robotics | Mapleverse |
| Distributed computing | Mapleverse |
| Virtual worlds | Finalverse |
| Educational simulations | Finalverse |
| Social AI experiences | Finalverse |
| Treasury management | iBank |
| Algorithmic trading | iBank |
| Compliance automation | iBank |

## Getting Started

1. **Choose your product** based on your primary concern
2. **Review the specification** for detailed requirements
3. **Use the corresponding platform pack**:
   - `mapleverse-pack` for Mapleverse
   - `finalverse-pack` for Finalverse
   - `ibank-pack` for iBank

## Architecture Integration

All products are built on the shared MAPLE/PALM infrastructure:
```
┌─────────────────────────────────────────────────────────────────────┐
│                         Product Layer                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │
│  │  Mapleverse │  │  Finalverse │  │    iBank    │                  │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘                  │
│         │                │                │                         │
├─────────┼────────────────┼────────────────┼─────────────────────────┤
│         │                │                │                         │
│  ┌──────▼──────┐  ┌──────▼──────┐  ┌──────▼──────┐                  │
│  │  Mapleverse │  │  Finalverse │  │    iBank    │                  │
│  │    Pack     │  │    Pack     │  │    Pack     │                  │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘                  │
│         │                │                │                         │
│         └────────────────┼────────────────┘                         │
│                          │                                          │
│                    ┌─────▼─────┐                                    │
│                    │ Platform  │                                    │
│                    │ Contract  │                                    │
│                    └─────┬─────┘                                    │
│                          │                                          │
├──────────────────────────┼──────────────────────────────────────────┤
│                          │                                          │
│                    ┌─────▼─────┐                                    │
│                    │   PALM    │                                    │
│                    │  Runtime  │                                    │
│                    └───────────┘                                    │
│                                                                     │
│                  Shared MAPLE Infrastructure                        │
└─────────────────────────────────────────────────────────────────────┘
```

## Platform Pack Selection

### Decision Tree

```
Start
  │
  ▼
┌─────────────────────────────────────┐
│ Is individual agent accountability  │
│ critical (financial, legal)?        │
└──────────────┬──────────────────────┘
               │
      ┌────────┴────────┐
      │ Yes             │ No
      ▼                 ▼
   iBank        ┌──────────────────────────┐
                │ Are humans directly       │
                │ interacting with agents?  │
                └──────────────┬────────────┘
                               │
                      ┌────────┴────────┐
                      │ Yes             │ No
                      ▼                 ▼
                  Finalverse       Mapleverse
```

### Detailed Comparison

#### Throughput vs Safety Trade-off

| Scenario | Mapleverse | Finalverse | iBank |
|----------|------------|------------|-------|
| Agent crash | Auto-restart immediately | Safe shutdown, review | Full audit, manual restart |
| Scale up 10x | Automatic | Requires review | Multi-approval required |
| Network partition | Best-effort recovery | Graceful degradation | Halt operations |
| Inconsistent state | Self-heal | Human intervention | Full reconciliation |

#### Policy Enforcement

| Policy | Mapleverse | Finalverse | iBank |
|--------|------------|------------|-------|
| DELETE operations | Allowed | Human approval | Never (archive only) |
| SCALE operations | Automatic | Rate-limited | Dual approval |
| FORCE_RECOVERY | Allowed | Human approval | Never |
| Hot reload | Enabled | Disabled | Disabled |

## Migration Between Products

### Mapleverse to Finalverse

When throughput requirements decrease and safety becomes more important:

1. Reduce agent count to within Finalverse limits
2. Enable human approval requirements
3. Increase checkpoint frequency
4. Extend data retention
5. Switch platform pack

### Finalverse to iBank

When moving to financial operations requiring full accountability:

1. Enable commitment ledger
2. Implement accountability proofs
3. Set up compliance monitoring
4. Configure audit chain
5. Disable force operations
6. Switch platform pack

## Resource Requirements

### Minimum Deployment

| Resource | Mapleverse | Finalverse | iBank |
|----------|------------|------------|-------|
| CPU Cores | 16 | 8 | 8 |
| Memory | 64 GB | 32 GB | 32 GB |
| Storage | 500 GB SSD | 200 GB SSD | 1 TB SSD (encrypted) |
| Network | 10 Gbps | 1 Gbps | 1 Gbps (isolated) |

### Production Deployment

| Resource | Mapleverse | Finalverse | iBank |
|----------|------------|------------|-------|
| CPU Cores | 128+ | 32+ | 32+ |
| Memory | 512 GB+ | 128 GB+ | 128 GB+ |
| Storage | 10 TB+ | 2 TB+ | 10 TB+ (encrypted, replicated) |
| Network | 100 Gbps | 10 Gbps | 10 Gbps (isolated, redundant) |

## Support & SLA

| Tier | Mapleverse | Finalverse | iBank |
|------|------------|------------|-------|
| Response Time | 4 hours | 1 hour | 15 minutes |
| Uptime SLA | 99.9% | 99.95% | 99.99% |
| Support Hours | Business hours | Extended hours | 24/7 |
| Incident Review | Optional | Required | Mandatory |

## Further Reading

- [Platform Pack Contract](../api/README.md)
- [Conformance Testing](../conformance.md)
- [Architecture Overview](../architecture.md)
- [Getting Started Tutorial](../tutorials/platform-packs.md)
