# Profiles

Profiles let MAPLE keep one runtime model while changing the operating constraints for different environments. A profile shapes which couplings are allowed, which approvals are required, and which safety guarantees dominate.

## Common profile set

| Profile | Best fit | Primary concern |
| --- | --- | --- |
| Human | Human participants in mixed environments | Agency protection |
| Agent | General service agents | Controlled tool authority |
| Financial | Payment and trading workflows | Audit and bounded consequence |
| World | Experiential or simulation contexts | Reversibility and interaction safety |
| Coordination | Pure AI-to-AI coordination | Throughput with explicit bounds |

## Selection guidance

- Choose `Human` when disengagement and consent are architectural constraints.
- Choose `Agent` for general-purpose service automation.
- Choose `Financial` when approvals, idempotency, and evidence are mandatory.
- Choose `World` when human-facing experience and reversibility matter.
- Choose `Coordination` for high-throughput agent meshes with no direct human coupling.

## Cross-profile thinking

Profiles are not just labels. They determine what a safe coupling means, what the budget for consequence is, and which Guard rules should apply before execution.

## Related reading

- [/docs/reference/invariants](https://mapleai.org/docs/reference/invariants)
- [/docs/architecture/worldline-model](https://mapleai.org/docs/architecture/worldline-model)
