# TypeScript SDK

Install:

```bash
npm install @maple-ai/sdk
```

The TypeScript SDK is aimed at control planes, operator consoles, and workflow services that need typed access to the commitment model.

## Example

```typescript
import { MapleClient, Profile } from "@maple-ai/sdk";

const client = await MapleClient.connect("http://localhost:8080");

const agent = await client.worldline.create({
  profile: Profile.Agent,
  label: "my-support-agent",
});

const result = await client.commit.submit({
  worldlineId: agent.id,
  obligation: "resolve customer ticket #1234",
  capabilities: ["zendesk.ticket.reply"],
});

console.log(result);
```

## Typical flow

- Connect once and reuse the client.
- Keep worldline IDs explicit in your workflow state.
- Treat commitment responses as governed outcomes, not just boolean success.
