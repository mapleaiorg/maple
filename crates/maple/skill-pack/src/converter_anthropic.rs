//! Anthropic tool format converter (D-03).
//!
//! Converts Anthropic tool definitions (from the Messages API) and
//! SKILL.md-based skill directories into the canonical Maple Skill Pack format.
//!
//! # Anthropic Tool Format (Messages API)
//!
//! ```json
//! {
//!   "name": "get_weather",
//!   "description": "Get the current weather in a location",
//!   "input_schema": {
//!     "type": "object",
//!     "properties": {
//!       "location": {
//!         "type": "string",
//!         "description": "City and state, e.g. San Francisco, CA"
//!       }
//!     },
//!     "required": ["location"]
//!   }
//! }
//! ```
//!
//! # SKILL.md Format
//!
//! A directory-based skill definition containing:
//! - `SKILL.md` — Markdown file with YAML frontmatter defining name, description, inputs, outputs
//! - Optional additional resources (templates, examples, etc.)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    manifest::{
        CapabilityRequirements, ResourceLimits, SandboxConfig, SandboxType, SkillManifest,
        SkillMetadata, SkillMetadataExtra,
    },
    IoField, SkillError, SkillPack,
};

/// An Anthropic tool definition (from the Messages API `tools` array).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicToolDef {
    /// Tool name (becomes the skill name).
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    /// Tool type — "custom" (default), "computer_20241022", "text_editor_20241022", "bash_20241022".
    #[serde(rename = "type", default = "default_tool_type")]
    pub tool_type: String,
    /// Cache control (for prompt caching).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
}

fn default_tool_type() -> String {
    "custom".into()
}

/// YAML frontmatter from a SKILL.md file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMdFrontmatter {
    /// Skill name.
    pub name: String,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Version string.
    #[serde(default = "default_version")]
    pub version: String,
    /// Author.
    #[serde(default)]
    pub author: Option<String>,
    /// Tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Input parameters (name → type description).
    #[serde(default)]
    pub inputs: BTreeMap<String, SkillMdField>,
    /// Output fields (name → type description).
    #[serde(default)]
    pub outputs: BTreeMap<String, SkillMdField>,
    /// Required capabilities.
    #[serde(default)]
    pub capabilities: Vec<String>,
}

fn default_version() -> String {
    "0.1.0".into()
}

/// A field definition from SKILL.md frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMdField {
    /// Field type.
    #[serde(rename = "type", default = "default_field_type")]
    pub field_type: String,
    /// Whether the field is required.
    #[serde(default)]
    pub required: bool,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Default value.
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}

fn default_field_type() -> String {
    "string".into()
}

/// Options for customizing the conversion from Anthropic format.
#[derive(Debug, Clone)]
pub struct AnthropicConvertOptions {
    /// Version to assign to the converted skill (default: 0.1.0).
    pub version: semver::Version,
    /// Author to set on the converted skill.
    pub author: Option<String>,
    /// Capability IDs to require (default: empty).
    pub capabilities: Vec<String>,
    /// Resource limits (default: conservative defaults).
    pub resources: ResourceLimits,
    /// Sandbox config (default: Process with 30s timeout).
    pub sandbox: SandboxConfig,
    /// Additional tags (default: ["anthropic", "converted"]).
    pub tags: Vec<String>,
}

impl Default for AnthropicConvertOptions {
    fn default() -> Self {
        Self {
            version: semver::Version::new(0, 1, 0),
            author: None,
            capabilities: Vec::new(),
            resources: ResourceLimits {
                max_compute_ms: 30_000,
                max_memory_bytes: 52_428_800,
                max_network_bytes: 10_485_760,
                max_storage_bytes: None,
                max_llm_tokens: None,
            },
            sandbox: SandboxConfig {
                sandbox_type: SandboxType::Process,
                timeout_ms: 30_000,
            },
            tags: vec!["anthropic".into(), "converted".into()],
        }
    }
}

/// Convert an Anthropic tool definition to a Maple SkillPack.
///
/// Maps the Anthropic `input_schema` JSON Schema to Maple's I/O field model.
pub fn from_anthropic_tool(
    tool: &AnthropicToolDef,
    opts: &AnthropicConvertOptions,
) -> Result<SkillPack, SkillError> {
    if tool.name.is_empty() {
        return Err(SkillError::InvalidManifest(
            "Anthropic tool name is empty".into(),
        ));
    }

    // Reject built-in tool types that aren't custom tools
    match tool.tool_type.as_str() {
        "custom" | "" => {}
        built_in => {
            // Built-in tools (computer, text_editor, bash) get special handling
            return from_anthropic_builtin(built_in, tool, opts);
        }
    }

    // Extract inputs from input_schema
    let inputs = extract_inputs_from_schema(tool.input_schema.as_ref())?;

    // If no inputs were extracted, create a minimal passthrough input
    let inputs = if inputs.is_empty() {
        let mut m = BTreeMap::new();
        m.insert(
            "input".into(),
            IoField {
                field_type: "any".into(),
                required: false,
                default: None,
                description: "Passthrough input".into(),
            },
        );
        m
    } else {
        inputs
    };

    // Standard output field (Anthropic tools don't define output schemas)
    let mut outputs = BTreeMap::new();
    outputs.insert(
        "result".into(),
        IoField {
            field_type: "any".into(),
            required: true,
            default: None,
            description: "Tool execution result".into(),
        },
    );

    let manifest = SkillManifest {
        skill: SkillMetadata {
            name: tool.name.clone(),
            version: opts.version.clone(),
            description: if tool.description.is_empty() {
                format!("Converted from Anthropic tool '{}'", tool.name)
            } else {
                tool.description.clone()
            },
            author: opts.author.clone(),
        },
        inputs,
        outputs,
        capabilities: CapabilityRequirements {
            required: opts.capabilities.clone(),
        },
        resources: opts.resources.clone(),
        sandbox: opts.sandbox.clone(),
        metadata: Some(SkillMetadataExtra {
            tags: opts.tags.clone(),
            license: None,
        }),
    };

    let pack = SkillPack {
        manifest,
        policies: Vec::new(),
        golden_traces: Vec::new(),
        source_path: None,
    };

    pack.validate()?;
    Ok(pack)
}

/// Convert an Anthropic built-in tool type to a SkillPack with appropriate
/// pre-configured inputs and capabilities.
fn from_anthropic_builtin(
    builtin_type: &str,
    tool: &AnthropicToolDef,
    opts: &AnthropicConvertOptions,
) -> Result<SkillPack, SkillError> {
    let (inputs, outputs, caps, desc) = match builtin_type {
        t if t.starts_with("computer") => {
            let mut inputs = BTreeMap::new();
            inputs.insert(
                "action".into(),
                IoField {
                    field_type: "string".into(),
                    required: true,
                    default: None,
                    description: "Action to perform: key, type, mouse_move, left_click, etc.".into(),
                },
            );
            inputs.insert(
                "coordinate".into(),
                IoField {
                    field_type: "array".into(),
                    required: false,
                    default: None,
                    description: "Screen coordinates [x, y]".into(),
                },
            );
            inputs.insert(
                "text".into(),
                IoField {
                    field_type: "string".into(),
                    required: false,
                    default: None,
                    description: "Text to type or key to press".into(),
                },
            );

            let mut outputs = BTreeMap::new();
            outputs.insert(
                "screenshot".into(),
                IoField {
                    field_type: "string".into(),
                    required: false,
                    default: None,
                    description: "Base64-encoded screenshot".into(),
                },
            );

            let caps = vec!["cap-computer-use".into()];
            let desc = "Anthropic computer use tool — interact with a desktop environment";
            (inputs, outputs, caps, desc)
        }
        t if t.starts_with("text_editor") => {
            let mut inputs = BTreeMap::new();
            inputs.insert(
                "command".into(),
                IoField {
                    field_type: "string".into(),
                    required: true,
                    default: None,
                    description: "Editor command: view, create, str_replace, insert, undo_edit".into(),
                },
            );
            inputs.insert(
                "path".into(),
                IoField {
                    field_type: "string".into(),
                    required: true,
                    default: None,
                    description: "File path".into(),
                },
            );
            inputs.insert(
                "old_str".into(),
                IoField {
                    field_type: "string".into(),
                    required: false,
                    default: None,
                    description: "String to replace (for str_replace)".into(),
                },
            );
            inputs.insert(
                "new_str".into(),
                IoField {
                    field_type: "string".into(),
                    required: false,
                    default: None,
                    description: "Replacement string".into(),
                },
            );

            let mut outputs = BTreeMap::new();
            outputs.insert(
                "result".into(),
                IoField {
                    field_type: "string".into(),
                    required: true,
                    default: None,
                    description: "Editor operation result".into(),
                },
            );

            let caps = vec!["cap-file-system".into()];
            let desc = "Anthropic text editor tool — view and edit files";
            (inputs, outputs, caps, desc)
        }
        t if t.starts_with("bash") => {
            let mut inputs = BTreeMap::new();
            inputs.insert(
                "command".into(),
                IoField {
                    field_type: "string".into(),
                    required: true,
                    default: None,
                    description: "Bash command to execute".into(),
                },
            );
            inputs.insert(
                "restart".into(),
                IoField {
                    field_type: "boolean".into(),
                    required: false,
                    default: Some(serde_json::json!(false)),
                    description: "Whether to restart the bash session".into(),
                },
            );

            let mut outputs = BTreeMap::new();
            outputs.insert(
                "stdout".into(),
                IoField {
                    field_type: "string".into(),
                    required: false,
                    default: None,
                    description: "Standard output".into(),
                },
            );
            outputs.insert(
                "stderr".into(),
                IoField {
                    field_type: "string".into(),
                    required: false,
                    default: None,
                    description: "Standard error".into(),
                },
            );

            let caps = vec!["cap-shell-exec".into()];
            let desc = "Anthropic bash tool — execute shell commands";
            (inputs, outputs, caps, desc)
        }
        _ => {
            return Err(SkillError::InvalidManifest(format!(
                "unknown Anthropic built-in tool type: '{builtin_type}'"
            )));
        }
    };

    let mut all_caps = opts.capabilities.clone();
    for cap in caps {
        if !all_caps.contains(&cap) {
            all_caps.push(cap);
        }
    }

    let manifest = SkillManifest {
        skill: SkillMetadata {
            name: tool.name.clone(),
            version: opts.version.clone(),
            description: if tool.description.is_empty() {
                desc.to_string()
            } else {
                tool.description.clone()
            },
            author: opts.author.clone(),
        },
        inputs,
        outputs,
        capabilities: CapabilityRequirements {
            required: all_caps,
        },
        resources: opts.resources.clone(),
        sandbox: opts.sandbox.clone(),
        metadata: Some(SkillMetadataExtra {
            tags: {
                let mut tags = opts.tags.clone();
                tags.push("builtin".into());
                tags
            },
            license: None,
        }),
    };

    let pack = SkillPack {
        manifest,
        policies: Vec::new(),
        golden_traces: Vec::new(),
        source_path: None,
    };

    pack.validate()?;
    Ok(pack)
}

/// Convert a raw JSON value (Anthropic tool definition) to a SkillPack.
pub fn from_anthropic_json(
    json: &serde_json::Value,
    opts: &AnthropicConvertOptions,
) -> Result<SkillPack, SkillError> {
    let tool: AnthropicToolDef = serde_json::from_value(json.clone()).map_err(|e| {
        SkillError::InvalidManifest(format!("failed to parse Anthropic tool definition: {e}"))
    })?;
    from_anthropic_tool(&tool, opts)
}

/// Convert multiple Anthropic tool definitions from a JSON array.
pub fn from_anthropic_tools_array(
    tools: &[serde_json::Value],
    opts: &AnthropicConvertOptions,
) -> Result<Vec<SkillPack>, SkillError> {
    tools.iter().map(|t| from_anthropic_json(t, opts)).collect()
}

/// Parse a SKILL.md file's frontmatter and convert to a SkillPack.
///
/// SKILL.md format:
/// ```text
/// ---
/// name: my-skill
/// description: Does something useful
/// version: "1.0.0"
/// tags: [utility, text]
/// inputs:
///   query:
///     type: string
///     required: true
///     description: The search query
/// outputs:
///   result:
///     type: string
///     description: The result
/// ---
///
/// # My Skill
///
/// Extended documentation here...
/// ```
pub fn from_skill_md(
    content: &str,
    skill_path: &str,
    opts: &AnthropicConvertOptions,
) -> Result<SkillPack, SkillError> {
    // Extract YAML frontmatter between --- markers
    let frontmatter = extract_frontmatter(content)?;

    let fm: SkillMdFrontmatter = serde_yaml_ng::from_str(&frontmatter).map_err(|e| {
        SkillError::InvalidManifest(format!("SKILL.md YAML parse error: {e}"))
    })?;

    if fm.name.is_empty() {
        return Err(SkillError::InvalidManifest(
            "SKILL.md name cannot be empty".into(),
        ));
    }

    let version = semver::Version::parse(&fm.version).map_err(|e| {
        SkillError::InvalidManifest(format!("invalid version '{}': {e}", fm.version))
    })?;

    // Convert frontmatter fields to IoField
    let inputs: BTreeMap<String, IoField> = fm
        .inputs
        .into_iter()
        .map(|(name, field)| {
            (
                name,
                IoField {
                    field_type: field.field_type,
                    required: field.required,
                    default: field.default,
                    description: field.description,
                },
            )
        })
        .collect();

    let outputs: BTreeMap<String, IoField> = fm
        .outputs
        .into_iter()
        .map(|(name, field)| {
            (
                name,
                IoField {
                    field_type: field.field_type,
                    required: field.required,
                    default: field.default,
                    description: field.description,
                },
            )
        })
        .collect();

    // Ensure at least one input and output
    let inputs = if inputs.is_empty() {
        let mut m = BTreeMap::new();
        m.insert(
            "input".into(),
            IoField {
                field_type: "any".into(),
                required: false,
                default: None,
                description: "Passthrough input".into(),
            },
        );
        m
    } else {
        inputs
    };

    let outputs = if outputs.is_empty() {
        let mut m = BTreeMap::new();
        m.insert(
            "result".into(),
            IoField {
                field_type: "any".into(),
                required: true,
                default: None,
                description: "Skill execution result".into(),
            },
        );
        m
    } else {
        outputs
    };

    let mut tags = fm.tags;
    for t in &opts.tags {
        if !tags.contains(t) {
            tags.push(t.clone());
        }
    }

    let mut capabilities = fm.capabilities;
    for c in &opts.capabilities {
        if !capabilities.contains(c) {
            capabilities.push(c.clone());
        }
    }

    let manifest = SkillManifest {
        skill: SkillMetadata {
            name: fm.name,
            version,
            description: fm.description,
            author: fm.author.or_else(|| opts.author.clone()),
        },
        inputs,
        outputs,
        capabilities: CapabilityRequirements {
            required: capabilities,
        },
        resources: opts.resources.clone(),
        sandbox: opts.sandbox.clone(),
        metadata: Some(SkillMetadataExtra {
            tags,
            license: None,
        }),
    };

    let pack = SkillPack {
        manifest,
        policies: Vec::new(),
        golden_traces: Vec::new(),
        source_path: Some(std::path::PathBuf::from(skill_path)),
    };

    pack.validate()?;
    Ok(pack)
}

/// Extract YAML frontmatter from a markdown file (between `---` markers).
fn extract_frontmatter(content: &str) -> Result<String, SkillError> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err(SkillError::InvalidManifest(
            "SKILL.md must start with YAML frontmatter (---) delimiter".into(),
        ));
    }

    let after_first = &trimmed[3..];
    let end_pos = after_first.find("\n---").ok_or_else(|| {
        SkillError::InvalidManifest("SKILL.md missing closing frontmatter (---) delimiter".into())
    })?;

    Ok(after_first[..end_pos].to_string())
}

/// Extract Maple IoField inputs from an Anthropic `input_schema` JSON Schema.
fn extract_inputs_from_schema(
    schema: Option<&serde_json::Value>,
) -> Result<BTreeMap<String, IoField>, SkillError> {
    let mut inputs = BTreeMap::new();

    let schema = match schema {
        Some(s) => s,
        None => return Ok(inputs),
    };

    let obj = schema.as_object().ok_or_else(|| {
        SkillError::InvalidManifest("input_schema must be a JSON object".into())
    })?;

    // Extract required fields list
    let required_fields: Vec<String> = obj
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // Extract properties
    if let Some(properties) = obj.get("properties").and_then(|v| v.as_object()) {
        for (name, prop_schema) in properties {
            let field_type = prop_schema
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("any")
                .to_string();

            let description = prop_schema
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let default = prop_schema.get("default").cloned();

            inputs.insert(
                name.clone(),
                IoField {
                    field_type,
                    required: required_fields.contains(name),
                    default,
                    description,
                },
            );
        }
    }

    Ok(inputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_anthropic_tool_basic() {
        let tool_json = serde_json::json!({
            "name": "get_weather",
            "description": "Get the current weather in a location",
            "input_schema": {
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City and state, e.g. San Francisco, CA"
                    },
                    "unit": {
                        "type": "string",
                        "description": "Temperature unit",
                        "default": "celsius"
                    }
                },
                "required": ["location"]
            }
        });

        let opts = AnthropicConvertOptions::default();
        let pack = from_anthropic_json(&tool_json, &opts).unwrap();

        assert_eq!(pack.name(), "get_weather");
        assert_eq!(
            pack.manifest.skill.description,
            "Get the current weather in a location"
        );
        assert_eq!(pack.manifest.inputs.len(), 2);
        assert!(pack.manifest.inputs["location"].required);
        assert!(!pack.manifest.inputs["unit"].required);
        assert_eq!(pack.manifest.inputs["location"].field_type, "string");
        assert_eq!(
            pack.manifest.inputs["unit"].default,
            Some(serde_json::json!("celsius"))
        );
    }

    #[test]
    fn convert_anthropic_no_schema() {
        let tool_json = serde_json::json!({
            "name": "get_time",
            "description": "Get the current time"
        });

        let opts = AnthropicConvertOptions::default();
        let pack = from_anthropic_json(&tool_json, &opts).unwrap();

        assert_eq!(pack.name(), "get_time");
        assert!(pack.manifest.inputs.contains_key("input"));
    }

    #[test]
    fn convert_anthropic_empty_name_error() {
        let tool_json = serde_json::json!({
            "name": "",
            "description": "Bad tool"
        });

        let opts = AnthropicConvertOptions::default();
        let result = from_anthropic_json(&tool_json, &opts);
        assert!(result.is_err());
    }

    #[test]
    fn convert_anthropic_computer_use() {
        let tool_json = serde_json::json!({
            "name": "computer",
            "type": "computer_20241022",
            "display_width_px": 1024,
            "display_height_px": 768
        });

        let tool: AnthropicToolDef = serde_json::from_value(tool_json).unwrap();
        let opts = AnthropicConvertOptions::default();
        let pack = from_anthropic_tool(&tool, &opts).unwrap();

        assert_eq!(pack.name(), "computer");
        assert!(pack.manifest.inputs.contains_key("action"));
        assert!(pack.manifest.inputs.contains_key("coordinate"));
        assert!(pack.required_capabilities().contains(&"cap-computer-use".to_string()));
        let meta = pack.manifest.metadata.as_ref().unwrap();
        assert!(meta.tags.contains(&"builtin".to_string()));
    }

    #[test]
    fn convert_anthropic_text_editor() {
        let tool_json = serde_json::json!({
            "name": "str_replace_editor",
            "type": "text_editor_20241022"
        });

        let tool: AnthropicToolDef = serde_json::from_value(tool_json).unwrap();
        let opts = AnthropicConvertOptions::default();
        let pack = from_anthropic_tool(&tool, &opts).unwrap();

        assert_eq!(pack.name(), "str_replace_editor");
        assert!(pack.manifest.inputs.contains_key("command"));
        assert!(pack.manifest.inputs.contains_key("path"));
        assert!(pack.required_capabilities().contains(&"cap-file-system".to_string()));
    }

    #[test]
    fn convert_anthropic_bash_tool() {
        let tool_json = serde_json::json!({
            "name": "bash",
            "type": "bash_20241022"
        });

        let tool: AnthropicToolDef = serde_json::from_value(tool_json).unwrap();
        let opts = AnthropicConvertOptions::default();
        let pack = from_anthropic_tool(&tool, &opts).unwrap();

        assert_eq!(pack.name(), "bash");
        assert!(pack.manifest.inputs.contains_key("command"));
        assert!(pack.manifest.outputs.contains_key("stdout"));
        assert!(pack.manifest.outputs.contains_key("stderr"));
        assert!(pack.required_capabilities().contains(&"cap-shell-exec".to_string()));
    }

    #[test]
    fn convert_anthropic_multiple_tools() {
        let tools = vec![
            serde_json::json!({
                "name": "search",
                "description": "Search documents",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" }
                    },
                    "required": ["query"]
                }
            }),
            serde_json::json!({
                "name": "lookup",
                "description": "Lookup a record by ID",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }),
        ];

        let opts = AnthropicConvertOptions::default();
        let packs = from_anthropic_tools_array(&tools, &opts).unwrap();

        assert_eq!(packs.len(), 2);
        assert_eq!(packs[0].name(), "search");
        assert_eq!(packs[1].name(), "lookup");
    }

    #[test]
    fn convert_skill_md_basic() {
        let content = r#"---
name: summarize
description: Summarize text content
version: "1.0.0"
author: maple-team
tags:
  - text
  - nlp
inputs:
  text:
    type: string
    required: true
    description: Text to summarize
  max_length:
    type: integer
    required: false
    description: Maximum summary length
    default: 200
outputs:
  summary:
    type: string
    required: true
    description: Generated summary
capabilities:
  - cap-llm
---

# Summarize Skill

This skill takes text and produces a concise summary.
"#;

        let opts = AnthropicConvertOptions::default();
        let pack = from_skill_md(content, "/skills/summarize/SKILL.md", &opts).unwrap();

        assert_eq!(pack.name(), "summarize");
        assert_eq!(pack.manifest.skill.description, "Summarize text content");
        assert_eq!(pack.version(), &semver::Version::new(1, 0, 0));
        assert_eq!(pack.manifest.skill.author.as_deref(), Some("maple-team"));
        assert_eq!(pack.manifest.inputs.len(), 2);
        assert!(pack.manifest.inputs["text"].required);
        assert!(!pack.manifest.inputs["max_length"].required);
        assert_eq!(
            pack.manifest.inputs["max_length"].default,
            Some(serde_json::json!(200))
        );
        assert_eq!(pack.manifest.outputs.len(), 1);
        assert!(pack.manifest.outputs.contains_key("summary"));
        assert!(pack.required_capabilities().contains(&"cap-llm".to_string()));

        let meta = pack.manifest.metadata.as_ref().unwrap();
        assert!(meta.tags.contains(&"text".to_string()));
        assert!(meta.tags.contains(&"nlp".to_string()));
        assert!(meta.tags.contains(&"anthropic".to_string()));
    }

    #[test]
    fn convert_skill_md_minimal() {
        let content = r#"---
name: ping
description: Simple ping tool
---

Returns pong.
"#;

        let opts = AnthropicConvertOptions::default();
        let pack = from_skill_md(content, "/skills/ping/SKILL.md", &opts).unwrap();

        assert_eq!(pack.name(), "ping");
        // Should have passthrough input and generic output
        assert!(pack.manifest.inputs.contains_key("input"));
        assert!(pack.manifest.outputs.contains_key("result"));
    }

    #[test]
    fn convert_skill_md_no_frontmatter_error() {
        let content = "# Just a plain markdown file\n\nNo frontmatter here.";
        let opts = AnthropicConvertOptions::default();
        let result = from_skill_md(content, "/test", &opts);
        assert!(result.is_err());
    }

    #[test]
    fn convert_skill_md_unclosed_frontmatter_error() {
        let content = "---\nname: broken\n\nNo closing delimiter.";
        let opts = AnthropicConvertOptions::default();
        let result = from_skill_md(content, "/test", &opts);
        assert!(result.is_err());
    }

    #[test]
    fn extract_frontmatter_basic() {
        let content = "---\nname: test\nversion: \"1.0.0\"\n---\n\nBody text.";
        let fm = extract_frontmatter(content).unwrap();
        assert!(fm.contains("name: test"));
        assert!(fm.contains("version: \"1.0.0\""));
    }

    #[test]
    fn convert_anthropic_complex_schema() {
        let tool_json = serde_json::json!({
            "name": "create_event",
            "description": "Create a calendar event",
            "input_schema": {
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Event title"
                    },
                    "start_time": {
                        "type": "string",
                        "description": "ISO 8601 start time"
                    },
                    "end_time": {
                        "type": "string",
                        "description": "ISO 8601 end time"
                    },
                    "attendees": {
                        "type": "array",
                        "description": "List of attendee email addresses"
                    },
                    "is_recurring": {
                        "type": "boolean",
                        "description": "Whether the event repeats",
                        "default": false
                    }
                },
                "required": ["title", "start_time", "end_time"]
            }
        });

        let opts = AnthropicConvertOptions::default();
        let pack = from_anthropic_json(&tool_json, &opts).unwrap();

        assert_eq!(pack.manifest.inputs.len(), 5);
        assert!(pack.manifest.inputs["title"].required);
        assert!(pack.manifest.inputs["start_time"].required);
        assert!(pack.manifest.inputs["end_time"].required);
        assert!(!pack.manifest.inputs["attendees"].required);
        assert!(!pack.manifest.inputs["is_recurring"].required);
        assert_eq!(pack.manifest.inputs["attendees"].field_type, "array");
        assert_eq!(pack.manifest.inputs["is_recurring"].field_type, "boolean");
    }
}
