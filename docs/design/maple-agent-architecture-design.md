# MAPLE AI Agent Architecture Design

## Making MAPLE the World's Best AI Agent Framework

**Version:** 1.0
**Date:** February 2026
**Status:** Design Proposal

---

## Table of Contents

1. [What is an AI Agent?](#1-what-is-an-ai-agent)
2. [Competitive Landscape Analysis](#2-competitive-landscape-analysis)
3. [MAPLE's Unique Value Proposition](#3-maples-unique-value-proposition)
4. [Core Agent Architecture](#4-core-agent-architecture)
5. [The MAPLE Agent Equation](#5-the-maple-agent-equation)
6. [Non-Bypassable Commitment Boundary](#6-non-bypassable-commitment-boundary)
7. [Multi-Provider Cognition Layer](#7-multi-provider-cognition-layer)
8. [Memory Architecture](#8-memory-architecture)
9. [Interoperability Design](#9-interoperability-design)
10. [Implementation Roadmap](#10-implementation-roadmap)

---

## 1. What is an AI Agent?

An **AI Agent** is an autonomous software system that can:

| Capability | Description | MAPLE Implementation |
|------------|-------------|---------------------|
| **Perceive** | Receive and understand inputs from environment | `MeaningFormation` via `resonator-meaning` |
| **Reason** | Process information, plan, and make decisions | `IntentStabilization` via `resonator-intent` |
| **Act** | Execute actions that affect the environment | `CommitmentGateway` via `AgentKernel` |
| **Learn** | Adapt based on feedback and experience | `EVE` (Evidence & Verification Engine) |
| **Persist** | Maintain state and memory across interactions | `Resonator` identity + `maple-storage` |

### Agent Execution Lifecycle

```
┌─────────────────────────────────────────────────────────────────────┐
│                       MAPLE AGENT LIFECYCLE                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    │
│   │ PRESENCE │───>│ COUPLING │───>│ MEANING  │───>│  INTENT  │    │
│   └──────────┘    └──────────┘    └──────────┘    └──────────┘    │
│        │                                               │           │
│        │ Invariant #1             Invariant #2     Invariant #3    │
│        │                                               │           │
│        v                                               v           │
│   ┌──────────┐                                 ┌──────────────┐    │
│   │ ATTENTION│                                 │  COMMITMENT  │    │
│   │  BUDGET  │                                 │              │    │
│   └──────────┘                                 └──────────────┘    │
│        │                                               │           │
│        │ Invariant #5                          Invariant #4        │
│        │                                               │           │
│        v                                               v           │
│   ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    │
│   │ COUPLING │───>│  POLICY  │───>│ GATEWAY  │───>│CONSEQUENCE│   │
│   │  BOUND   │    │  CHECK   │    │ EXECUTE  │    │  RECORD   │    │
│   └──────────┘    └──────────┘    └──────────┘    └──────────┘    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. Competitive Landscape Analysis

### 2.1 OpenAI Agents SDK

**Architecture:**
- Function calling + Assistants API
- Swarm (experimental multi-agent)
- Server-side or client-side tool execution

**Strengths:**
- Simple function calling interface
- Good developer experience
- Strong model capabilities

**Weaknesses:**
- No commitment semantics
- No accountability ledger
- Bypass-prone safety
- Vendor lock-in

### 2.2 Anthropic Claude Code / Agent SDK

**Architecture:**
- Tool use with structured outputs
- MCP (Model Context Protocol) for context
- Sandboxed local execution

**Strengths:**
- Constitutional AI safety approach
- Good sandboxing
- MCP standardization

**Weaknesses:**
- No persistent relationships
- No attention economics
- Policy-based (bypassable) safety
- No cryptographic accountability

### 2.3 Google Gemini / Vertex AI Agents

**Architecture:**
- Agent Builder for no-code/low-code
- Cloud Functions integration
- A2A (Agent-to-Agent) protocol

**Strengths:**
- Enterprise integration
- Cloud-native
- Good observability

**Weaknesses:**
- Ephemeral agent identity
- No resource bounds
- Heavy cloud dependency
- Limited autonomy controls

### 2.4 xAI Grok

**Architecture:**
- API-based, early stage
- OpenAI-compatible endpoints
- Limited tool support

**Strengths:**
- Fast model iteration
- Real-time data access

**Weaknesses:**
- Minimal agent infrastructure
- No governance features
- Limited persistence

### 2.5 Meta Llama

**Architecture:**
- Open weights
- Community implementations (AutoGen, LangChain, etc.)
- BYO infrastructure

**Strengths:**
- Open source
- Privacy (local deployment)
- Customizable

**Weaknesses:**
- No unified agent framework
- User-implemented safety
- Fragmented ecosystem

### 2.6 Comparative Matrix

| Feature | OpenAI | Anthropic | Google | xAI | Meta | **MAPLE** |
|---------|--------|-----------|--------|-----|------|-----------|
| **Commitment Semantics** | None | None | None | None | None | **Built-in** |
| **Accountability Ledger** | Logs | Logs | Logs | Minimal | User | **Cryptographic** |
| **Non-Bypassable Safety** | No | No | No | No | No | **Architectural** |
| **Attention Economics** | No | No | No | No | No | **Native** |
| **Persistent Identity** | Threads | None | None | None | External | **Resonator** |
| **Multi-Agent Coordination** | Swarm | None | A2A | None | External | **Coupling** |
| **Provider Agnostic** | No | No | No | No | Yes | **Yes** |
| **Human Agency Protection** | Policy | Policy | Policy | Policy | User | **Architectural** |
| **Scale Target** | 1000s | 100s | 1000s | 100s | Variable | **100M+** |

---

## 3. MAPLE's Unique Value Proposition

### 3.1 Core Differentiators

#### 1. Commitment-First Architecture

Every consequential action requires an explicit commitment:

```rust
// Other frameworks
agent.execute_tool("transfer_money", args);  // Direct execution, no accountability

// MAPLE
let commitment = kernel.draft_commitment(resonator_id, "transfer_funds", "reason").await?;
let decision = gateway.authorize(commitment).await?;
if decision.allows_execution() {
    let receipt = gateway.execute(cap_id, args, &commitment_id, context).await?;
    // Cryptographic receipt recorded to ledger
}
```

#### 2. Architectural Invariants (Not Policy)

The 9 invariants are enforced at runtime - they cannot be bypassed:

1. **Presence precedes meaning** - No cognition without presence
2. **Meaning precedes intent** - No goals without understanding
3. **Intent precedes commitment** - No promises without clear goals
4. **Commitment precedes consequence** - No side effects without explicit commitment
5. **Coupling bounded by attention** - No unlimited resource consumption
6. **Safety overrides optimization** - Never bypass safety for performance
7. **Human agency cannot be bypassed** - Architectural protection
8. **Failure must be explicit** - Never hide errors
9. **Implementation provenance & evolution** - Operator upgrades require replay verification and evidence anchors

#### 3. Resonance Relationships

Unlike message-passing systems, MAPLE creates stateful relationships:

```
Traditional:  Agent A ──message──> Agent B ──message──> Agent C
              (stateless, isolated, ephemeral)

MAPLE:        Resonator A <══coupling══> Resonator B <══coupling══> Resonator C
              (stateful, relationship-aware, persistent)
```

#### 4. Attention Economics

Finite resources prevent abuse:

```rust
pub struct AttentionBudget {
    pub total_capacity: f64,
    pub allocated: f64,      // Cannot exceed total
    pub safety_reserve: f64, // 10% always reserved
}
```

#### 5. Provider-Agnostic Cognition

Same governance for any LLM backend:

```rust
// All providers produce identical gating behavior
for backend in [Llama, OpenAi, Anthropic, Gemini, Grok] {
    let result = kernel.handle(request.with_backend(backend)).await;
    // Same invariant checks, same commitment requirements, same receipts
}
```

---

## 4. Core Agent Architecture

### 4.1 Architecture Layers

```
┌─────────────────────────────────────────────────────────────────────┐
│                     MAPLE AGENT ARCHITECTURE                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    APPLICATION LAYER                          │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │  │
│  │  │ Mapleverse  │  │ Finalverse  │  │       iBank         │   │  │
│  │  │  (100M+ AI) │  │ (Human-AI)  │  │  (AI Finance)       │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘   │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                               │                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    AGENT KERNEL LAYER                         │  │
│  │  ┌───────────────────────────────────────────────────────┐   │  │
│  │  │              AgentKernel (maple-runtime)              │   │  │
│  │  │  ┌─────────┐ ┌──────────┐ ┌────────┐ ┌────────────┐  │   │  │
│  │  │  │ Agent   │ │Commitment│ │Capability│ │ Invariant │  │   │  │
│  │  │  │ Host    │ │ Gateway  │ │Executor │ │  Guard    │  │   │  │
│  │  │  └─────────┘ └──────────┘ └────────┘ └────────────┘  │   │  │
│  │  └───────────────────────────────────────────────────────┘   │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                               │                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    COGNITION LAYER                            │  │
│  │  ┌─────────────────────────────────────────────────────────┐ │  │
│  │  │              ModelAdapter Interface                     │ │  │
│  │  │  ┌───────┐ ┌────────┐ ┌────────┐ ┌───────┐ ┌───────┐  │ │  │
│  │  │  │ Llama │ │ OpenAI │ │Anthropic│ │Gemini │ │ Grok  │  │ │  │
│  │  │  └───────┘ └────────┘ └────────┘ └───────┘ └───────┘  │ │  │
│  │  └─────────────────────────────────────────────────────────┘ │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                               │                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    GOVERNANCE LAYER                           │  │
│  │  ┌─────────┐  ┌──────────┐  ┌─────────┐  ┌────────────────┐  │  │
│  │  │   AAS   │  │   RCF    │  │   UAL   │  │      EVE       │  │  │
│  │  │Authority│  │Commitment│  │Language │  │ Evidence &     │  │  │
│  │  │& Account│  │  Format  │  │         │  │ Verification   │  │  │
│  │  └─────────┘  └──────────┘  └─────────┘  └────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                               │                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    RESONATOR LAYER                            │  │
│  │  ┌─────────┐ ┌──────────┐ ┌─────────┐ ┌─────────┐ ┌────────┐ │  │
│  │  │Identity │ │ Meaning  │ │ Intent  │ │Commitment│ │Profiles│ │  │
│  │  └─────────┘ └──────────┘ └─────────┘ └─────────┘ └────────┘ │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                               │                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    INFRASTRUCTURE LAYER                       │  │
│  │  ┌───────────┐  ┌──────────┐  ┌───────┐  ┌─────────────────┐ │  │
│  │  │  Storage  │  │ Temporal │  │  MRP  │  │     PALM        │ │  │
│  │  │(Postgres) │  │Coordinator│ │Router │  │ Orchestration   │ │  │
│  │  └───────────┘  └──────────┘  └───────┘  └─────────────────┘ │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 4.2 Component Responsibilities

| Component | Responsibility | Crate |
|-----------|---------------|-------|
| **AgentKernel** | Agent lifecycle, request handling | `maple-runtime` |
| **CommitmentGateway** | Authorization, execution, receipts | `maple-runtime` |
| **InvariantGuard** | Enforce 9 architectural invariants | `maple-runtime` |
| **ModelAdapter** | LLM cognition abstraction | `maple-runtime/cognition` |
| **AAS** | Authority, capability, policy, ledger | `aas-*` |
| **RCF** | Commitment format, validation, audit | `rcf-*` |
| **Resonator** | Identity, meaning, intent, profiles | `resonator-*` |
| **PALM** | Fleet orchestration, deployment | `palm-*` |
| **Storage** | Persistence (Postgres/Memory) | `maple-storage` |

---

## 5. The MAPLE Agent Equation

```
Agent = Resonator + Profile + CapabilitySet + ContractSet + State
```

### 5.1 Resonator (Identity)

The persistent, accountable entity:

```rust
pub struct Resonator {
    pub resonator_id: ResonatorId,      // Unique, persistent identity
    pub identity: IdentityRef,           // Verifiable reference
    pub profile: ResonatorProfile,       // Behavioral configuration
    pub state: ResonatorState,           // Current operational state
    pub capabilities: Vec<CognitiveCapability>,
    pub created_at: DateTime<Utc>,
}
```

### 5.2 Profile (Behavioral Configuration)

Defines how the agent operates:

```rust
pub struct AgentExecutionProfile {
    pub name: String,                          // "coordination", "ibank", etc.
    pub min_intent_confidence: f64,            // Threshold for action (0.65)
    pub require_commitment_for_consequence: bool,  // true for consequential
}

pub struct ResonatorProfile {
    pub name: String,
    pub domains: Vec<EffectDomain>,            // Computation, Finance, Social, Governance
    pub risk_tolerance: RiskTolerance,         // Conservative, Balanced, Aggressive
    pub autonomy_level: AutonomyLevel,         // FullHumanOversight, Guided, High
    pub constraints: Vec<ProfileConstraint>,
}
```

### 5.3 CapabilitySet (What Agent Can Do)

```rust
pub struct CapabilityDescriptor {
    pub name: String,                    // "echo_log", "simulate_transfer"
    pub domain: EffectDomain,            // Finance, Computation, etc.
    pub scope: ScopeConstraint,          // Targets and operations allowed
    pub consequential: bool,             // Requires explicit commitment?
}

// Safe capability - auto-commitment allowed
CapabilityDescriptor::safe("echo_log")

// Dangerous capability - requires explicit commitment
CapabilityDescriptor::dangerous("transfer_funds")
```

### 5.4 ContractSet (Active Commitments)

```rust
pub struct RcfCommitment {
    pub id: CommitmentId,
    pub principal: IdentityRef,          // Who is committing
    pub effect_domain: EffectDomain,     // What domain
    pub scope: ScopeConstraint,          // What scope
    pub temporal_validity: TemporalValidity,  // When valid
    pub required_capabilities: Vec<CapabilityRef>,
    pub intended_outcome: IntendedOutcome,
    pub audit_trail: Option<AuditTrail>,
}
```

### 5.5 State (Runtime Context)

```rust
pub struct AgentState {
    pub resonator_id: ResonatorId,
    pub identity: IdentityRef,
    pub profile: CanonicalResonatorProfile,
    pub attention_budget: AttentionBudget,
    pub coupling_graph: CouplingGraph,
    pub contract_engine: Arc<dyn ContractEngine>,
    pub capability_registry: Arc<CapabilityRegistry>,
    pub policy_engine: Arc<PolicyEngine>,
    pub ledger: Arc<AccountabilityLedger>,
    pub short_memory: Arc<dyn ShortMemoryHandle>,
    pub journal: Arc<dyn JournalSummaryHandle>,
}
```

---

## 6. Non-Bypassable Commitment Boundary

### 6.1 The CommitmentGateway

The **only** path to consequences:

```rust
impl CommitmentGateway {
    /// The ONLY path to consequence execution.
    pub async fn execute(
        &self,
        capability_id: &str,
        params: Value,
        contract_id: &RcfCommitmentId,
        context: CommitmentExecutionContext<'_>,
    ) -> Result<CommitmentExecutionReceipt, CommitmentExecutionError> {
        // a) Contract exists and is active
        // b) Profile allows contract type
        // c) Policy engine approves capability usage
        // d) Capability constraints pass
        // e) Execute tool via connector
        // f) Write ToolCallResult to journal
        // g) Write AccountabilityRecorded with receipt hash
    }
}
```

### 6.2 Execution Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                    COMMITMENT GATEWAY FLOW                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   ┌──────────────────┐                                              │
│   │   Agent Request  │                                              │
│   │ (capability_id,  │                                              │
│   │  params, context)│                                              │
│   └────────┬─────────┘                                              │
│            │                                                        │
│            v                                                        │
│   ┌──────────────────┐     ┌─────────────────────────────────────┐  │
│   │ Contract Exists? │────>│ NO: ContractMissing Error           │  │
│   │   & Active?      │     │ (Invariant #4 violated)             │  │
│   └────────┬─────────┘     └─────────────────────────────────────┘  │
│            │ YES                                                    │
│            v                                                        │
│   ┌──────────────────┐     ┌─────────────────────────────────────┐  │
│   │ Profile Allows   │────>│ NO: PolicyDenied Error              │  │
│   │ Contract Domain? │     │ (Domain mismatch)                   │  │
│   └────────┬─────────┘     └─────────────────────────────────────┘  │
│            │ YES                                                    │
│            v                                                        │
│   ┌──────────────────┐     ┌─────────────────────────────────────┐  │
│   │ Policy Engine    │────>│ NO: PolicyDenied Error              │  │
│   │ Approves?        │     │ (Risk, limits, or review required)  │  │
│   └────────┬─────────┘     └─────────────────────────────────────┘  │
│            │ YES                                                    │
│            v                                                        │
│   ┌──────────────────┐     ┌─────────────────────────────────────┐  │
│   │ Capability       │────>│ NO: CapabilityDenied Error          │  │
│   │ Constraints OK?  │     │ (Scope, limits exceeded)            │  │
│   └────────┬─────────┘     └─────────────────────────────────────┘  │
│            │ YES                                                    │
│            v                                                        │
│   ┌──────────────────┐                                              │
│   │ Journal: Tool    │                                              │
│   │ Call Issued      │                                              │
│   └────────┬─────────┘                                              │
│            │                                                        │
│            v                                                        │
│   ┌──────────────────┐     ┌─────────────────────────────────────┐  │
│   │ Execute Tool     │────>│ ERROR: ToolFailure                  │  │
│   │ (via executor)   │     │ (Record failure receipt)            │  │
│   └────────┬─────────┘     └─────────────────────────────────────┘  │
│            │ SUCCESS                                                │
│            v                                                        │
│   ┌──────────────────┐                                              │
│   │ Journal: Tool    │                                              │
│   │ Call Result      │                                              │
│   └────────┬─────────┘                                              │
│            │                                                        │
│            v                                                        │
│   ┌──────────────────┐                                              │
│   │ Compute Receipt  │                                              │
│   │ Hash (SHA-256)   │                                              │
│   └────────┬─────────┘                                              │
│            │                                                        │
│            v                                                        │
│   ┌──────────────────┐     ┌─────────────────────────────────────┐  │
│   │ Write to Ledger  │────>│ ERROR: ReceiptWriteFailure          │  │
│   │ (AAS)            │     │ (Persistence failed)                │  │
│   └────────┬─────────┘     └─────────────────────────────────────┘  │
│            │ SUCCESS                                                │
│            v                                                        │
│   ┌──────────────────┐                                              │
│   │ Return Receipt   │                                              │
│   │ (receipt_id,     │                                              │
│   │  contract_id,    │                                              │
│   │  hash, result)   │                                              │
│   └──────────────────┘                                              │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 6.3 Bypass Detection

The system actively detects and prevents bypass attempts:

```rust
tokio::task_local! {
    static COMMITMENT_GATEWAY_ACTIVE: bool;
}

// Capability executors can verify they're being called correctly
#[async_trait]
impl CapabilityExecutor for DangerousCapability {
    async fn execute(&self, invocation: &CapabilityInvocation)
        -> Result<CapabilityExecution, CapabilityExecutionError>
    {
        // Panic if called outside gateway (test mode)
        if !commitment_gateway_active() {
            panic!("dangerous capability invoked outside CommitmentGateway");
        }
        if invocation.commitment_id.is_none() {
            panic!("dangerous capability executed without commitment reference");
        }
        // ... execute
    }
}
```

---

## 7. Multi-Provider Cognition Layer

### 7.1 ModelAdapter Trait

Provider-agnostic interface:

```rust
#[async_trait]
pub trait ModelAdapter: Send + Sync {
    fn backend(&self) -> ModelBackend;
    fn config(&self) -> &ModelProviderConfig;

    /// Generate cognition output with validation and repair
    async fn infer(&self, request: &ModelRequest) -> Result<ModelResponse, ModelAdapterError>;

    /// Propose a Meaning draft
    async fn propose_meaning(&self, input: &MeaningInput, state: &CognitionState)
        -> Result<MeaningDraft, ModelAdapterError>;

    /// Propose an Intent draft
    async fn propose_intent(&self, meaning: &MeaningDraft, state: &CognitionState)
        -> Result<IntentDraft, ModelAdapterError>;

    /// Draft an RCF-compatible contract
    async fn draft_contract(&self, intent: &IntentDraft, state: &CognitionState)
        -> Result<ContractDraft, ModelAdapterError>;

    /// Suggest capability calls
    async fn suggest_capability_calls(&self, contract: &ContractDraft, state: &CognitionState)
        -> Result<Vec<CapabilityCallCandidate>, ModelAdapterError>;

    /// Produce episodic summary
    async fn summarize(&self, journal_slice: &[JournalSliceItem])
        -> Result<EpisodicSummary, ModelAdapterError>;
}
```

### 7.2 Supported Backends

| Backend | Implementation | Default Model | Endpoint |
|---------|---------------|---------------|----------|
| **Llama** | `LlamaAdapter` | llama3.2 | localhost:11434 |
| **OpenAI** | `OpenAiAdapter` | gpt-4o-mini | api.openai.com |
| **Anthropic** | `AnthropicAdapter` | claude-3-5-sonnet | api.anthropic.com |
| **Gemini** | `GeminiAdapter` | gemini-2.0-flash | generativelanguage.googleapis.com |
| **Grok** | `GrokAdapter` | grok-2 | api.x.ai |

### 7.3 Structured Output Validation

Three-phase validation ensures safety:

```rust
pub enum ValidationStatus {
    Validated,  // Strict JSON schema parse succeeded
    Repaired,   // Parse succeeded after deterministic repair
    Fallback,   // Could not parse - safe fallback used
}

impl ValidationStatus {
    /// Only validated/repaired outputs can drive tool execution
    pub fn allows_tool_execution(self) -> bool {
        !matches!(self, ValidationStatus::Fallback)
    }
}
```

### 7.4 JSON Repair Pipeline

Handles common LLM output issues:

```rust
fn json_candidates(raw: &str) -> Vec<String> {
    let mut candidates = Vec::new();

    // 1. Raw input as-is
    candidates.push(raw.trim().to_string());

    // 2. Extract from code fence (```json ... ```)
    if let Some(fenced) = extract_json_code_fence(raw) {
        candidates.push(fenced);
    }

    // 3. Extract first JSON object from prose
    if let Some(extracted) = extract_first_json_object(raw) {
        candidates.push(extracted.clone());
        candidates.push(extracted.replace('\'', "\""));  // Single to double quotes
        candidates.push(strip_trailing_commas(&extracted));
    }

    // 4. Strip trailing commas globally
    candidates.push(strip_trailing_commas(raw));

    dedupe_candidates(candidates)
}
```

---

## 8. Memory Architecture

### 8.1 Memory Hierarchy

```
┌─────────────────────────────────────────────────────────────────────┐
│                       MAPLE MEMORY HIERARCHY                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │                  WORKING MEMORY (Short-Term)                │   │
│   │   ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │   │
│   │   │ Last Meaning │  │  Last Intent │  │ Current Context  │  │   │
│   │   │   Summary    │  │              │  │   (Attention)    │  │   │
│   │   └──────────────┘  └──────────────┘  └──────────────────┘  │   │
│   │                                                             │   │
│   │   Trait: ShortMemoryHandle                                  │   │
│   │   Impl:  InMemoryShortMemory                               │   │
│   └─────────────────────────────────────────────────────────────┘   │
│                               │                                     │
│                               v                                     │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │                 JOURNAL MEMORY (Session)                    │   │
│   │   ┌────────────────────────────────────────────────────┐    │   │
│   │   │  Stage Transitions:  Meaning → Intent → Commitment │    │   │
│   │   │  Tool Calls Issued / Results                       │    │   │
│   │   │  Accountability Records (hashes)                   │    │   │
│   │   └────────────────────────────────────────────────────┘    │   │
│   │                                                             │   │
│   │   Trait: JournalSummaryHandle                              │   │
│   │   Impl:  InMemoryJournalSummary                            │   │
│   └─────────────────────────────────────────────────────────────┘   │
│                               │                                     │
│                               v                                     │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │              EPISODIC MEMORY (Summarized)                   │   │
│   │   ┌────────────────────────────────────────────────────┐    │   │
│   │   │  Summarized journal slices via ModelAdapter        │    │   │
│   │   │  Key points, open questions                        │    │   │
│   │   └────────────────────────────────────────────────────┘    │   │
│   │                                                             │   │
│   │   Generated by: ModelAdapter::summarize()                  │   │
│   └─────────────────────────────────────────────────────────────┘   │
│                               │                                     │
│                               v                                     │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │              PERSISTENT MEMORY (Long-Term)                  │   │
│   │   ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │   │
│   │   │  Commitment  │  │    Audit     │  │      Agent       │  │   │
│   │   │   Records    │  │    Chains    │  │   Checkpoints    │  │   │
│   │   └──────────────┘  └──────────────┘  └──────────────────┘  │   │
│   │                                                             │   │
│   │   Trait: MapleStorage                                      │   │
│   │   Impl:  PostgreSQL / InMemory                             │   │
│   └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 8.2 Storage Abstraction

```rust
#[async_trait]
pub trait MapleStorage: Send + Sync {
    // Commitment records (system of record)
    async fn write_commitment(&self, ...) -> StorageResult<()>;
    async fn read_commitment(&self, ...) -> StorageResult<CommitmentRecord>;

    // Append-only audit chains
    async fn append_audit(&self, ...) -> StorageResult<()>;
    async fn audit_range(&self, ...) -> StorageResult<Vec<AuditRecord>>;

    // Agent checkpoints for restart/resume
    async fn checkpoint_agent(&self, ...) -> StorageResult<()>;
    async fn resume_agent(&self, ...) -> StorageResult<AgentCheckpoint>;

    // Semantic memory for cognition assistance
    async fn semantic_store(&self, ...) -> StorageResult<()>;
}
```

---

## 9. Interoperability Design

### 9.1 External Protocol Mapping

#### MCP (Model Context Protocol) Integration

```
MCP Tools → MAPLE Capabilities

MCP Tool Definition:
{
  "name": "read_file",
  "description": "Read file contents",
  "inputSchema": { "path": "string" }
}

MAPLE Capability:
CapabilityDescriptor {
    name: "read_file",
    domain: EffectDomain::Computation,
    scope: ScopeConstraint::new(["fs:*"], ["read"]),
    consequential: false  // Read is safe
}
```

#### Google A2A Integration

```
A2A Messages → MAPLE Presence/Coupling Events

A2A AgentInfo → Resonator Presence Signal
A2A TaskRequest → Intent Formation Input
A2A TaskResponse → Consequence Record
```

#### Vendor Agent SDK Integration

```
┌─────────────────────────────────────────────────────────────────────┐
│              VENDOR SDK INTEGRATION ARCHITECTURE                    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │                    External World                           │   │
│   │   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │   │
│   │   │ OpenAI   │  │ Claude   │  │ Gemini   │  │  Grok    │   │   │
│   │   │ Agents   │  │ Agents   │  │ Agents   │  │ Agents   │   │   │
│   │   └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │   │
│   └────────┼─────────────┼─────────────┼─────────────┼─────────┘   │
│            │             │             │             │              │
│            v             v             v             v              │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │                  MAPLE Interop Gateway                      │   │
│   │   ┌─────────────────────────────────────────────────────┐   │   │
│   │   │              Protocol Adapters                      │   │   │
│   │   │  ┌───────┐ ┌─────────┐ ┌────────┐ ┌─────────────┐  │   │   │
│   │   │  │ MCP   │ │   A2A   │ │  REST  │ │  WebSocket  │  │   │   │
│   │   │  └───────┘ └─────────┘ └────────┘ └─────────────┘  │   │   │
│   │   └─────────────────────────────────────────────────────┘   │   │
│   │                           │                                 │   │
│   │                           v                                 │   │
│   │   ┌─────────────────────────────────────────────────────┐   │   │
│   │   │           Capability Proxy (ALWAYS gates)           │   │   │
│   │   │                                                     │   │   │
│   │   │  1. Map external request → Capability invocation    │   │   │
│   │   │  2. Require commitment for consequential ops        │   │   │
│   │   │  3. Route through CommitmentGateway                 │   │   │
│   │   │  4. Return result with receipt                      │   │   │
│   │   └─────────────────────────────────────────────────────┘   │   │
│   └─────────────────────────────────────────────────────────────┘   │
│                           │                                         │
│                           v                                         │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │                   MAPLE Agent Kernel                        │   │
│   │          (Full invariant + AAS + ledger enforcement)        │   │
│   └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 9.2 Key Integration Principle

> **External protocols are ONLY transports/connectors.**
> **They can NEVER bypass MAPLE gates.**

```rust
// WRONG: Direct external tool execution
async fn handle_mcp_tool(request: McpToolRequest) -> McpToolResponse {
    let result = tool_executor.execute(request.tool, request.args).await;  // BYPASS!
    McpToolResponse { result }
}

// CORRECT: Route through MAPLE gates
async fn handle_mcp_tool(request: McpToolRequest, kernel: &AgentKernel) -> McpToolResponse {
    // Map MCP tool to MAPLE capability
    let capability = map_mcp_to_capability(&request.tool)?;

    // If consequential, require commitment
    let commitment = if capability.consequential {
        Some(kernel.draft_commitment(resonator_id, &capability.name, "MCP request").await?)
    } else {
        None
    };

    // Execute through kernel (full gating)
    let handle_request = AgentHandleRequest {
        resonator_id,
        backend: ModelBackend::LocalLlama,
        prompt: format!("Execute MCP tool: {}", request.tool),
        override_tool: Some(capability.name),
        override_args: Some(request.args),
        commitment,
    };

    let response = kernel.handle(handle_request).await?;

    McpToolResponse {
        result: response.action.map(|a| a.payload),
        receipt_id: response.audit_event_id,
    }
}
```

---

## 10. Implementation Roadmap

### Phase 1: Core Agent Kernel (Current)

**Status:** Implemented in `maple-runtime/src/agent_kernel/`

- [x] AgentKernel with full invariant enforcement
- [x] CommitmentGateway with receipt generation
- [x] Multi-provider ModelAdapter (Llama, OpenAI, Anthropic, Gemini, Grok)
- [x] Structured output validation and repair
- [x] Safe/dangerous capability distinction
- [x] AAS integration (capability, policy, ledger)
- [x] Journal and audit trail

### Phase 2: Enhanced Cognition Pipeline

**Status:** In Progress

- [ ] Full meaning formation engine with convergence tracking
- [ ] Intent stabilization with temporal checks
- [ ] Contract drafting improvements
- [ ] Multi-turn conversation support
- [ ] Enhanced memory summarization

### Phase 3: Interoperability Layer

**Status:** Planned

- [ ] MCP protocol adapter
- [ ] A2A protocol adapter
- [ ] Generic webhook/REST integration
- [ ] WebSocket streaming support

### Phase 4: Distributed Agents

**Status:** Planned

- [ ] Multi-node agent deployment
- [ ] Cross-node coupling support
- [ ] Federated learning integration
- [ ] Global consensus for high-value commitments

### Phase 5: Platform Features

**Status:** Planned

- [ ] Mapleverse alpha (100K+ agent coordination)
- [ ] Finalverse alpha (human-AI coexistence)
- [ ] iBank alpha (autonomous finance)
- [ ] Web dashboard and observability

---

## Appendix A: Error Taxonomy

```rust
pub enum AgentKernelError {
    // Identity/Registration
    AgentNotFound(String),
    Runtime(String),

    // Authorization
    Aas(String),
    CapabilityDenied,
    CapabilityDeniedDetail(String),
    PolicyDenied(String),

    // Commitment
    ContractMissing { reason: String },
    CommitmentValidation(String),
    CommitmentCapabilityMismatch { capability, commitment_id, reason },
    ApprovalRequired(Decision),

    // Execution
    ModelAdapterMissing(String),
    Model(String),
    UnknownCapability(String),
    ExecutorMissing(String),
    ToolFailure(String),
    CapabilityExecution(String),

    // Persistence
    Storage(String),
    ReceiptWriteFailure(String),

    // Invariants
    Invariant(InvariantViolation),
    InvariantContractViolation(String),
}
```

---

## Appendix B: Configuration Examples

### Mapleverse (Pure AI)

```rust
let config = RuntimeConfig {
    profiles: ProfileConfig {
        human_profiles_allowed: false,
        allowed_profiles: vec![ResonatorProfile::Coordination],
    },
    coupling: CouplingConfig {
        require_explicit_intent: true,
        require_commitment_for_state_change: true,
    },
    commitment: CommitmentConfig {
        require_audit_trail: true,
        require_digital_signature: true,
    },
};
```

### Finalverse (Human-AI)

```rust
let config = RuntimeConfig {
    profiles: ProfileConfig {
        human_profiles_allowed: true,
        allowed_profiles: vec![ResonatorProfile::Human, ResonatorProfile::World],
    },
    safety: SafetyConfig {
        human_agency_protection: true,
        coercion_detection: true,
        emotional_exploitation_prevention: true,
        reversible_consequences_preferred: true,
    },
};
```

### iBank (AI Finance)

```rust
let config = RuntimeConfig {
    profiles: ProfileConfig {
        human_profiles_allowed: false,
        allowed_profiles: vec![ResonatorProfile::IBank],
    },
    commitment: CommitmentConfig {
        require_audit_trail: true,
        require_risk_assessment: true,
        require_digital_signature: true,
    },
    consequence: ConsequenceConfig {
        maximum_autonomous_consequence_value: 1_000_000.0,
        require_reversibility_assessment: true,
    },
};
```

---

## Conclusion

MAPLE's agent architecture is designed to be the **world's best AI Agent framework** by:

1. **Enforcing architectural invariants** - Safety that cannot be bypassed
2. **Requiring explicit commitments** - Full accountability for every action
3. **Supporting multiple cognition providers** - No vendor lock-in
4. **Providing attention economics** - Bounded, sustainable operation
5. **Enabling persistent relationships** - Beyond stateless message passing
6. **Scaling to 100M+ agents** - Enterprise and research ready

The key insight is that **governance is not separate from execution** - it's woven into the fabric of how agents operate. This makes MAPLE uniquely positioned to power the next generation of autonomous AI systems.
