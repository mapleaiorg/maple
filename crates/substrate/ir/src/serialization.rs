//! WLIR module serialization.
//!
//! Supports two formats:
//! - **Text**: Human-readable pseudo-assembly for debugging/inspection
//! - **Binary**: Compact binary format for storage and transmission
//!
//! Both formats include magic headers for format identification.
//! The `WlirSerializer` trait is implemented by `SimulatedSerializer`
//! for deterministic testing. Uses serde_json internally.

use crate::error::{WlirError, WlirResult};
use crate::module::WlirModule;

// ── Serialization Format ─────────────────────────────────────────────

/// Format for WLIR module serialization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WlirFormat {
    /// Human-readable text format (pseudo-assembly).
    Text,
    /// Compact binary format for storage/transmission.
    Binary,
}

impl std::fmt::Display for WlirFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Binary => write!(f, "binary"),
        }
    }
}

// ── Magic Headers ────────────────────────────────────────────────────

/// Magic header for text-format WLIR files.
pub const WLIR_TEXT_MAGIC: &str = "WLIR-TEXT-V1";

/// Magic header for binary-format WLIR files.
pub const WLIR_BINARY_MAGIC: &[u8] = b"WLIR\x00\x01";

// ── Serialization Result ─────────────────────────────────────────────

/// Result of serializing a WLIR module.
#[derive(Clone, Debug)]
pub struct SerializedModule {
    /// The format used for serialization.
    pub format: WlirFormat,
    /// Serialized data (text or binary).
    pub data: Vec<u8>,
    /// Size in bytes.
    pub size_bytes: usize,
    /// Module name (for identification).
    pub module_name: String,
    /// Module version.
    pub module_version: String,
}

impl SerializedModule {
    /// Whether this is a text-format serialization.
    pub fn is_text(&self) -> bool {
        self.format == WlirFormat::Text
    }

    /// Whether this is a binary-format serialization.
    pub fn is_binary(&self) -> bool {
        self.format == WlirFormat::Binary
    }

    /// Get the data as a UTF-8 string (only valid for text format).
    pub fn as_text(&self) -> Option<&str> {
        if self.is_text() {
            std::str::from_utf8(&self.data).ok()
        } else {
            None
        }
    }
}

// ── Serializer Trait ─────────────────────────────────────────────────

/// Trait for serializing and deserializing WLIR modules.
pub trait WlirSerializer: Send + Sync {
    /// Serialize a WLIR module to the specified format.
    fn serialize(&self, module: &WlirModule, format: &WlirFormat) -> WlirResult<SerializedModule>;

    /// Deserialize a WLIR module from serialized data.
    fn deserialize(&self, data: &SerializedModule) -> WlirResult<WlirModule>;

    /// Name of this serializer implementation.
    fn name(&self) -> &str;
}

// ── Simulated Serializer ─────────────────────────────────────────────

/// Simulated serializer for deterministic testing.
///
/// Uses serde_json internally with magic-number headers prepended.
/// Text format produces readable JSON, binary format produces compact JSON with binary header.
pub struct SimulatedSerializer;

impl SimulatedSerializer {
    /// Create a new simulated serializer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimulatedSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl WlirSerializer for SimulatedSerializer {
    fn serialize(&self, module: &WlirModule, format: &WlirFormat) -> WlirResult<SerializedModule> {
        match format {
            WlirFormat::Text => {
                let json = serde_json::to_string_pretty(module)
                    .map_err(|e| WlirError::SerializationFailed(e.to_string()))?;

                let text = format!("{}\n{}", WLIR_TEXT_MAGIC, json);
                let data = text.into_bytes();
                let size = data.len();

                Ok(SerializedModule {
                    format: WlirFormat::Text,
                    data,
                    size_bytes: size,
                    module_name: module.name.clone(),
                    module_version: module.version.clone(),
                })
            }
            WlirFormat::Binary => {
                let json = serde_json::to_vec(module)
                    .map_err(|e| WlirError::SerializationFailed(e.to_string()))?;

                let mut data = Vec::with_capacity(WLIR_BINARY_MAGIC.len() + json.len());
                data.extend_from_slice(WLIR_BINARY_MAGIC);
                data.extend_from_slice(&json);
                let size = data.len();

                Ok(SerializedModule {
                    format: WlirFormat::Binary,
                    data,
                    size_bytes: size,
                    module_name: module.name.clone(),
                    module_version: module.version.clone(),
                })
            }
        }
    }

    fn deserialize(&self, data: &SerializedModule) -> WlirResult<WlirModule> {
        match data.format {
            WlirFormat::Text => {
                let text = std::str::from_utf8(&data.data)
                    .map_err(|e| WlirError::DeserializationFailed(e.to_string()))?;

                // Strip magic header
                let json = text
                    .strip_prefix(WLIR_TEXT_MAGIC)
                    .ok_or_else(|| {
                        WlirError::DeserializationFailed("missing text magic header".into())
                    })?
                    .trim_start();

                serde_json::from_str(json)
                    .map_err(|e| WlirError::DeserializationFailed(e.to_string()))
            }
            WlirFormat::Binary => {
                // Strip magic header
                if data.data.len() < WLIR_BINARY_MAGIC.len() {
                    return Err(WlirError::DeserializationFailed(
                        "data too short for binary format".into(),
                    ));
                }
                if &data.data[..WLIR_BINARY_MAGIC.len()] != WLIR_BINARY_MAGIC {
                    return Err(WlirError::DeserializationFailed(
                        "missing binary magic header".into(),
                    ));
                }

                let json = &data.data[WLIR_BINARY_MAGIC.len()..];
                serde_json::from_slice(json)
                    .map_err(|e| WlirError::DeserializationFailed(e.to_string()))
            }
        }
    }

    fn name(&self) -> &str {
        "simulated-serializer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instructions::WlirInstruction;
    use crate::module::{WlirFunction, WlirModule};
    use crate::types::WlirType;

    fn make_module() -> WlirModule {
        let mut module = WlirModule::new("test-module", "1.0.0");
        let mut f = WlirFunction::new("main", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::Nop);
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);
        module
    }

    #[test]
    fn serialize_text_format() {
        let serializer = SimulatedSerializer::new();
        let module = make_module();
        let result = serializer.serialize(&module, &WlirFormat::Text).unwrap();
        assert!(result.is_text());
        assert!(!result.is_binary());
        let text = result.as_text().unwrap();
        assert!(text.starts_with(WLIR_TEXT_MAGIC));
        assert!(text.contains("test-module"));
    }

    #[test]
    fn serialize_binary_format() {
        let serializer = SimulatedSerializer::new();
        let module = make_module();
        let result = serializer.serialize(&module, &WlirFormat::Binary).unwrap();
        assert!(result.is_binary());
        assert!(!result.is_text());
        assert!(result.data.starts_with(WLIR_BINARY_MAGIC));
    }

    #[test]
    fn text_roundtrip() {
        let serializer = SimulatedSerializer::new();
        let module = make_module();
        let serialized = serializer.serialize(&module, &WlirFormat::Text).unwrap();
        let deserialized = serializer.deserialize(&serialized).unwrap();
        assert_eq!(deserialized.name, module.name);
        assert_eq!(deserialized.version, module.version);
        assert_eq!(deserialized.functions.len(), module.functions.len());
    }

    #[test]
    fn binary_roundtrip() {
        let serializer = SimulatedSerializer::new();
        let module = make_module();
        let serialized = serializer.serialize(&module, &WlirFormat::Binary).unwrap();
        let deserialized = serializer.deserialize(&serialized).unwrap();
        assert_eq!(deserialized.name, module.name);
        assert_eq!(deserialized.version, module.version);
        assert_eq!(deserialized.functions.len(), module.functions.len());
    }

    #[test]
    fn binary_smaller_than_text() {
        let serializer = SimulatedSerializer::new();
        let module = make_module();
        let text = serializer.serialize(&module, &WlirFormat::Text).unwrap();
        let binary = serializer.serialize(&module, &WlirFormat::Binary).unwrap();
        assert!(binary.size_bytes < text.size_bytes);
    }

    #[test]
    fn deserialize_invalid_text_header() {
        let serializer = SimulatedSerializer::new();
        let invalid = SerializedModule {
            format: WlirFormat::Text,
            data: b"INVALID-HEADER\n{}".to_vec(),
            size_bytes: 17,
            module_name: "test".into(),
            module_version: "1.0".into(),
        };
        let result = serializer.deserialize(&invalid);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("magic header"));
    }

    #[test]
    fn deserialize_invalid_binary_header() {
        let serializer = SimulatedSerializer::new();
        let invalid = SerializedModule {
            format: WlirFormat::Binary,
            data: b"INVALID".to_vec(),
            size_bytes: 7,
            module_name: "test".into(),
            module_version: "1.0".into(),
        };
        let result = serializer.deserialize(&invalid);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("magic header"));
    }

    #[test]
    fn deserialize_too_short_binary() {
        let serializer = SimulatedSerializer::new();
        let invalid = SerializedModule {
            format: WlirFormat::Binary,
            data: b"WL".to_vec(),
            size_bytes: 2,
            module_name: "test".into(),
            module_version: "1.0".into(),
        };
        let result = serializer.deserialize(&invalid);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn format_display() {
        assert_eq!(WlirFormat::Text.to_string(), "text");
        assert_eq!(WlirFormat::Binary.to_string(), "binary");
    }

    #[test]
    fn serializer_name() {
        let serializer = SimulatedSerializer::new();
        assert_eq!(serializer.name(), "simulated-serializer");
    }
}
