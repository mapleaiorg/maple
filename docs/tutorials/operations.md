# Operations Tutorial

This tutorial focuses on the current operational surfaces: daemon lifecycle, CLI diagnostics, worldline flows, and runtime inspection.

## 1. Start the daemon

```bash
cargo run -p palm-daemon
```

Optional in-memory storage:

```bash
PALM_STORAGE_TYPE=memory cargo run -p palm-daemon
```

## 2. Verify daemon health

```bash
cargo run -p maple-cli -- daemon status
cargo run -p maple-cli -- doctor
```

If you have a local model runtime available, you can also run:

```bash
cargo run -p maple-cli -- doctor --model llama3.2
```

## 3. Exercise worldline operations

```bash
cargo run -p maple-cli -- worldline list
cargo run -p maple-cli -- kernel status
cargo run -p maple-cli -- gov list
```

## 4. Exercise AgentKernel demo paths

```bash
# Safe path
cargo run -p maple-cli -- agent demo --prompt "log runtime status"

# Dangerous path denied without commitment
cargo run -p maple-cli -- agent demo --dangerous --prompt "transfer 500 usd to demo"

# Dangerous path with explicit commitment
cargo run -p maple-cli -- agent demo --dangerous --with-commitment --amount 500 --prompt "transfer 500 usd to demo"
```

## 5. Inspect audit data

```bash
cargo run -p maple-cli -- agent audit --limit 20
cargo run -p maple-cli -- agent commitments --limit 20
cargo run -p maple-cli -- provenance ancestors EVENT_ID --depth 5
```

## 6. Optional durable storage

```bash
docker run --name maple-postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=maple \
  -p 5432:5432 \
  -d postgres:16

PALM_STORAGE_TYPE=postgres \
PALM_STORAGE_URL=postgres://postgres:postgres@localhost:5432/maple \
cargo run -p palm-daemon
```

## Next

- [WorldLine Quickstart](worldline-quickstart.md)
- [CLI Reference](../api/cli-reference.md)
- [Fleet Deployment Guide](../guides/fleet-deployment.md)
