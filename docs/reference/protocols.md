# Protocols

The prompt pack frames MAPLE as a protocol suite, not just a runtime binary. These protocol names are useful shorthand for how the platform moves intent, authorization, and evidence through the system.

## Protocol map

| Protocol | Purpose | Mental model |
| --- | --- | --- |
| MRP | Maple Resonance Protocol | Signal and cognition flow |
| CEP | Commitment Exchange Protocol | Typed action declaration and approval |
| PVP | Provenance Verification Protocol | Receipt and lineage verification |
| GCP | Governance Coordination Protocol | Policy and approval coordination |
| WLP | WorldLine Presence Protocol | Identity presence and liveness |

## State sketches

### MRP

`present -> coupled -> meaning -> intent`

### CEP

`proposed -> evaluated -> authorized | denied | held -> executed`

### PVP

`receipt -> proof lookup -> lineage verification -> replay`

### GCP

`policy load -> simulate -> enforce -> review`

### WLP

`registered -> active -> observed -> suspended`

These names are most useful when you need to explain that MAPLE is not treating "tool call" as a single opaque operation. It breaks execution into explicit protocols with enforcement points.
