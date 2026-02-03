# 🗣️ Universal Agent Language (UAL)

UAL is the **interaction language** for humans and agents. It is SQL‑like DDL/DML designed for coordination, governance, and action requests.  
UAL statements compile into **RCF (Resonance Commitment Format)** artifacts or **PALM operations**.

**Key principle**:  
**UAL is what you say. RCF is what you sign.**

## Relationship to RCF

- **UAL** expresses intent in a human/agent‑friendly, declarative form.
- **RCF** is the formal commitment artifact used for validation, adjudication, and execution.
- The compiler bridges the two: `UAL → RCF`.

## UAL v0.1 Grammar (Supported)

### Commitments (RCF)

```
COMMIT BY <principal>
    DOMAIN <domain>
    OUTCOME <string>
    [SCOPE GLOBAL | SCOPE <target>]
    [TARGET <id>]...
    [TAG <tag>]...
    [REVERSIBLE | IRREVERSIBLE]
    [VALID_FROM <RFC3339>]
    [VALID_UNTIL <RFC3339>]
;
```

### Operations (PALM)

```
CREATE SPEC <spec_id> [VERSION <version>];
UPDATE SPEC <spec_id> [VERSION <version>];
DEPRECATE SPEC <spec_id>;

CREATE DEPLOYMENT SPEC <spec_id> [REPLICAS <n>];
UPDATE DEPLOYMENT <deployment_id>;
SCALE DEPLOYMENT <deployment_id> TO <n>;
DELETE DEPLOYMENT <deployment_id>;
ROLLBACK DEPLOYMENT <deployment_id>;
PAUSE DEPLOYMENT <deployment_id>;
RESUME DEPLOYMENT <deployment_id>;

RESTART INSTANCE <instance_id>;
TERMINATE INSTANCE <instance_id>;
MIGRATE INSTANCE <instance_id>;
DRAIN INSTANCE <instance_id>;

CHECKPOINT INSTANCE <instance_id>;
RESTORE CHECKPOINT <instance_id>;
DELETE CHECKPOINT <snapshot_id>;

HEALTH CHECK INSTANCE <instance_id>;
FORCE RECOVERY INSTANCE <instance_id>;

CONFIGURE POLICY <policy_name>;
VIEW AUDIT LOG <filter>;
```

## Examples

### 1. Commitment

```
COMMIT BY agent-001
  DOMAIN Computation
  OUTCOME "Generate monthly risk report"
  SCOPE GLOBAL
  TAG compliance
  IRREVERSIBLE
  VALID_FROM 2026-02-03T09:00:00Z
  VALID_UNTIL 2026-03-03T09:00:00Z;
```

### 2. Deployment

```
CREATE DEPLOYMENT SPEC risk-agent REPLICAS 3;
SCALE DEPLOYMENT dep-123 TO 5;
```

## LLM Integration (Ollama/GPT/Claude)

LLMs should **produce UAL** (human‑readable intent). The system compiles UAL to RCF and validates it.

Recommended flow:

1. LLM generates a UAL statement.
2. `ual-compiler` converts it to RCF / PALM operations.
3. `rcf-validator` validates commitments.
4. AAS adjudicates, and Mapleverse executes.

This keeps LLMs in **proposal mode**, while MAPLE enforces governance and accountability.
