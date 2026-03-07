//! Model metadata types for MAPLE Model Manager.
//!
//! Defines the complete metadata schema for models including architecture,
//! tokenizer configuration, inference defaults, and capability declarations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete metadata describing a model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelMetadata {
    /// Human-readable model name (e.g. "llama-3.1-8b-instruct").
    pub name: String,

    /// Semantic version of the model.
    pub version: semver::Version,

    /// Model family (e.g. "llama", "mistral", "qwen").
    pub family: String,

    /// Parameter count as a human-readable string (e.g. "8B", "70B").
    pub parameters: String,

    /// Quantization details, if the model is quantized.
    pub quantization: Option<QuantizationInfo>,

    /// Neural-network architecture description.
    pub architecture: ModelArchitecture,

    /// Tokenizer configuration.
    pub tokenizer: TokenizerInfo,

    /// Context window information.
    pub context: ContextInfo,

    /// Declared model capabilities.
    pub capabilities: Vec<ModelCapability>,

    /// License information.
    pub license: ModelLicense,

    /// Default inference parameters.
    pub defaults: InferenceDefaults,

    /// Optional prompt template.
    pub template: Option<PromptTemplate>,

    /// On-disk weight format.
    pub format: ModelFormat,

    /// Total weight-file size in bytes.
    pub size_bytes: u64,

    /// BLAKE3 hash of the weights file.
    pub weights_hash: String,
}

/// Quantization details for a model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuantizationInfo {
    /// Quantization method (e.g. "GPTQ", "AWQ", "GGUF").
    pub method: String,

    /// Quantization level (e.g. "Q4_K_M", "Q5_K_S").
    pub level: String,

    /// Average bits per weight.
    pub bits_per_weight: f32,
}

/// Neural-network architecture descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelArchitecture {
    /// Architecture type (e.g. "transformer", "mamba", "rwkv").
    pub arch_type: String,

    /// Number of transformer/recurrent layers.
    pub num_layers: u32,

    /// Hidden dimension size.
    pub hidden_dim: u32,

    /// Number of attention heads.
    pub num_heads: u32,

    /// Number of key-value heads (for GQA/MQA).
    pub num_kv_heads: Option<u32>,

    /// Vocabulary size.
    pub vocab_size: u32,

    /// Embedding dimension (if different from hidden_dim).
    pub embed_dim: Option<u32>,
}

/// Tokenizer configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenizerInfo {
    /// Tokenizer type (e.g. "BPE", "SentencePiece", "Unigram").
    pub tokenizer_type: String,

    /// Vocabulary size.
    pub vocab_size: u32,

    /// Special tokens mapping (e.g. "bos" -> "<s>", "eos" -> "</s>").
    pub special_tokens: HashMap<String, String>,
}

/// Context window information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextInfo {
    /// Maximum supported context length.
    pub max_context: u32,

    /// Default context length to use.
    pub default_context: u32,

    /// RoPE scaling method, if any (e.g. "linear", "dynamic").
    pub rope_scaling: Option<String>,
}

/// A capability that a model can declare.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ModelCapability {
    Chat,
    Completion,
    ToolCalling,
    JsonMode,
    Vision,
    Audio,
    CodeGeneration,
    Embedding,
    FunctionCalling,
    StructuredOutput,
}

/// License information for a model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelLicense {
    /// SPDX identifier, if applicable.
    pub spdx: Option<String>,

    /// Human-readable license name.
    pub name: String,

    /// URL to the full license text.
    pub url: Option<String>,

    /// Whether commercial use is permitted.
    pub commercial_use: bool,

    /// List of restrictions or conditions.
    pub restrictions: Vec<String>,
}

/// Default inference parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InferenceDefaults {
    /// Sampling temperature.
    pub temperature: f32,

    /// Top-p (nucleus) sampling threshold.
    pub top_p: f32,

    /// Top-k sampling value.
    pub top_k: Option<u32>,

    /// Repeat penalty factor.
    pub repeat_penalty: Option<f32>,

    /// Maximum number of tokens to generate.
    pub max_tokens: Option<u32>,

    /// Stop sequences.
    pub stop_sequences: Vec<String>,
}

/// Prompt template configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptTemplate {
    /// Template format identifier (e.g. "chatml", "llama2", "alpaca").
    pub format: String,

    /// System message template.
    pub system_template: Option<String>,

    /// User message template.
    pub user_template: String,

    /// Assistant message template.
    pub assistant_template: String,

    /// Tool/function call template.
    pub tool_template: Option<String>,
}

/// On-disk weight file format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ModelFormat {
    Gguf,
    Safetensors,
    Ggml,
    Pytorch,
    Onnx,
}

impl ModelFormat {
    /// Returns the conventional file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            ModelFormat::Gguf => "gguf",
            ModelFormat::Safetensors => "safetensors",
            ModelFormat::Ggml => "ggml",
            ModelFormat::Pytorch => "pt",
            ModelFormat::Onnx => "onnx",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_metadata() -> ModelMetadata {
        let mut special_tokens = HashMap::new();
        special_tokens.insert("bos".to_string(), "<s>".to_string());
        special_tokens.insert("eos".to_string(), "</s>".to_string());

        ModelMetadata {
            name: "llama-3.1-8b-instruct".to_string(),
            version: semver::Version::new(1, 0, 0),
            family: "llama".to_string(),
            parameters: "8B".to_string(),
            quantization: Some(QuantizationInfo {
                method: "GGUF".to_string(),
                level: "Q4_K_M".to_string(),
                bits_per_weight: 4.5,
            }),
            architecture: ModelArchitecture {
                arch_type: "transformer".to_string(),
                num_layers: 32,
                hidden_dim: 4096,
                num_heads: 32,
                num_kv_heads: Some(8),
                vocab_size: 128256,
                embed_dim: None,
            },
            tokenizer: TokenizerInfo {
                tokenizer_type: "BPE".to_string(),
                vocab_size: 128256,
                special_tokens,
            },
            context: ContextInfo {
                max_context: 131072,
                default_context: 8192,
                rope_scaling: Some("dynamic".to_string()),
            },
            capabilities: vec![
                ModelCapability::Chat,
                ModelCapability::Completion,
                ModelCapability::ToolCalling,
                ModelCapability::CodeGeneration,
            ],
            license: ModelLicense {
                spdx: Some("Llama-3.1".to_string()),
                name: "Llama 3.1 Community License".to_string(),
                url: Some("https://llama.meta.com/llama3_1/license/".to_string()),
                commercial_use: true,
                restrictions: vec!["700M MAU limit".to_string()],
            },
            defaults: InferenceDefaults {
                temperature: 0.7,
                top_p: 0.9,
                top_k: Some(40),
                repeat_penalty: Some(1.1),
                max_tokens: Some(2048),
                stop_sequences: vec!["<|eot_id|>".to_string()],
            },
            template: Some(PromptTemplate {
                format: "llama3".to_string(),
                system_template: Some("<|begin_of_text|><|start_header_id|>system<|end_header_id|>\n\n{{system}}<|eot_id|>".to_string()),
                user_template: "<|start_header_id|>user<|end_header_id|>\n\n{{content}}<|eot_id|>".to_string(),
                assistant_template: "<|start_header_id|>assistant<|end_header_id|>\n\n{{content}}<|eot_id|>".to_string(),
                tool_template: None,
            }),
            format: ModelFormat::Gguf,
            size_bytes: 4_370_000_000,
            weights_hash: "blake3:abc123def456".to_string(),
        }
    }

    #[test]
    fn test_metadata_serde_roundtrip_json() {
        let metadata = sample_metadata();
        let json = serde_json::to_string_pretty(&metadata).expect("serialize");
        let deserialized: ModelMetadata =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(metadata, deserialized);
    }

    #[test]
    fn test_model_capability_serde_kebab_case() {
        let capabilities = vec![
            ModelCapability::Chat,
            ModelCapability::ToolCalling,
            ModelCapability::JsonMode,
            ModelCapability::CodeGeneration,
            ModelCapability::FunctionCalling,
            ModelCapability::StructuredOutput,
        ];
        let json = serde_json::to_string(&capabilities).expect("serialize");
        assert!(json.contains("\"chat\""), "expected kebab-case 'chat'");
        assert!(
            json.contains("\"tool-calling\""),
            "expected kebab-case 'tool-calling'"
        );
        assert!(
            json.contains("\"json-mode\""),
            "expected kebab-case 'json-mode'"
        );
        assert!(
            json.contains("\"code-generation\""),
            "expected kebab-case 'code-generation'"
        );
        assert!(
            json.contains("\"function-calling\""),
            "expected kebab-case 'function-calling'"
        );
        assert!(
            json.contains("\"structured-output\""),
            "expected kebab-case 'structured-output'"
        );

        // Roundtrip
        let deserialized: Vec<ModelCapability> =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(capabilities, deserialized);
    }

    #[test]
    fn test_model_format_extension() {
        assert_eq!(ModelFormat::Gguf.extension(), "gguf");
        assert_eq!(ModelFormat::Safetensors.extension(), "safetensors");
        assert_eq!(ModelFormat::Ggml.extension(), "ggml");
        assert_eq!(ModelFormat::Pytorch.extension(), "pt");
        assert_eq!(ModelFormat::Onnx.extension(), "onnx");
    }
}
