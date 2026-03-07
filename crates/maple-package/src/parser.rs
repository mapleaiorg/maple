use crate::manifest::MapleManifest;
use std::path::Path;

/// Parse a Maplefile from disk. Supports YAML and JSON.
pub fn parse_maplefile(path: &Path) -> Result<MapleManifest, ParseError> {
    let content =
        std::fs::read_to_string(path).map_err(|e| ParseError::Io(path.to_path_buf(), e))?;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("yaml");

    match ext {
        "json" => serde_json::from_str(&content).map_err(ParseError::Json),
        "yaml" | "yml" | _ => serde_yaml::from_str(&content).map_err(ParseError::Yaml),
    }
}

/// Parse a Maplefile from a string (for testing and embedded use)
pub fn parse_maplefile_str(
    content: &str,
    format: ManifestFormat,
) -> Result<MapleManifest, ParseError> {
    match format {
        ManifestFormat::Yaml => serde_yaml::from_str(content).map_err(ParseError::Yaml),
        ManifestFormat::Json => serde_json::from_str(content).map_err(ParseError::Json),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestFormat {
    Yaml,
    Json,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Failed to read {0}: {1}")]
    Io(std::path::PathBuf, std::io::Error),
    #[error("Invalid YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
}
