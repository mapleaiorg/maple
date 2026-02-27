# Operations Tutorial

This tutorial covers daemon startup, CLI operations, AgentKernel boundary execution, and runtime troubleshooting.

## 1. CLI Entry Points

Primary CLI:

```bash
cargo run -p maple-cli -- --help
```

Optional compatibility CLI:

```bash
cargo run -p palm -- --help
```

## 2. Start Daemon

```bash
# default config
cargo run -p palm-daemon

# explicit in-memory storage
PALM_STORAGE_TYPE=memory cargo run -p palm-daemon
```

Default API endpoint: `http://127.0.0.1:8080`

## 3. Daemon Lifecycle

```bash
cargo run -p maple-cli -- daemon status
cargo run -p maple-cli -- daemon stop
```

## 4. Local Doctor Checks

```bash
cargo run -p maple-cli -- doctor
cargo run -p maple-cli -- doctor --model llama3.2
```

## 5. AgentKernel Boundary from CLI

```bash
# safe path
cargo run -p maple-cli -- agent demo --prompt "log runtime status"

# dangerous path denied without commitment
cargo run -p maple-cli -- agent demo --dangerous --prompt "transfer 500 usd to demo"

# dangerous path with explicit commitment
cargo run -p maple-cli -- agent demo --dangerous --with-commitment --amount 500 --prompt "transfer 500 usd to demo"
```

Inspect persisted boundary state:

```bash
cargo run -p maple-cli -- agent audit --limit 20
cargo run -p maple-cli -- agent commitments --limit 20
cargo run -p maple-cli -- agent contract --id <commitment_id>
```

## 6. WorldLine Command Groups

```bash
cargo run -p maple-cli -- worldline list
cargo run -p maple-cli -- commit submit --file /tmp/commitment.json
cargo run -p maple-cli -- provenance worldline-history <worldline_id>
cargo run -p maple-cli -- financial projection <worldline_id> USD
cargo run -p maple-cli -- gov list
```

## 7. Real-Time Monitoring

```bash
cargo run -p maple-cli -- events watch
cargo run -p maple-cli -- playground activities --limit 50
cargo run -p maple-cli -- health summary
```

## 8. PostgreSQL Setup (Optional Durable Storage)

```bash
docker run --name maple-postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=maple \
  -p 5432:5432 \
  -v maple_pgdata:/var/lib/postgresql/data \
  -d postgres:16

docker exec maple-postgres pg_isready -U postgres -d maple

PALM_STORAGE_TYPE=postgres \
PALM_STORAGE_URL=postgres://postgres:postgres@localhost:5432/maple \
cargo run -p palm-daemon -- --platform mapleverse
```

## 9. Runtime Independence Checks

Validate standalone runtime mode in CI/ops:

```bash
cargo check -p maple-runtime --no-default-features
cargo test -p maple-runtime --no-default-features --lib
```

## 10. Next

- [Maple Runtime Standalone Tutorial](maple-runtime-standalone.md)
- [iBank Commitment Boundary Tutorial](ibank-commitment-boundary.md)
- [WorldLine Quickstart](worldline-quickstart.md)
