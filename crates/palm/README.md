# PALM Layer (`crates/palm/*`)

PALM is the operations/control-plane layer.

Components:

- `types`: Shared domain types.
- `registry`: Agent spec/instance registry.
- `deployment`: Deployment orchestration.
- `health`: Health checks and resilience.
- `state`: Checkpoint/snapshot/state management.
- `control`: Unified control-plane orchestration.
- `policy`: Policy evaluation and gates.
- `shared-state`: Shared UI/API state contracts.
- `daemon`: Background API and scheduler service.
- `observability`: Metrics/tracing/audit queries.
- `cli`: Direct `palm` CLI.
