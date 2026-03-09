# Guard and Policies

Guard is MAPLE's enforcement layer for capability firewalls, approval workflows, redaction, and compliance posture. The core rule is deny by default: an agent has no authority until a capability grant and policy rule say otherwise.

## Minimal policy

```yaml
version: v1
default: deny

rules:
  - id: allow-ticket-read
    match:
      capability: [zendesk.ticket.read]
    action: allow

  - id: require-approval-for-reply
    match:
      capability: [zendesk.ticket.reply]
      riskTier: [high]
    action: require_approval
```

## Policy building blocks

- Conditions: capability, profile, tenant, risk tier, data class, environment
- Actions: allow, deny, hold, require approval, redact, route to alternate backend
- Compliance overlays: reusable rule packs for specific obligations

## PII and secrets controls

Guard should sit in front of both tool use and model use. That means it can:

- block unsafe tool arguments
- redact obvious secrets before inference
- force a local model for regulated data
- record why a request was denied or held

## Approval workflows

Approval should be treated as part of the runtime path, not a side channel in Slack or email.

```yaml
action: require_approval
approvals:
  count: 2
  approvers: [finance-ops, security-duty]
```

## Compliance packs

The prompt pack positions MAPLE compliance as modular overlays. A practical deployment model is:

1. load a base Guard policy
2. layer a pack such as PCI or HIPAA
3. add tenant-specific exceptions explicitly
4. validate in a lower environment before promotion

## CLI examples

```bash
maple guard validate ./policies/finance.yaml
maple guard simulate --policy ./policies/finance.yaml --input ./fixtures/payment.json
maple guard apply --policy ./policies/finance.yaml
```

## Operator guidance

- Keep grants narrow and capability names specific.
- Separate read-only permissions from consequence-producing permissions.
- Make rejection receipts part of your normal operations review.
