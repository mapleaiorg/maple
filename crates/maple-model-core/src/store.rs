//! Local model storage.
//!
//! Manages a structured on-disk store for model weights, tokenizers,
//! and metadata. Provides indexing, listing, versioned access, and
//! usage tracking.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::metadata::ModelMetadata;

/// Errors that can occur during model store operations.
#[derive(Debug, thiserror::Error)]
pub enum ModelStoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("model not found: {0}")]
    NotFound(String),

    #[error("model version not found: {name}@{version}")]
    VersionNotFound { name: String, version: String },

    #[error("model already exists: {name}@{version}")]
    AlreadyExists { name: String, version: String },

    #[error("store error: {0}")]
    Other(String),
}

/// An entry in the model list returned by `list_models`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelListEntry {
    /// Model name.
    pub name: String,

    /// Available versions.
    pub versions: Vec<String>,

    /// Default (latest) version.
    pub default_version: String,

    /// Total size on disk in bytes across all versions.
    pub total_size_bytes: u64,

    /// Number of times this model has been used.
    pub use_count: u64,

    /// Last time this model was used.
    pub last_used: Option<DateTime<Utc>>,
}

/// On-disk index of all stored models.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelIndex {
    /// All models in the store, keyed by name.
    pub models: std::collections::HashMap<String, ModelIndexEntry>,
}

/// Index entry for a single model name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelIndexEntry {
    /// Available versions.
    pub versions: std::collections::HashMap<String, ModelVersionInfo>,

    /// Default version string.
    pub default_version: String,

    /// Total usage count across all versions.
    pub use_count: u64,

    /// Last usage timestamp.
    pub last_used: Option<DateTime<Utc>>,
}

/// Information about a specific model version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersionInfo {
    /// Path to the model directory (relative to store root).
    pub path: PathBuf,

    /// Size on disk in bytes.
    pub size_bytes: u64,

    /// When this version was stored.
    pub stored_at: DateTime<Utc>,

    /// BLAKE3 hash of the weights.
    pub weights_hash: String,
}

/// Local model store manager.
pub struct ModelStore {
    root: PathBuf,
}

impl ModelStore {
    /// Open the default model store at `~/.maple/models`.
    pub fn open() -> Result<Self, ModelStoreError> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| {
                ModelStoreError::Other("cannot determine home directory".to_string())
            })?;
        let root = PathBuf::from(home).join(".maple").join("models");
        Self::open_at(root)
    }

    /// Open a model store at a specific root path (useful for testing).
    pub fn open_at(root: PathBuf) -> Result<Self, ModelStoreError> {
        std::fs::create_dir_all(&root)?;
        let store = Self { root };

        // Ensure the index file exists
        let index_path = store.index_path();
        if !index_path.exists() {
            store.save_index(&ModelIndex::default())?;
        }

        Ok(store)
    }

    /// Store a model with its associated files.
    ///
    /// Returns the path to the stored model directory.
    pub fn store_model(
        &self,
        metadata: &ModelMetadata,
        weights_path: &Path,
        tokenizer_path: Option<&Path>,
        config_path: Option<&Path>,
    ) -> Result<PathBuf, ModelStoreError> {
        let version_str = metadata.version.to_string();
        let model_dir = self
            .root
            .join(&metadata.name)
            .join(&version_str);

        // Check for duplicates
        let mut index = self.load_index()?;
        if let Some(entry) = index.models.get(&metadata.name) {
            if entry.versions.contains_key(&version_str) {
                return Err(ModelStoreError::AlreadyExists {
                    name: metadata.name.clone(),
                    version: version_str,
                });
            }
        }

        // Create directory and copy files
        std::fs::create_dir_all(&model_dir)?;

        // Copy weights
        let weights_dest = model_dir.join(format!(
            "weights.{}",
            metadata.format.extension()
        ));
        std::fs::copy(weights_path, &weights_dest)?;

        // Copy tokenizer if provided
        if let Some(tok_path) = tokenizer_path {
            let tok_dest = model_dir.join("tokenizer.json");
            std::fs::copy(tok_path, &tok_dest)?;
        }

        // Copy config if provided
        if let Some(cfg_path) = config_path {
            let cfg_dest = model_dir.join("config.json");
            std::fs::copy(cfg_path, &cfg_dest)?;
        }

        // Write metadata
        let meta_json = serde_json::to_string_pretty(metadata)?;
        std::fs::write(model_dir.join("metadata.json"), meta_json)?;

        // Update index
        let version_info = ModelVersionInfo {
            path: model_dir.strip_prefix(&self.root).unwrap_or(&model_dir).to_path_buf(),
            size_bytes: metadata.size_bytes,
            stored_at: Utc::now(),
            weights_hash: metadata.weights_hash.clone(),
        };

        let model_entry = index
            .models
            .entry(metadata.name.clone())
            .or_insert_with(|| ModelIndexEntry {
                versions: std::collections::HashMap::new(),
                default_version: version_str.clone(),
                use_count: 0,
                last_used: None,
            });

        model_entry
            .versions
            .insert(version_str.clone(), version_info);

        // Update default to latest version
        let mut all_versions: Vec<semver::Version> = model_entry
            .versions
            .keys()
            .filter_map(|v| semver::Version::parse(v).ok())
            .collect();
        all_versions.sort();
        if let Some(latest) = all_versions.last() {
            model_entry.default_version = latest.to_string();
        }

        self.save_index(&index)?;

        Ok(model_dir)
    }

    /// Get the on-disk path for a model.
    ///
    /// If `version` is `None`, returns the default (latest) version.
    pub fn get_model_path(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<PathBuf, ModelStoreError> {
        let index = self.load_index()?;
        let entry = index
            .models
            .get(name)
            .ok_or_else(|| ModelStoreError::NotFound(name.to_string()))?;

        let ver = version.unwrap_or(&entry.default_version);
        let version_info = entry.versions.get(ver).ok_or_else(|| {
            ModelStoreError::VersionNotFound {
                name: name.to_string(),
                version: ver.to_string(),
            }
        })?;

        Ok(self.root.join(&version_info.path))
    }

    /// List all stored models.
    pub fn list_models(&self) -> Result<Vec<ModelListEntry>, ModelStoreError> {
        let index = self.load_index()?;
        let mut entries = Vec::new();

        for (name, model_entry) in &index.models {
            let versions: Vec<String> = model_entry.versions.keys().cloned().collect();
            let total_size: u64 =
                model_entry.versions.values().map(|v| v.size_bytes).sum();

            entries.push(ModelListEntry {
                name: name.clone(),
                versions,
                default_version: model_entry.default_version.clone(),
                total_size_bytes: total_size,
                use_count: model_entry.use_count,
                last_used: model_entry.last_used,
            });
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    /// Remove a model from the store.
    ///
    /// If `version` is `None`, removes all versions.
    pub fn remove_model(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<(), ModelStoreError> {
        let mut index = self.load_index()?;
        let entry = index
            .models
            .get_mut(name)
            .ok_or_else(|| ModelStoreError::NotFound(name.to_string()))?;

        match version {
            Some(ver) => {
                let version_info =
                    entry.versions.remove(ver).ok_or_else(|| {
                        ModelStoreError::VersionNotFound {
                            name: name.to_string(),
                            version: ver.to_string(),
                        }
                    })?;

                // Remove the version directory
                let dir = self.root.join(&version_info.path);
                if dir.exists() {
                    std::fs::remove_dir_all(&dir)?;
                }

                // If no versions left, remove the entire model entry
                if entry.versions.is_empty() {
                    index.models.remove(name);
                    // Also remove the model directory
                    let model_dir = self.root.join(name);
                    if model_dir.exists() {
                        std::fs::remove_dir_all(&model_dir)?;
                    }
                } else {
                    // Update default version
                    let mut all_versions: Vec<semver::Version> = entry
                        .versions
                        .keys()
                        .filter_map(|v| semver::Version::parse(v).ok())
                        .collect();
                    all_versions.sort();
                    if let Some(latest) = all_versions.last() {
                        entry.default_version = latest.to_string();
                    }
                }
            }
            None => {
                // Remove all versions
                let model_dir = self.root.join(name);
                if model_dir.exists() {
                    std::fs::remove_dir_all(&model_dir)?;
                }
                index.models.remove(name);
            }
        }

        self.save_index(&index)?;
        Ok(())
    }

    /// Record a usage event for a model.
    pub fn record_use(
        &self,
        name: &str,
        version: &str,
    ) -> Result<(), ModelStoreError> {
        let mut index = self.load_index()?;
        let entry = index
            .models
            .get_mut(name)
            .ok_or_else(|| ModelStoreError::NotFound(name.to_string()))?;

        // Verify the version exists
        if !entry.versions.contains_key(version) {
            return Err(ModelStoreError::VersionNotFound {
                name: name.to_string(),
                version: version.to_string(),
            });
        }

        entry.use_count += 1;
        entry.last_used = Some(Utc::now());

        self.save_index(&index)?;
        Ok(())
    }

    // --- Internal helpers ---

    fn index_path(&self) -> PathBuf {
        self.root.join("index.json")
    }

    fn load_index(&self) -> Result<ModelIndex, ModelStoreError> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(ModelIndex::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let index: ModelIndex = serde_json::from_str(&content)?;
        Ok(index)
    }

    fn save_index(&self, index: &ModelIndex) -> Result<(), ModelStoreError> {
        let content = serde_json::to_string_pretty(index)?;
        std::fs::write(self.index_path(), content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    /// Create a sample ModelMetadata for testing.
    fn test_metadata(name: &str, version: &str) -> ModelMetadata {
        ModelMetadata {
            name: name.to_string(),
            version: semver::Version::parse(version).unwrap(),
            family: "llama".to_string(),
            parameters: "8B".to_string(),
            quantization: None,
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
                special_tokens: HashMap::new(),
            },
            context: ContextInfo {
                max_context: 8192,
                default_context: 4096,
                rope_scaling: None,
            },
            capabilities: vec![ModelCapability::Chat],
            license: ModelLicense {
                spdx: None,
                name: "MIT".to_string(),
                url: None,
                commercial_use: true,
                restrictions: vec![],
            },
            defaults: InferenceDefaults {
                temperature: 0.7,
                top_p: 0.9,
                top_k: None,
                repeat_penalty: None,
                max_tokens: None,
                stop_sequences: vec![],
            },
            template: None,
            format: ModelFormat::Gguf,
            size_bytes: 1000,
            weights_hash: "blake3:testhash".to_string(),
        }
    }

    /// Create a temporary weights file for testing.
    fn create_temp_weights(dir: &Path) -> PathBuf {
        let weights_path = dir.join("test_weights.gguf");
        std::fs::write(&weights_path, b"fake-model-weights-data").unwrap();
        weights_path
    }

    #[test]
    fn test_store_list_get_remove() {
        let tmp = TempDir::new().unwrap();
        let store_dir = tmp.path().join("models");
        let store = ModelStore::open_at(store_dir).unwrap();

        let meta = test_metadata("test-model", "1.0.0");
        let weights = create_temp_weights(tmp.path());

        // Store
        let model_path = store
            .store_model(&meta, &weights, None, None)
            .unwrap();
        assert!(model_path.exists());
        assert!(model_path.join("metadata.json").exists());
        assert!(model_path.join("weights.gguf").exists());

        // List
        let models = store.list_models().unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "test-model");
        assert_eq!(models[0].versions, vec!["1.0.0"]);
        assert_eq!(models[0].default_version, "1.0.0");

        // Get path
        let path = store.get_model_path("test-model", None).unwrap();
        assert_eq!(path, model_path);

        let path_explicit = store
            .get_model_path("test-model", Some("1.0.0"))
            .unwrap();
        assert_eq!(path_explicit, model_path);

        // Remove
        store.remove_model("test-model", None).unwrap();
        let models = store.list_models().unwrap();
        assert!(models.is_empty());
        assert!(!model_path.exists());
    }

    #[test]
    fn test_multiple_versions_coexist() {
        let tmp = TempDir::new().unwrap();
        let store_dir = tmp.path().join("models");
        let store = ModelStore::open_at(store_dir).unwrap();

        let weights = create_temp_weights(tmp.path());

        let meta_v1 = test_metadata("multi-model", "1.0.0");
        let meta_v2 = test_metadata("multi-model", "2.0.0");
        let meta_v3 = test_metadata("multi-model", "1.5.0");

        let path_v1 = store
            .store_model(&meta_v1, &weights, None, None)
            .unwrap();
        let path_v2 = store
            .store_model(&meta_v2, &weights, None, None)
            .unwrap();
        let path_v3 = store
            .store_model(&meta_v3, &weights, None, None)
            .unwrap();

        // All three should exist
        assert!(path_v1.exists());
        assert!(path_v2.exists());
        assert!(path_v3.exists());

        // List should show one model with three versions
        let models = store.list_models().unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].versions.len(), 3);

        // Paths should be different
        assert_ne!(path_v1, path_v2);
        assert_ne!(path_v1, path_v3);

        // Get specific versions
        let got_v1 = store
            .get_model_path("multi-model", Some("1.0.0"))
            .unwrap();
        assert_eq!(got_v1, path_v1);

        let got_v2 = store
            .get_model_path("multi-model", Some("2.0.0"))
            .unwrap();
        assert_eq!(got_v2, path_v2);

        // Remove a single version
        store
            .remove_model("multi-model", Some("1.5.0"))
            .unwrap();
        let models = store.list_models().unwrap();
        assert_eq!(models[0].versions.len(), 2);
        assert!(!path_v3.exists());

        // Remaining versions should still work
        assert!(store
            .get_model_path("multi-model", Some("1.0.0"))
            .is_ok());
        assert!(store
            .get_model_path("multi-model", Some("2.0.0"))
            .is_ok());
    }

    #[test]
    fn test_record_use_increments_counter() {
        let tmp = TempDir::new().unwrap();
        let store_dir = tmp.path().join("models");
        let store = ModelStore::open_at(store_dir).unwrap();

        let meta = test_metadata("use-model", "1.0.0");
        let weights = create_temp_weights(tmp.path());
        store.store_model(&meta, &weights, None, None).unwrap();

        // Initial use count should be 0
        let models = store.list_models().unwrap();
        assert_eq!(models[0].use_count, 0);
        assert!(models[0].last_used.is_none());

        // Record several uses
        store.record_use("use-model", "1.0.0").unwrap();
        store.record_use("use-model", "1.0.0").unwrap();
        store.record_use("use-model", "1.0.0").unwrap();

        let models = store.list_models().unwrap();
        assert_eq!(models[0].use_count, 3);
        assert!(models[0].last_used.is_some());
    }

    #[test]
    fn test_default_version_selection() {
        let tmp = TempDir::new().unwrap();
        let store_dir = tmp.path().join("models");
        let store = ModelStore::open_at(store_dir).unwrap();

        let weights = create_temp_weights(tmp.path());

        // Store v1.0.0 first -> it becomes default
        let meta_v1 = test_metadata("default-model", "1.0.0");
        store.store_model(&meta_v1, &weights, None, None).unwrap();

        let models = store.list_models().unwrap();
        assert_eq!(models[0].default_version, "1.0.0");

        // Store v2.0.0 -> it becomes default (higher)
        let meta_v2 = test_metadata("default-model", "2.0.0");
        let path_v2 = store
            .store_model(&meta_v2, &weights, None, None)
            .unwrap();

        let models = store.list_models().unwrap();
        assert_eq!(models[0].default_version, "2.0.0");

        // Get with None should return v2.0.0
        let default_path = store
            .get_model_path("default-model", None)
            .unwrap();
        assert_eq!(default_path, path_v2);

        // Store v1.5.0 -> default should still be v2.0.0
        let meta_v15 = test_metadata("default-model", "1.5.0");
        store.store_model(&meta_v15, &weights, None, None).unwrap();

        let models = store.list_models().unwrap();
        assert_eq!(models[0].default_version, "2.0.0");

        // Remove v2.0.0 -> default should fall back to v1.5.0
        store
            .remove_model("default-model", Some("2.0.0"))
            .unwrap();

        let models = store.list_models().unwrap();
        assert_eq!(models[0].default_version, "1.5.0");
    }
}
