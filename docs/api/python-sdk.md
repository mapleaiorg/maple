# Python SDK

MapleAI does not currently ship an official published Python SDK from this repo.

## Current integration path

Use the PALM daemon REST API from your automation or evaluation harness.

```python
import requests

response = requests.get("http://127.0.0.1:8080/health", timeout=10)
response.raise_for_status()

print(response.json())
```

For richer workflows, target the REST resources documented in [REST API](rest-api.md).

## Recommended scope today

- automation and operator scripts
- evaluation harnesses
- data workflows that need runtime health or audit data
- internal generated clients against the PALM HTTP API

## Status

Python integration is currently HTTP-first. An official SDK package may come later, but it is not published from this repo today.
