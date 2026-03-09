# Python SDK

Install:

```bash
pip install maple-sdk
```

The Python SDK is a good fit for automation, data workflows, and evaluation harnesses that need governed action submission without rebuilding the operator plane.

## Example

```python
from maple_sdk import MapleClient, Profile


async def main():
    client = await MapleClient.connect("http://localhost:8080")

    agent = await client.worldline.create(
        profile=Profile.Agent,
        label="my-support-agent",
    )

    result = await client.commit.submit(
        worldline_id=agent.id,
        obligation="resolve customer ticket #1234",
        capabilities=["zendesk.ticket.reply"],
    )

    print(result)
```

## Typical flow

- Create async clients for operator or batch workloads.
- Use provenance queries to validate expected outcomes after automation runs.
- Keep human approval loops separate from background execution loops.
