# Model Management

MAPLE treats models like governed runtime dependencies. The user-facing mental model is intentionally close to Ollama, but the operating model is broader: local models, hosted APIs, routing policy, and cost-aware enforcement all sit behind the same control surface.

## Core commands

```bash
maple model pull llama3.2:8b-q4
maple model ls
maple model run llama3.2:8b-q4
maple model serve --openai-compatible
maple model inspect llama3.2:8b-q4
maple model rm llama3.2:8b-q4
```

## Local and cloud backends

- Local: Ollama, llama.cpp, private inference services
- Hosted: OpenAI, Anthropic, Gemini, Grok, or other provider adapters

The point of the abstraction is policy control. Agent packages can ask for a capability class or a preferred model profile while the platform decides which backend is allowed in the active environment.

## MapleModelfile

```text
FROM llama3.2:8b-q4
PARAMETER temperature 0.2
PARAMETER top_p 0.9
SYSTEM "You are the governed support planner for MAPLE."
```

Use this when you need a reusable, versioned local model profile instead of one-off CLI arguments.

## Routing policy

```yaml
routes:
  - match:
      dataClass: regulated
    use: ollama:llama3.2:8b-q4

  - match:
      taskClass: deep-reasoning
    use: anthropic:claude-sonnet
```

MAPLE routing is where model neutrality becomes operational rather than aspirational.

## Governance controls

- Backend allowlists by tenant
- Jurisdiction-aware routing
- Cost ceilings per workload
- Approval paths for model swaps
- Benchmark gating before promotion

## Benchmarking and improvement

```bash
maple model benchmark llama3.2:8b-q4 --suite support-routing
```

Foundry can then use the benchmark output to decide whether a student model is ready to take traffic or should remain in shadow mode.
