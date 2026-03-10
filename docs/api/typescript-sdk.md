# TypeScript SDK

MapleAI does not currently ship an official published TypeScript SDK from this repo.

## Current integration path

Use the PALM daemon REST API and keep a thin client in your application or control plane.

```typescript
const response = await fetch("http://127.0.0.1:8080/health");
const status = await response.json();

console.log(status);
```

For richer workflows, target the REST resources documented in [REST API](rest-api.md).

## Recommended scope today

- operator consoles
- internal workflow services
- generated clients against the PALM HTTP API
- browser or Node.js tooling that needs runtime visibility

## Status

TypeScript integration is currently HTTP-first. An official SDK package may come later, but it is not published from this repo today.
