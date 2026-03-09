# REST API

The PALM daemon exposes MAPLE runtime APIs under `/api/v1`. The current public surface is organized around worldlines, commitments, governance, financial actions, kernel status, and the operator playground.

## Authentication and transport

- Use API keys or JWTs at the gateway layer when MAPLE is deployed behind a shared control surface.
- Treat `/api/v1` as the stable resource prefix.
- Use streaming endpoints for activity and event visibility where available.

## Core resources

### Worldlines

| Method | Path | Purpose |
| --- | --- | --- |
| `POST` | `/worldlines` | Create a worldline |
| `GET` | `/worldlines` | List worldlines |
| `GET` | `/worldlines/:id` | Inspect a worldline |

Create example:

```json
POST /api/v1/worldlines
{
  "profile": "financial",
  "label": "treasury-a"
}
```

### Commitments

| Method | Path | Purpose |
| --- | --- | --- |
| `POST` | `/commitments` | Submit a commitment declaration |
| `GET` | `/commitments/:id` | Fetch current commitment status |
| `GET` | `/commitments/:id/audit-trail` | Inspect gate-stage history |

Example response:

```json
{
  "commitment_id": "cmt_123",
  "decision_receipt_id": "dr_456",
  "status": "authorized"
}
```

### Governance

| Method | Path | Purpose |
| --- | --- | --- |
| `POST` | `/governance/policies` | Add a policy |
| `GET` | `/governance/policies` | List policies |
| `POST` | `/governance/simulate` | Simulate a policy decision |

Alias paths also exist under `/worldline-governance/*` for compatibility.

### Financial actions

| Method | Path | Purpose |
| --- | --- | --- |
| `POST` | `/financial/settle` | Submit settlement legs |
| `GET` | `/financial/:worldline_id/balance/:asset` | Fetch a projected balance |

Financial writes require both a valid `commitment_id` and a `decision_receipt_id`.

### Kernel and provenance

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/kernel/status` | Runtime status |
| `GET` | `/kernel/metrics` | Runtime metrics |
| `GET` | `/provenance/:event_id/ancestors` | Traverse ancestry |
| `GET` | `/provenance/worldline/:id/history` | Fetch worldline history |

## Error shape

Use structured errors so operators can tell whether a failure was:

- validation
- policy denial
- approval hold
- missing capability
- backend or infrastructure failure

```json
{
  "code": "policy_denied",
  "message": "capability zendesk.ticket.reply is not granted",
  "receipt_id": "dr_456"
}
```

## Streaming and operator surfaces

The current API docs also describe activity feeds under the playground namespace, including server-sent events for live updates. That is useful for operator consoles, dashboards, and audit-tail style monitoring.
