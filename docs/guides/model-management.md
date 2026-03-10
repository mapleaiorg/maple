# Model Management

MAPLE treats models like governed runtime dependencies. The user-facing mental model is intentionally close to Ollama, but the current implementation is broader and more library-first: local models, hosted APIs, routing policy, and cost-aware enforcement sit behind the same control surface.

## What is implemented now

- `maple-model-core` for model metadata, `MapleModelfile` parsing, and local on-disk storage
- `maple-model-router` for policy, fallback, and circuit breaking
- `maple-model-server` for OpenAI-compatible request and response types
- `maple-model-benchmark` for suites and quality gates
- PALM playground for backend selection and one-shot inference

## Current local workflow with Ollama

Today the practical "Ollama-like" MAPLE workflow is Ollama plus PALM.

```bash
ollama serve
ollama pull llama3.2:3b

cargo run -p maple-cli -- doctor --model llama3.2:3b
cargo run -p maple-cli -- daemon start --foreground
cargo run -p palm -- playground backends
cargo run -p palm -- playground set-backend \
  --kind local_llama \
  --model llama3.2:3b \
  --endpoint http://127.0.0.1:11434
cargo run -p palm -- playground infer "Summarize current runtime status"
```

That is the current operator instruction. Ollama provides the local backend. PALM records and controls which backend is active. The model crates provide the storage, routing, serving, and benchmark building blocks underneath.

## Current cloud workflow

PALM playground also supports hosted backends:

```bash
cargo run -p palm -- playground set-backend \
  --kind open_ai \
  --model gpt-4o-mini \
  --api-key "$OPENAI_API_KEY"

cargo run -p palm -- playground infer "Summarize current runtime status"
```

Other supported backend kinds in the current CLI are `anthropic`, `grok`, and `gemini`.

## MapleModelfile

The implemented file format is YAML, not an Ollama-style line-oriented file.

```yaml
kind: MapleModelfile
name: support-local
base: "llama3.2:3b"

defaults:
  temperature: 0.2
  top_p: 0.9
  max_tokens: 2048

templates:
  system: "You are the governed support planner for MAPLE."

contracts:
  - "rcf://safety/v1"

governance:
  allowlists:
    - data_classification: internal
  jurisdictions:
    - CA
  max_cost_per_1k: 0.01
  audit_level: full

benchmarks:
  min_tokens_per_second: 20
  max_ttft_ms: 800
  min_eval_score: 0.8
```

Use this when you need a reusable, versioned local model profile instead of one-off playground settings.

## Local model storage

`maple-model-core` stores models under:

```text
~/.maple/models
```

The store tracks versions, disk usage, and last-use metadata. That is the current implementation backing for future polished `maple model ls` or `maple model rm` style UX.

## Routing and governance

MAPLE routing is where model neutrality becomes operational rather than aspirational.

- Backend allowlists by tenant
- Jurisdiction-aware routing
- Cost ceilings per workload
- Fallback and circuit breaking
- Benchmark gating before promotion

Those controls are implemented in `maple-model-router` and related crates rather than a finished `maple model route` CLI today.

## Benchmarking and improvement

The benchmark surface currently lives in `maple-model-benchmark`. It defines suites, task results, comparisons, and quality gates that Foundry can consume for promotion decisions.

## Current CLI status

The following product-style commands are not exposed in `maple-cli` yet:

```text
maple model pull
maple model ls
maple model run
maple model serve
maple model inspect
maple model rm
maple model benchmark
```

Use Ollama plus PALM playground for operations today, and use the model crates when you need deeper integration.
