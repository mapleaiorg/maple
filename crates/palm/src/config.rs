//! CLI configuration

use crate::error::{CliError, CliResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliConfig {
    /// PALM daemon endpoint
    pub endpoint: Option<String>,

    /// Default platform profile
    pub default_platform: Option<String>,

    /// Default namespace
    pub default_namespace: Option<String>,

    /// Request timeout in seconds
    pub timeout_seconds: Option<u64>,
}

impl CliConfig {
    /// Load configuration from file
    pub fn load(path: Option<&str>) -> CliResult<Self> {
        let config_path = match path {
            Some(p) => PathBuf::from(p),
            None => Self::default_config_path()?,
        };

        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)?;
            let config: CliConfig =
                toml::from_str(&contents).map_err(|e| CliError::Config(e.to_string()))?;
            Ok(config)
        } else {
            Ok(CliConfig::default())
        }
    }

    /// Get the default configuration file path
    fn default_config_path() -> CliResult<PathBuf> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| CliError::Config("Cannot find config directory".into()))?;
        Ok(config_dir.join("palm").join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CliConfig::default();
        assert!(config.endpoint.is_none());
        assert!(config.default_platform.is_none());
    }

    #[test]
    fn test_load_missing_config() {
        // Should return default config when file doesn't exist
        let config = CliConfig::load(Some("/nonexistent/path/config.toml")).unwrap();
        assert!(config.endpoint.is_none());
    }
}
