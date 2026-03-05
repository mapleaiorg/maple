//! OpenAI tool format converter (D-02).
//!
//! Converts OpenAI function-calling tool definitions into the canonical
//! Maple Skill Pack format. Handles both the legacy `functions` format
//! and the modern `tools` array format.
//!
//! # OpenAI Tool Format
//!
//! ```json
//! {
//!   "type": "function",
//!   "function": {
//!     "name": "get_weather",
//!     "description": "Get the current weather in a city",
//!     "parameters": {
//!       "type": "object",
//!       "properties": {
//!         "city": { "type": "string", "description": "City name" },
//!         "units": { "type": "string", "enum": ["celsius", "fahrenheit"] }
//!       },
//!       "required": ["city"]
//!     },
//!     "strict": true
//!   }
//! }
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    manifest::{
        CapabilityRequirements, ResourceLimits, SandboxConfig, SandboxType, SkillManifest,
        SkillMetadata, SkillMetadataExtra,
    },
    IoField, SkillError, SkillPack,
};

/// An OpenAI tool definition (the `tools` array element).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiToolDef {
    /// Tool type — always "function" for function-calling tools.
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function definition.
    pub function: OpenAiFunctionDef,
}

/// An OpenAI function definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiFunctionDef {
    /// Function name (becomes the skill name).
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// JSON Schema for the function parameters.
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
    /// Whether strict mode is enabled (OpenAI structured outputs).
    #[serde(default)]
    pub strict: Option<bool>,
}

/// Options for customizing the conversion from OpenAI format.
#[derive(Debug, Clone)]
pub struct OpenAiConvertOptions {
    /// Version to assign to the converted skill (default: 0.1.0).
    pub version: semver::Version,
    /// Author to set on the converted skill.
    pub author: Option<String>,
    /// Capability IDs to require (default: empty).
    pub capabilities: Vec<String>,
    /// Resource limits (default: conservative defaults).
    pub resources: ResourceLimits,
    /// Sandbox config (default: Process with 15s timeout).
    pub sandbox: SandboxConfig,
    /// Additional tags (default: ["openai", "converted"]).
    pub tags: Vec<String>,
}

impl Default for OpenAiConvertOptions {
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
            tags: vec!["openai".into(), "converted".into()],
        }
    }
}

/// Convert an OpenAI tool definition to a Maple SkillPack.
///
/// Extracts the function name, description, and JSON Schema parameters,
/// mapping them to the canonical Maple I/O format.
pub fn from_openai_tool(tool: &OpenAiToolDef, opts: &OpenAiConvertOptions) -> Result<SkillPack, SkillError> {
    if tool.tool_type != "function" {
        return Err(SkillError::InvalidManifest(format!(
            "unsupported OpenAI tool type '{}', expected 'function'",
            tool.tool_type
        )));
    }
    from_openai_function(&tool.function, opts)
}

/// Convert an OpenAI function definition to a Maple SkillPack.
///
/// This handles the inner `function` object directly, useful for
/// legacy `functions` array format.
pub fn from_openai_function(
    func: &OpenAiFunctionDef,
    opts: &OpenAiConvertOptions,
) -> Result<SkillPack, SkillError> {
    if func.name.is_empty() {
        return Err(SkillError::InvalidManifest(
            "OpenAI function name is empty".into(),
        ));
    }

    // Extract inputs from JSON Schema parameters
    let inputs = extract_inputs_from_schema(func.parameters.as_ref())?;

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

    // Create a standard output field (OpenAI tools don't define output schemas)
    let mut outputs = BTreeMap::new();
    outputs.insert(
        "result".into(),
        IoField {
            field_type: "any".into(),
            required: true,
            default: None,
            description: "Function execution result".into(),
        },
    );

    let manifest = SkillManifest {
        skill: SkillMetadata {
            name: func.name.clone(),
            version: opts.version.clone(),
            description: if func.description.is_empty() {
                format!("Converted from OpenAI function '{}'", func.name)
            } else {
                func.description.clone()
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

/// Convert a raw JSON value (OpenAI tool definition) to a SkillPack.
///
/// Accepts either:
/// - A full tool object `{"type": "function", "function": {...}}`
/// - A bare function object `{"name": "...", "parameters": {...}}`
pub fn from_openai_json(
    json: &serde_json::Value,
    opts: &OpenAiConvertOptions,
) -> Result<SkillPack, SkillError> {
    // Try parsing as full tool definition first
    if let Ok(tool) = serde_json::from_value::<OpenAiToolDef>(json.clone()) {
        return from_openai_tool(&tool, opts);
    }

    // Try parsing as bare function definition
    if let Ok(func) = serde_json::from_value::<OpenAiFunctionDef>(json.clone()) {
        return from_openai_function(&func, opts);
    }

    Err(SkillError::InvalidManifest(
        "could not parse as OpenAI tool or function definition".into(),
    ))
}

/// Convert multiple OpenAI tool definitions from a JSON array.
pub fn from_openai_tools_array(
    tools: &[serde_json::Value],
    opts: &OpenAiConvertOptions,
) -> Result<Vec<SkillPack>, SkillError> {
    tools
        .iter()
        .map(|t| from_openai_json(t, opts))
        .collect()
}

/// Extract Maple IoField inputs from an OpenAI JSON Schema `parameters` object.
fn extract_inputs_from_schema(
    schema: Option<&serde_json::Value>,
) -> Result<BTreeMap<String, IoField>, SkillError> {
    let mut inputs = BTreeMap::new();

    let schema = match schema {
        Some(s) => s,
        None => return Ok(inputs),
    };

    let obj = schema.as_object().ok_or_else(|| {
        SkillError::InvalidManifest("parameters must be a JSON object".into())
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
            let field = json_schema_to_io_field(prop_schema, required_fields.contains(name));
            inputs.insert(name.clone(), field);
        }
    }

    Ok(inputs)
}

/// Convert a single JSON Schema property to a Maple IoField.
fn json_schema_to_io_field(schema: &serde_json::Value, required: bool) -> IoField {
    let field_type = schema
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("any")
        .to_string();

    let description = schema
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let default = schema.get("default").cloned();

    IoField {
        field_type,
        required,
        default,
        description,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_openai_tool_full() {
        let tool_json = serde_json::json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get current weather for a location",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "city": {
                            "type": "string",
                            "description": "City name"
                        },
                        "units": {
                            "type": "string",
                            "description": "Temperature units",
                            "default": "celsius"
                        }
                    },
                    "required": ["city"]
                }
            }
        });

        let opts = OpenAiConvertOptions::default();
        let pack = from_openai_json(&tool_json, &opts).unwrap();

        assert_eq!(pack.name(), "get_weather");
        assert_eq!(
            pack.manifest.skill.description,
            "Get current weather for a location"
        );
        assert_eq!(pack.manifest.inputs.len(), 2);
        assert!(pack.manifest.inputs["city"].required);
        assert!(!pack.manifest.inputs["units"].required);
        assert_eq!(pack.manifest.inputs["city"].field_type, "string");
        assert_eq!(pack.manifest.inputs["units"].description, "Temperature units");
        assert_eq!(
            pack.manifest.inputs["units"].default,
            Some(serde_json::json!("celsius"))
        );
        // Output should be a generic result
        assert!(pack.manifest.outputs.contains_key("result"));
    }

    #[test]
    fn convert_openai_bare_function() {
        let func_json = serde_json::json!({
            "name": "calculate",
            "description": "Evaluate a math expression",
            "parameters": {
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "Math expression"
                    }
                },
                "required": ["expression"]
            }
        });

        let opts = OpenAiConvertOptions::default();
        let pack = from_openai_json(&func_json, &opts).unwrap();

        assert_eq!(pack.name(), "calculate");
        assert_eq!(pack.manifest.inputs.len(), 1);
        assert!(pack.manifest.inputs["expression"].required);
    }

    #[test]
    fn convert_openai_no_parameters() {
        let func_json = serde_json::json!({
            "name": "get_time",
            "description": "Get the current time"
        });

        let opts = OpenAiConvertOptions::default();
        let pack = from_openai_json(&func_json, &opts).unwrap();

        assert_eq!(pack.name(), "get_time");
        // Should have a passthrough input
        assert_eq!(pack.manifest.inputs.len(), 1);
        assert!(pack.manifest.inputs.contains_key("input"));
    }

    #[test]
    fn convert_openai_strict_mode() {
        let tool_json = serde_json::json!({
            "type": "function",
            "function": {
                "name": "structured_output",
                "description": "Returns structured data",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" }
                    },
                    "required": ["query"]
                },
                "strict": true
            }
        });

        let opts = OpenAiConvertOptions::default();
        let pack = from_openai_json(&tool_json, &opts).unwrap();
        assert_eq!(pack.name(), "structured_output");
    }

    #[test]
    fn convert_openai_empty_name_error() {
        let func_json = serde_json::json!({
            "name": "",
            "description": "Bad function"
        });

        let opts = OpenAiConvertOptions::default();
        let result = from_openai_json(&func_json, &opts);
        assert!(result.is_err());
    }

    #[test]
    fn convert_openai_wrong_type_error() {
        let tool_json = serde_json::json!({
            "type": "code_interpreter",
            "function": {
                "name": "test"
            }
        });

        let opts = OpenAiConvertOptions::default();
        let result = from_openai_json(&tool_json, &opts);
        assert!(result.is_err());
    }

    #[test]
    fn convert_openai_multiple_tools() {
        let tools = vec![
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "search",
                    "description": "Search the web",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Search query" }
                        },
                        "required": ["query"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "fetch",
                    "description": "Fetch a URL",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "url": { "type": "string", "description": "URL to fetch" }
                        },
                        "required": ["url"]
                    }
                }
            }),
        ];

        let opts = OpenAiConvertOptions::default();
        let packs = from_openai_tools_array(&tools, &opts).unwrap();

        assert_eq!(packs.len(), 2);
        assert_eq!(packs[0].name(), "search");
        assert_eq!(packs[1].name(), "fetch");
    }

    #[test]
    fn convert_openai_preserves_tags() {
        let func_json = serde_json::json!({
            "name": "tagged_func",
            "parameters": {
                "type": "object",
                "properties": {
                    "x": { "type": "integer" }
                },
                "required": ["x"]
            }
        });

        let opts = OpenAiConvertOptions {
            tags: vec!["openai".into(), "converted".into(), "math".into()],
            ..Default::default()
        };
        let pack = from_openai_json(&func_json, &opts).unwrap();

        let meta = pack.manifest.metadata.as_ref().unwrap();
        assert_eq!(meta.tags.len(), 3);
        assert!(meta.tags.contains(&"openai".to_string()));
        assert!(meta.tags.contains(&"math".to_string()));
    }

    #[test]
    fn convert_openai_complex_parameters() {
        let tool_json = serde_json::json!({
            "type": "function",
            "function": {
                "name": "create_record",
                "description": "Create a database record",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "table": {
                            "type": "string",
                            "description": "Table name"
                        },
                        "data": {
                            "type": "object",
                            "description": "Record data as key-value pairs"
                        },
                        "tags": {
                            "type": "array",
                            "description": "Record tags"
                        },
                        "priority": {
                            "type": "integer",
                            "description": "Priority level",
                            "default": 0
                        },
                        "active": {
                            "type": "boolean",
                            "description": "Whether the record is active",
                            "default": true
                        }
                    },
                    "required": ["table", "data"]
                }
            }
        });

        let opts = OpenAiConvertOptions::default();
        let pack = from_openai_json(&tool_json, &opts).unwrap();

        assert_eq!(pack.manifest.inputs.len(), 5);
        assert!(pack.manifest.inputs["table"].required);
        assert!(pack.manifest.inputs["data"].required);
        assert!(!pack.manifest.inputs["tags"].required);
        assert!(!pack.manifest.inputs["priority"].required);
        assert_eq!(pack.manifest.inputs["data"].field_type, "object");
        assert_eq!(pack.manifest.inputs["tags"].field_type, "array");
        assert_eq!(pack.manifest.inputs["active"].field_type, "boolean");
        assert_eq!(
            pack.manifest.inputs["priority"].default,
            Some(serde_json::json!(0))
        );
    }
}
