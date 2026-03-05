//! End-to-end integration tests for the Maple Skill Pack pipeline (E-01).
//!
//! Tests the complete lifecycle:
//! 1. Convert vendor tool definitions → SkillPack
//! 2. Load packs from disk
//! 3. Register in a SkillRegistry
//! 4. Search and discover by name, tag, capability
//! 5. Validate inputs against manifests
//! 6. Execute golden trace conformance checks

use maple_skill_pack::{
    converter_anthropic::{self, AnthropicConvertOptions, AnthropicToolDef},
    converter_openai::{self, OpenAiConvertOptions},
    loader::SkillPackLoader,
    manifest::SkillManifest,
    policy::PolicyEffect,
    registry::{SkillRegistry, SkillSource},
    SkillPack,
};
use std::io::Write;

// ──────────────────────────────────────────────────────────────────────
//  E2E: OpenAI → SkillPack → Registry → Discovery
// ──────────────────────────────────────────────────────────────────────

#[test]
fn openai_tool_to_registry_roundtrip() {
    let tools = vec![
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "web_search",
                "description": "Search the web for information",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        },
                        "max_results": {
                            "type": "integer",
                            "description": "Maximum results",
                            "default": 10
                        }
                    },
                    "required": ["query"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "code_execute",
                "description": "Execute code in a sandbox",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "language": {
                            "type": "string",
                            "description": "Programming language"
                        },
                        "code": {
                            "type": "string",
                            "description": "Code to execute"
                        }
                    },
                    "required": ["language", "code"]
                }
            }
        }),
    ];

    let opts = OpenAiConvertOptions {
        tags: vec!["openai".into(), "converted".into(), "api-tools".into()],
        ..Default::default()
    };

    // Step 1: Convert all tools
    let packs = converter_openai::from_openai_tools_array(&tools, &opts).unwrap();
    assert_eq!(packs.len(), 2);

    // Step 2: Register in registry
    let mut registry = SkillRegistry::new();
    for pack in packs {
        let source = SkillSource::OpenAI {
            original_tool: serde_json::json!({}),
        };
        registry.register(pack, source).unwrap();
    }

    // Step 3: Verify discovery
    assert_eq!(registry.list().len(), 2);

    // By name
    let search_skill = registry.get_by_name("web_search").unwrap();
    assert_eq!(search_skill.pack.manifest.inputs.len(), 2);
    assert!(search_skill.pack.manifest.inputs["query"].required);

    let code_skill = registry.get_by_name("code_execute").unwrap();
    assert_eq!(code_skill.pack.manifest.inputs.len(), 2);
    assert!(code_skill.pack.manifest.inputs["language"].required);
    assert!(code_skill.pack.manifest.inputs["code"].required);

    // By tag
    let api_skills = registry.find_by_tag("api-tools");
    assert_eq!(api_skills.len(), 2);

    // By keyword search
    let results = registry.search("web");
    assert!(results.iter().any(|s| s.pack.name() == "web_search"));

    let results = registry.search("code");
    assert!(results.iter().any(|s| s.pack.name() == "code_execute"));
}

// ──────────────────────────────────────────────────────────────────────
//  E2E: Anthropic → SkillPack → Registry → Discovery
// ──────────────────────────────────────────────────────────────────────

#[test]
fn anthropic_tool_to_registry_roundtrip() {
    let tools = vec![
        serde_json::json!({
            "name": "analyze_sentiment",
            "description": "Analyze the sentiment of text",
            "input_schema": {
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Text to analyze"
                    },
                    "language": {
                        "type": "string",
                        "description": "Language code (e.g. en, fr)",
                        "default": "en"
                    }
                },
                "required": ["text"]
            }
        }),
        serde_json::json!({
            "name": "translate",
            "description": "Translate text between languages",
            "input_schema": {
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Text to translate"
                    },
                    "source_lang": {
                        "type": "string",
                        "description": "Source language"
                    },
                    "target_lang": {
                        "type": "string",
                        "description": "Target language"
                    }
                },
                "required": ["text", "target_lang"]
            }
        }),
    ];

    let opts = AnthropicConvertOptions {
        tags: vec!["anthropic".into(), "converted".into(), "nlp".into()],
        capabilities: vec!["cap-llm".into()],
        ..Default::default()
    };

    // Convert
    let packs = converter_anthropic::from_anthropic_tools_array(&tools, &opts).unwrap();
    assert_eq!(packs.len(), 2);

    // Register
    let mut registry = SkillRegistry::new();
    for pack in packs {
        let source = SkillSource::Anthropic {
            skill_path: "/tools/nlp".into(),
        };
        registry.register(pack, source).unwrap();
    }

    // Verify
    assert_eq!(registry.list().len(), 2);

    let sentiment = registry.get_by_name("analyze_sentiment").unwrap();
    assert!(sentiment.pack.manifest.inputs["text"].required);
    assert!(!sentiment.pack.manifest.inputs["language"].required);

    // By capability
    let llm_skills = registry.find_by_capability("cap-llm");
    assert_eq!(llm_skills.len(), 2);

    // By tag
    let nlp_skills = registry.find_by_tag("nlp");
    assert_eq!(nlp_skills.len(), 2);
}

// ──────────────────────────────────────────────────────────────────────
//  E2E: Mixed vendor tools in a single registry
// ──────────────────────────────────────────────────────────────────────

#[test]
fn mixed_vendor_registry() {
    let mut registry = SkillRegistry::new();

    // Register an OpenAI tool
    let openai_tool = serde_json::json!({
        "type": "function",
        "function": {
            "name": "dalle_generate",
            "description": "Generate an image with DALL-E",
            "parameters": {
                "type": "object",
                "properties": {
                    "prompt": { "type": "string", "description": "Image description" },
                    "size": { "type": "string", "default": "1024x1024" }
                },
                "required": ["prompt"]
            }
        }
    });
    let openai_opts = OpenAiConvertOptions {
        tags: vec!["openai".into(), "image".into()],
        ..Default::default()
    };
    let pack = converter_openai::from_openai_json(&openai_tool, &openai_opts).unwrap();
    registry
        .register(pack, SkillSource::OpenAI { original_tool: openai_tool.clone() })
        .unwrap();

    // Register an Anthropic tool
    let anthropic_tool = serde_json::json!({
        "name": "claude_analyze",
        "description": "Analyze document with Claude",
        "input_schema": {
            "type": "object",
            "properties": {
                "document": { "type": "string", "description": "Document content" },
                "question": { "type": "string", "description": "Question about the document" }
            },
            "required": ["document", "question"]
        }
    });
    let anthropic_opts = AnthropicConvertOptions {
        tags: vec!["anthropic".into(), "analysis".into()],
        ..Default::default()
    };
    let pack = converter_anthropic::from_anthropic_json(&anthropic_tool, &anthropic_opts).unwrap();
    registry
        .register(pack, SkillSource::Anthropic { skill_path: "/tools/analyze".into() })
        .unwrap();

    // Register a native MAPLE skill
    let native_manifest = SkillManifest::from_toml(
        r#"
[skill]
name = "maple-resonance"
version = "1.0.0"
description = "Native MAPLE resonance analysis"

[inputs.worldline_id]
type = "string"
required = true
description = "WorldLine ID to analyze"

[outputs.resonance_score]
type = "number"
description = "Resonance score (0.0-1.0)"

[capabilities]
required = ["cap-resonance"]

[resources]
max_compute_ms = 5000
max_memory_bytes = 10485760
max_network_bytes = 0

[sandbox]
type = "trusted"
timeout_ms = 10000

[metadata]
tags = ["maple", "native", "resonance"]
"#,
    )
    .unwrap();
    let native_pack = SkillPack {
        manifest: native_manifest,
        policies: Vec::new(),
        golden_traces: Vec::new(),
        source_path: None,
    };
    registry.register(native_pack, SkillSource::Native).unwrap();

    // Verify mixed registry
    assert_eq!(registry.list().len(), 3);

    // Each source type is represented
    let dalle = registry.get_by_name("dalle_generate").unwrap();
    assert!(matches!(&dalle.source, SkillSource::OpenAI { .. }));

    let claude = registry.get_by_name("claude_analyze").unwrap();
    assert!(matches!(&claude.source, SkillSource::Anthropic { .. }));

    let native = registry.get_by_name("maple-resonance").unwrap();
    assert!(matches!(&native.source, SkillSource::Native));

    // Cross-vendor search works
    let image_skills = registry.find_by_tag("image");
    assert_eq!(image_skills.len(), 1);
    assert_eq!(image_skills[0].pack.name(), "dalle_generate");

    let resonance = registry.find_by_capability("cap-resonance");
    assert_eq!(resonance.len(), 1);
}

// ──────────────────────────────────────────────────────────────────────
//  E2E: Disk load → Register → Validate → Golden trace
// ──────────────────────────────────────────────────────────────────────

#[test]
fn disk_load_to_golden_trace_validation() {
    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join("text-classify");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::create_dir_all(skill_dir.join("tests/golden")).unwrap();

    // Write manifest.toml
    let manifest = r#"[skill]
name = "text-classify"
version = "1.2.0"
description = "Classify text into categories"
author = "maple-team"

[inputs.text]
type = "string"
required = true
description = "Text to classify"

[inputs.categories]
type = "array"
required = true
description = "Possible categories"

[outputs.category]
type = "string"
description = "Predicted category"

[outputs.confidence]
type = "number"
description = "Confidence score"

[capabilities]
required = ["cap-llm"]

[resources]
max_compute_ms = 10000
max_memory_bytes = 52428800
max_network_bytes = 5242880

[sandbox]
type = "process"
timeout_ms = 15000

[metadata]
tags = ["nlp", "classification", "text"]
license = "MIT"
"#;
    std::fs::write(skill_dir.join("manifest.toml"), manifest).unwrap();

    // Write policy.toml
    let policy = r#"[[policies]]
name = "rate-limit"
effect = "deny"
reason = "Rate limit exceeded"

[policies.condition]
type = "rate_exceeds"
resource = "text-classify"
max_per_minute = 60
"#;
    std::fs::write(skill_dir.join("policy.toml"), policy).unwrap();

    // Write golden trace
    let golden = serde_json::json!([{
        "name": "basic_classification",
        "description": "Classify a simple text",
        "input": {
            "text": "The stock market crashed today",
            "categories": ["politics", "sports", "finance", "technology"]
        },
        "expected_output": {
            "category": "finance"
        }
    }]);
    let golden_path = skill_dir.join("tests/golden/basic.json");
    let mut f = std::fs::File::create(&golden_path).unwrap();
    write!(f, "{}", serde_json::to_string_pretty(&golden).unwrap()).unwrap();

    // Step 1: Load from disk
    let loader = SkillPackLoader::new(vec![dir.path().to_path_buf()]);
    let packs = loader.load_all().unwrap();
    assert_eq!(packs.len(), 1);
    let pack = &packs[0];

    // Verify loaded content
    assert_eq!(pack.name(), "text-classify");
    assert_eq!(pack.version(), &semver::Version::new(1, 2, 0));
    assert_eq!(pack.manifest.inputs.len(), 2);
    assert_eq!(pack.manifest.outputs.len(), 2);
    assert_eq!(pack.policies.len(), 1);
    assert_eq!(pack.policies[0].effect, PolicyEffect::Deny);
    assert_eq!(pack.golden_traces.len(), 1);

    // Step 2: Validate a well-formed input
    let valid_input = serde_json::json!({
        "text": "Hello world",
        "categories": ["greeting", "farewell"]
    });
    assert!(pack.manifest.validate_input(&valid_input).is_ok());

    // Step 3: Validate a malformed input (missing required field)
    let bad_input = serde_json::json!({
        "categories": ["a", "b"]
    });
    assert!(pack.manifest.validate_input(&bad_input).is_err());

    // Step 4: Register in registry
    let mut registry = SkillRegistry::new();
    registry.register(pack.clone(), SkillSource::Native).unwrap();

    let found = registry.get_by_name("text-classify").unwrap();
    assert_eq!(found.pack.policies.len(), 1);

    // Step 5: Golden trace conformance
    let trace = &pack.golden_traces[0];
    assert_eq!(trace.name, "basic_classification");

    // Simulate actual output matching expected
    let actual_output = serde_json::json!({
        "category": "finance",
        "confidence": 0.95
    });
    assert!(trace.matches_output(&actual_output));

    // Wrong category should fail
    let wrong_output = serde_json::json!({
        "category": "sports",
        "confidence": 0.3
    });
    assert!(!trace.matches_output(&wrong_output));

    // Step 6: Discovery
    let nlp_skills = registry.find_by_tag("nlp");
    assert_eq!(nlp_skills.len(), 1);
    assert_eq!(nlp_skills[0].pack.name(), "text-classify");

    let llm_skills = registry.find_by_capability("cap-llm");
    assert_eq!(llm_skills.len(), 1);
}

// ──────────────────────────────────────────────────────────────────────
//  E2E: SKILL.md → SkillPack → Register
// ──────────────────────────────────────────────────────────────────────

#[test]
fn skill_md_to_registry_pipeline() {
    let skill_md = r#"---
name: code-review
description: Automated code review with structured feedback
version: "2.0.0"
author: maple-platform
tags:
  - code
  - review
  - quality
inputs:
  code:
    type: string
    required: true
    description: Source code to review
  language:
    type: string
    required: true
    description: Programming language
  rules:
    type: array
    required: false
    description: Custom lint rules
outputs:
  issues:
    type: array
    required: true
    description: List of issues found
  score:
    type: number
    required: true
    description: Code quality score (0-100)
capabilities:
  - cap-llm
  - cap-code-analysis
---

# Code Review Skill

This skill performs automated code review and provides structured feedback.

## Usage

Provide source code and the programming language. Optionally specify custom rules.

## Output

Returns a list of issues with severity levels and a quality score.
"#;

    let opts = AnthropicConvertOptions {
        tags: vec!["anthropic".into(), "converted".into()],
        ..Default::default()
    };
    let pack =
        converter_anthropic::from_skill_md(skill_md, "/skills/code-review/SKILL.md", &opts)
            .unwrap();

    assert_eq!(pack.name(), "code-review");
    assert_eq!(pack.version(), &semver::Version::new(2, 0, 0));
    assert_eq!(pack.manifest.inputs.len(), 3);
    assert_eq!(pack.manifest.outputs.len(), 2);
    assert!(pack.manifest.inputs["code"].required);
    assert!(pack.manifest.inputs["language"].required);
    assert!(!pack.manifest.inputs["rules"].required);

    // Register
    let mut registry = SkillRegistry::new();
    registry
        .register(
            pack,
            SkillSource::Anthropic {
                skill_path: "/skills/code-review/SKILL.md".into(),
            },
        )
        .unwrap();

    // Discovery
    let code_skills = registry.find_by_tag("code");
    assert_eq!(code_skills.len(), 1);

    let analysis_skills = registry.find_by_capability("cap-code-analysis");
    assert_eq!(analysis_skills.len(), 1);
    assert_eq!(analysis_skills[0].pack.name(), "code-review");
}

// ──────────────────────────────────────────────────────────────────────
//  E2E: Anthropic built-in tools pipeline
// ──────────────────────────────────────────────────────────────────────

#[test]
fn anthropic_builtin_tools_pipeline() {
    let mut registry = SkillRegistry::new();

    // Register all Anthropic built-in tool types
    let builtins = vec![
        serde_json::json!({
            "name": "computer",
            "type": "computer_20241022",
            "display_width_px": 1024,
            "display_height_px": 768
        }),
        serde_json::json!({
            "name": "str_replace_editor",
            "type": "text_editor_20241022"
        }),
        serde_json::json!({
            "name": "bash",
            "type": "bash_20241022"
        }),
    ];

    let opts = AnthropicConvertOptions::default();

    for tool_json in &builtins {
        let tool: AnthropicToolDef = serde_json::from_value(tool_json.clone()).unwrap();
        let pack = converter_anthropic::from_anthropic_tool(&tool, &opts).unwrap();
        registry
            .register(
                pack,
                SkillSource::Anthropic {
                    skill_path: format!("builtin:{}", tool.name),
                },
            )
            .unwrap();
    }

    assert_eq!(registry.list().len(), 3);

    // Verify capabilities
    let computer_skills = registry.find_by_capability("cap-computer-use");
    assert_eq!(computer_skills.len(), 1);
    assert_eq!(computer_skills[0].pack.name(), "computer");

    let fs_skills = registry.find_by_capability("cap-file-system");
    assert_eq!(fs_skills.len(), 1);
    assert_eq!(fs_skills[0].pack.name(), "str_replace_editor");

    let shell_skills = registry.find_by_capability("cap-shell-exec");
    assert_eq!(shell_skills.len(), 1);
    assert_eq!(shell_skills[0].pack.name(), "bash");

    // All built-in tools should have the "builtin" tag
    let builtin_skills = registry.find_by_tag("builtin");
    assert_eq!(builtin_skills.len(), 3);
}

// ──────────────────────────────────────────────────────────────────────
//  E2E: Unregister and re-register
// ──────────────────────────────────────────────────────────────────────

#[test]
fn unregister_and_reregister_lifecycle() {
    let mut registry = SkillRegistry::new();

    // Register
    let tool_json = serde_json::json!({
        "name": "temp_skill",
        "description": "A temporary skill",
        "input_schema": {
            "type": "object",
            "properties": {
                "data": { "type": "string" }
            },
            "required": ["data"]
        }
    });
    let opts = AnthropicConvertOptions::default();
    let pack = converter_anthropic::from_anthropic_json(&tool_json, &opts).unwrap();
    let skill_id = registry.register(pack.clone(), SkillSource::Native).unwrap();

    assert_eq!(registry.list().len(), 1);
    assert!(registry.get(&skill_id).is_some());
    assert!(registry.get_by_name("temp_skill").is_some());

    // Unregister
    registry.unregister(&skill_id).unwrap();
    assert_eq!(registry.list().len(), 0);
    assert!(registry.get(&skill_id).is_none());
    assert!(registry.get_by_name("temp_skill").is_none());

    // Re-register (fresh ID)
    let new_id = registry.register(pack, SkillSource::Native).unwrap();
    assert_ne!(skill_id, new_id); // Should get a new ID
    assert_eq!(registry.list().len(), 1);
    assert!(registry.get_by_name("temp_skill").is_some());
}

// ──────────────────────────────────────────────────────────────────────
//  E2E: Large-scale registry performance sanity
// ──────────────────────────────────────────────────────────────────────

#[test]
fn registry_handles_many_skills() {
    let mut registry = SkillRegistry::new();
    let count = 200;

    let opts = OpenAiConvertOptions::default();

    for i in 0..count {
        let tool_json = serde_json::json!({
            "name": format!("skill_{:04}", i),
            "description": format!("Auto-generated skill number {}", i),
            "parameters": {
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                },
                "required": ["input"]
            }
        });
        let pack = converter_openai::from_openai_json(&tool_json, &opts).unwrap();
        registry.register(pack, SkillSource::Native).unwrap();
    }

    assert_eq!(registry.list().len(), count);

    // Search should still work efficiently
    let found = registry.get_by_name("skill_0100");
    assert!(found.is_some());

    let results = registry.search("skill_01");
    assert!(!results.is_empty());

    // All should have the "openai" tag
    let tagged = registry.find_by_tag("openai");
    assert_eq!(tagged.len(), count);
}

// ──────────────────────────────────────────────────────────────────────
//  E2E: Policy + validation integration
// ──────────────────────────────────────────────────────────────────────

#[test]
fn skill_with_policies_and_validation() {
    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join("rate-limited-api");
    std::fs::create_dir_all(&skill_dir).unwrap();

    // Manifest
    std::fs::write(
        skill_dir.join("manifest.toml"),
        r#"[skill]
name = "rate-limited-api"
version = "1.0.0"
description = "An API with rate limiting"

[inputs.endpoint]
type = "string"
required = true
description = "API endpoint"

[inputs.payload]
type = "object"
required = false
description = "Request payload"

[outputs.response]
type = "object"
description = "API response"

[capabilities]
required = ["cap-network-access"]

[resources]
max_compute_ms = 5000
max_memory_bytes = 10485760
max_network_bytes = 1048576

[sandbox]
type = "process"
timeout_ms = 10000

[metadata]
tags = ["api", "network", "rate-limited"]
"#,
    )
    .unwrap();

    // Policy with multiple rules
    std::fs::write(
        skill_dir.join("policy.toml"),
        r#"[[policies]]
name = "rate-limit"
effect = "deny"
reason = "Too many requests per minute"

[policies.condition]
type = "rate_exceeds"
resource = "api-calls"
max_per_minute = 30

[[policies]]
name = "budget-guard"
effect = "deny"
reason = "Network budget exceeded"

[policies.condition]
type = "budget_exceeds"
resource = "network-bytes"
max_value = 1048576
"#,
    )
    .unwrap();

    // Load
    let loader = SkillPackLoader::new(vec![dir.path().to_path_buf()]);
    let packs = loader.load_all().unwrap();
    assert_eq!(packs.len(), 1);

    let pack = &packs[0];
    assert_eq!(pack.policies.len(), 2);
    assert_eq!(pack.policies[0].name, "rate-limit");
    assert_eq!(pack.policies[0].effect, PolicyEffect::Deny);
    assert_eq!(pack.policies[1].name, "budget-guard");

    // Validate good input
    let good_input = serde_json::json!({"endpoint": "/api/v1/data"});
    assert!(pack.manifest.validate_input(&good_input).is_ok());

    // Validate bad input (missing required)
    let bad_input = serde_json::json!({"payload": {"key": "value"}});
    assert!(pack.manifest.validate_input(&bad_input).is_err());

    // Register and verify
    let mut registry = SkillRegistry::new();
    registry.register(pack.clone(), SkillSource::Native).unwrap();

    let found = registry.find_by_tag("rate-limited");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].pack.policies.len(), 2);
}
