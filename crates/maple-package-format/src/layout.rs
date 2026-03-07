/// A MAPLE package is stored as an OCI artifact with these layers:
///
/// ## Agent/Skill/Contract Package Layout
///
/// | Layer | Directory     | Description                                          |
/// |-------|---------------|------------------------------------------------------|
/// | 0     | manifest.json | MapleManifest serialized                             |
/// | 1     | prompts/      | System prompts, templates                            |
/// | 2     | skills/       | Skill dependency manifests (references, not code)    |
/// | 3     | contracts/    | Contract definitions in canonical form               |
/// | 4     | memory/       | Schema definitions, migrations                       |
/// | 5     | eval/         | Evaluation suites, test vectors                      |
/// | 6     | static/       | Static assets, UI components                         |
/// | 7     | provenance/   | Build provenance, signatures, SBOM                   |
///
/// ## Model Package Layout
///
/// | Layer | Directory   | Description                                            |
/// |-------|-------------|--------------------------------------------------------|
/// | 0     | manifest.json | MapleManifest                                        |
/// | 1     | model/        | GGUF/safetensors shards                              |
/// | 2     | tokenizer/    | Tokenizer files                                      |
/// | 3     | config/       | Model config, templates, sampling defaults           |
/// | 4     | provenance/   | Signatures, SBOM, origin attestation                 |
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use flate2::write::GzEncoder;
use flate2::Compression;
use tracing::{debug, info};

use crate::content_hash::LayerDigest;
use crate::PackageFormatError;

/// Media type constants for OCI artifact registration
pub const MAPLE_MANIFEST_MEDIA_TYPE: &str = "application/vnd.maple.manifest.v1+json";
pub const MAPLE_CONFIG_MEDIA_TYPE: &str = "application/vnd.maple.config.v1+json";
pub const MAPLE_AGENT_LAYER_MEDIA_TYPE: &str =
    "application/vnd.maple.agent.layer.v1.tar+gzip";
pub const MAPLE_SKILL_LAYER_MEDIA_TYPE: &str =
    "application/vnd.maple.skill.layer.v1.tar+gzip";
pub const MAPLE_CONTRACT_LAYER_MEDIA_TYPE: &str =
    "application/vnd.maple.contract.layer.v1.tar+gzip";
pub const MAPLE_MODEL_LAYER_MEDIA_TYPE: &str =
    "application/vnd.maple.model.layer.v1.tar+gzip";
pub const MAPLE_EVAL_LAYER_MEDIA_TYPE: &str =
    "application/vnd.maple.eval.layer.v1.tar+gzip";
pub const MAPLE_PROVENANCE_LAYER_MEDIA_TYPE: &str =
    "application/vnd.maple.provenance.layer.v1.tar+gzip";
pub const MAPLE_PROMPTS_LAYER_MEDIA_TYPE: &str =
    "application/vnd.maple.prompts.layer.v1.tar+gzip";
pub const MAPLE_MEMORY_LAYER_MEDIA_TYPE: &str =
    "application/vnd.maple.memory.layer.v1.tar+gzip";
pub const MAPLE_STATIC_LAYER_MEDIA_TYPE: &str =
    "application/vnd.maple.static.layer.v1.tar+gzip";
pub const MAPLE_TOKENIZER_LAYER_MEDIA_TYPE: &str =
    "application/vnd.maple.tokenizer.layer.v1.tar+gzip";
pub const MAPLE_MODEL_CONFIG_LAYER_MEDIA_TYPE: &str =
    "application/vnd.maple.model-config.layer.v1.tar+gzip";

/// Describes a single layer in the OCI artifact
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LayerDescriptor {
    /// OCI media type for this layer
    #[serde(rename = "mediaType")]
    pub media_type: String,
    /// Content-addressed digest
    pub digest: String,
    /// Layer size in bytes (compressed)
    pub size: u64,
    /// Annotations (OCI spec)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub annotations: HashMap<String, String>,
}

/// The complete OCI manifest for a MAPLE package.
///
/// Follows OCI Image Manifest Spec v2:
/// https://github.com/opencontainers/image-spec/blob/main/manifest.md
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OciManifest {
    /// Schema version (always 2)
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    /// Manifest media type
    #[serde(rename = "mediaType")]
    pub media_type: String,
    /// Config blob descriptor
    pub config: LayerDescriptor,
    /// Ordered list of layer descriptors
    pub layers: Vec<LayerDescriptor>,
    /// Annotations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub annotations: HashMap<String, String>,
}

/// A built layer ready for OCI push
#[derive(Debug)]
pub struct BuiltLayer {
    pub descriptor: LayerDescriptor,
    /// Path to the compressed tar file
    pub path: PathBuf,
    /// BLAKE3 digest for internal use
    pub blake3_digest: LayerDigest,
}

/// Build the OCI layer structure from a package directory.
///
/// Ensures deterministic builds via:
/// - Sorted directory entries
/// - Zeroed timestamps (Unix epoch)
/// - Zeroed uid/gid
/// - Consistent compression level
pub struct PackageBuilder {
    work_dir: PathBuf,
    blobs_dir: PathBuf,
    layers: Vec<BuiltLayer>,
}

impl PackageBuilder {
    /// Create a new package builder.
    ///
    /// `work_dir` is a temporary directory for intermediate artifacts.
    pub fn new(work_dir: &Path) -> Result<Self, PackageFormatError> {
        let work_dir = work_dir.to_path_buf();
        let blobs_dir = work_dir.join("blobs");
        std::fs::create_dir_all(&blobs_dir)?;
        Ok(Self {
            work_dir,
            blobs_dir,
            layers: Vec::new(),
        })
    }

    /// Add a directory as a compressed, deterministic tar layer.
    ///
    /// The tar is built with:
    /// - Entries sorted alphabetically for reproducibility
    /// - All timestamps set to Unix epoch (0)
    /// - All uid/gid set to 0
    /// - Gzip compression at level 6 (balance speed/size)
    pub fn add_layer(
        &mut self,
        source_dir: &Path,
        media_type: &str,
        annotations: HashMap<String, String>,
    ) -> Result<&LayerDescriptor, PackageFormatError> {
        if !source_dir.is_dir() {
            return Err(PackageFormatError::NotADirectory(
                source_dir.to_path_buf(),
            ));
        }

        // Collect all entries and sort for determinism
        let mut entries: Vec<PathBuf> = Vec::new();
        collect_entries_sorted(source_dir, &mut entries)?;

        // Create deterministic tar + gzip
        let tar_gz_path = self.blobs_dir.join(format!("layer-{}.tar.gz", self.layers.len()));
        {
            let file = std::fs::File::create(&tar_gz_path)?;
            let encoder = GzEncoder::new(file, Compression::new(6));
            let mut tar_builder = tar::Builder::new(encoder);

            for entry_path in &entries {
                let relative = entry_path
                    .strip_prefix(source_dir)
                    .map_err(|e| PackageFormatError::Tar(e.to_string()))?;

                if entry_path.is_file() {
                    let mut header = tar::Header::new_gnu();
                    let file_data = std::fs::read(entry_path)?;
                    header.set_size(file_data.len() as u64);
                    header.set_mode(0o644);
                    header.set_uid(0);
                    header.set_gid(0);
                    header.set_mtime(0); // Unix epoch for reproducibility
                    header.set_cksum();

                    tar_builder
                        .append_data(&mut header, relative, file_data.as_slice())
                        .map_err(|e| PackageFormatError::Tar(e.to_string()))?;
                } else if entry_path.is_dir() {
                    let mut header = tar::Header::new_gnu();
                    header.set_size(0);
                    header.set_mode(0o755);
                    header.set_uid(0);
                    header.set_gid(0);
                    header.set_mtime(0);
                    header.set_entry_type(tar::EntryType::Directory);
                    header.set_cksum();

                    let dir_path = format!("{}/", relative.display());
                    tar_builder
                        .append_data(&mut header, &dir_path, std::io::empty())
                        .map_err(|e| PackageFormatError::Tar(e.to_string()))?;
                }
            }

            tar_builder
                .finish()
                .map_err(|e| PackageFormatError::Tar(e.to_string()))?;
            let encoder = tar_builder
                .into_inner()
                .map_err(|e| PackageFormatError::Tar(e.to_string()))?;
            encoder.finish()?;
        }

        // Compute digests
        let blake3_digest = LayerDigest::blake3_from_file(&tar_gz_path)?;
        let sha256_digest = LayerDigest::sha256_from_file(&tar_gz_path)?;
        let size = std::fs::metadata(&tar_gz_path)?.len();

        // Rename blob to content-addressed name
        let blob_path = self
            .blobs_dir
            .join(format!("sha256-{}", &sha256_digest.hex));
        if tar_gz_path != blob_path {
            std::fs::rename(&tar_gz_path, &blob_path)?;
        }

        debug!(
            media_type,
            digest = %sha256_digest,
            blake3 = %blake3_digest,
            size,
            "Built layer"
        );

        let descriptor = LayerDescriptor {
            media_type: media_type.to_string(),
            digest: sha256_digest.to_oci_digest(),
            size,
            annotations,
        };

        self.layers.push(BuiltLayer {
            descriptor: descriptor.clone(),
            path: blob_path,
            blake3_digest,
        });

        Ok(&self.layers.last().unwrap().descriptor)
    }

    /// Add raw bytes as a blob (for config/manifest).
    pub fn add_blob(
        &mut self,
        data: &[u8],
        media_type: &str,
    ) -> Result<LayerDescriptor, PackageFormatError> {
        let sha256_digest = LayerDigest::sha256_from_bytes(data);
        let blob_path = self
            .blobs_dir
            .join(format!("sha256-{}", &sha256_digest.hex));
        std::fs::write(&blob_path, data)?;

        Ok(LayerDescriptor {
            media_type: media_type.to_string(),
            digest: sha256_digest.to_oci_digest(),
            size: data.len() as u64,
            annotations: HashMap::new(),
        })
    }

    /// Build the final OCI manifest from all added layers.
    ///
    /// This:
    /// 1. Serializes the MapleManifest as the config blob
    /// 2. Computes config digest
    /// 3. Assembles the OCI manifest referencing all layers
    pub fn build(
        mut self,
        manifest: &maple_package::MapleManifest,
    ) -> Result<(OciManifest, PathBuf), PackageFormatError> {
        // Serialize manifest as config blob
        let config_json = serde_json::to_vec_pretty(manifest)
            .map_err(|e| PackageFormatError::Serialization(e.to_string()))?;
        let config_descriptor = self.add_blob(&config_json, MAPLE_CONFIG_MEDIA_TYPE)?;

        let mut annotations = HashMap::new();
        annotations.insert(
            "org.opencontainers.image.title".to_string(),
            manifest.name.to_qualified(),
        );
        annotations.insert(
            "org.opencontainers.image.version".to_string(),
            manifest.version.to_string(),
        );
        if let Some(ref desc) = manifest.description {
            annotations.insert(
                "org.opencontainers.image.description".to_string(),
                desc.clone(),
            );
        }
        annotations.insert(
            "ai.maple.package.kind".to_string(),
            manifest.kind.to_string(),
        );

        let oci_manifest = OciManifest {
            schema_version: 2,
            media_type: MAPLE_MANIFEST_MEDIA_TYPE.to_string(),
            config: config_descriptor,
            layers: self.layers.iter().map(|l| l.descriptor.clone()).collect(),
            annotations,
        };

        // Write the OCI manifest
        let manifest_json = serde_json::to_vec_pretty(&oci_manifest)
            .map_err(|e| PackageFormatError::Serialization(e.to_string()))?;
        let manifest_path = self.work_dir.join("manifest.json");
        std::fs::write(&manifest_path, &manifest_json)?;

        info!(
            layers = self.layers.len(),
            work_dir = %self.work_dir.display(),
            "Package built successfully"
        );

        Ok((oci_manifest, self.work_dir))
    }

    /// Get the work directory path
    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    /// Get the blobs directory path
    pub fn blobs_dir(&self) -> &Path {
        &self.blobs_dir
    }

    /// Get the number of layers added so far
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }
}

/// Recursively collect all entries under `dir`, sorted alphabetically.
///
/// This ensures deterministic tar creation regardless of filesystem ordering.
fn collect_entries_sorted(dir: &Path, entries: &mut Vec<PathBuf>) -> Result<(), PackageFormatError> {
    let mut dir_entries: Vec<PathBuf> = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        dir_entries.push(entry.path());
    }

    // Sort for determinism
    dir_entries.sort();

    for path in dir_entries {
        entries.push(path.clone());
        if path.is_dir() {
            collect_entries_sorted(&path, entries)?;
        }
    }

    Ok(())
}

/// Determine the appropriate media types for layers based on package kind.
pub fn media_types_for_kind(kind: &maple_package::PackageKind) -> Vec<(&'static str, &'static str)> {
    use maple_package::PackageKind;
    match kind {
        PackageKind::AgentPackage => vec![
            ("prompts", MAPLE_PROMPTS_LAYER_MEDIA_TYPE),
            ("skills", MAPLE_SKILL_LAYER_MEDIA_TYPE),
            ("contracts", MAPLE_CONTRACT_LAYER_MEDIA_TYPE),
            ("memory", MAPLE_MEMORY_LAYER_MEDIA_TYPE),
            ("eval", MAPLE_EVAL_LAYER_MEDIA_TYPE),
            ("static", MAPLE_STATIC_LAYER_MEDIA_TYPE),
            ("provenance", MAPLE_PROVENANCE_LAYER_MEDIA_TYPE),
        ],
        PackageKind::SkillPackage => vec![
            ("prompts", MAPLE_PROMPTS_LAYER_MEDIA_TYPE),
            ("contracts", MAPLE_CONTRACT_LAYER_MEDIA_TYPE),
            ("eval", MAPLE_EVAL_LAYER_MEDIA_TYPE),
            ("provenance", MAPLE_PROVENANCE_LAYER_MEDIA_TYPE),
        ],
        PackageKind::ContractBundle => vec![
            ("contracts", MAPLE_CONTRACT_LAYER_MEDIA_TYPE),
            ("provenance", MAPLE_PROVENANCE_LAYER_MEDIA_TYPE),
        ],
        PackageKind::ModelPackage => vec![
            ("model", MAPLE_MODEL_LAYER_MEDIA_TYPE),
            ("tokenizer", MAPLE_TOKENIZER_LAYER_MEDIA_TYPE),
            ("config", MAPLE_MODEL_CONFIG_LAYER_MEDIA_TYPE),
            ("provenance", MAPLE_PROVENANCE_LAYER_MEDIA_TYPE),
        ],
        PackageKind::EvalSuite => vec![
            ("eval", MAPLE_EVAL_LAYER_MEDIA_TYPE),
            ("provenance", MAPLE_PROVENANCE_LAYER_MEDIA_TYPE),
        ],
        PackageKind::KnowledgePack | PackageKind::PolicyPack | PackageKind::EvidencePack => vec![
            ("static", MAPLE_STATIC_LAYER_MEDIA_TYPE),
            ("provenance", MAPLE_PROVENANCE_LAYER_MEDIA_TYPE),
        ],
        PackageKind::UiModule => vec![
            ("static", MAPLE_STATIC_LAYER_MEDIA_TYPE),
            ("provenance", MAPLE_PROVENANCE_LAYER_MEDIA_TYPE),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_dir(tmp: &TempDir, name: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = tmp.path().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        for (filename, content) in files {
            let file_path = dir.join(filename);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&file_path, content).unwrap();
        }
        dir
    }

    #[test]
    fn test_deterministic_tar() {
        let tmp = TempDir::new().unwrap();
        let source = create_test_dir(
            &tmp,
            "source",
            &[
                ("file_b.txt", "content b"),
                ("file_a.txt", "content a"),
                ("sub/file_c.txt", "content c"),
            ],
        );

        let work1 = tmp.path().join("work1");
        let work2 = tmp.path().join("work2");

        let mut builder1 = PackageBuilder::new(&work1).unwrap();
        let mut builder2 = PackageBuilder::new(&work2).unwrap();

        builder1
            .add_layer(&source, MAPLE_AGENT_LAYER_MEDIA_TYPE, HashMap::new())
            .unwrap();
        builder2
            .add_layer(&source, MAPLE_AGENT_LAYER_MEDIA_TYPE, HashMap::new())
            .unwrap();

        // Same input → same digest (deterministic)
        assert_eq!(
            builder1.layers[0].descriptor.digest,
            builder2.layers[0].descriptor.digest
        );
        assert_eq!(
            builder1.layers[0].descriptor.size,
            builder2.layers[0].descriptor.size
        );
    }

    #[test]
    fn test_package_builder_basic() {
        let tmp = TempDir::new().unwrap();
        let prompts_dir = create_test_dir(
            &tmp,
            "prompts",
            &[("system.md", "You are a helpful assistant")],
        );

        let work = tmp.path().join("work");
        let mut builder = PackageBuilder::new(&work).unwrap();

        let desc = builder
            .add_layer(&prompts_dir, MAPLE_PROMPTS_LAYER_MEDIA_TYPE, HashMap::new())
            .unwrap();

        assert_eq!(desc.media_type, MAPLE_PROMPTS_LAYER_MEDIA_TYPE);
        assert!(desc.size > 0);
        assert!(desc.digest.starts_with("sha256:"));
        assert_eq!(builder.layer_count(), 1);
    }

    #[test]
    fn test_not_a_directory_error() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("not_a_dir.txt");
        std::fs::write(&file_path, "content").unwrap();

        let work = tmp.path().join("work");
        let mut builder = PackageBuilder::new(&work).unwrap();

        let result = builder.add_layer(
            &file_path,
            MAPLE_AGENT_LAYER_MEDIA_TYPE,
            HashMap::new(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_media_types_for_kind() {
        let agent_types = media_types_for_kind(&maple_package::PackageKind::AgentPackage);
        assert!(agent_types.len() >= 5);
        assert!(agent_types.iter().any(|(name, _)| *name == "prompts"));
        assert!(agent_types.iter().any(|(name, _)| *name == "provenance"));

        let model_types = media_types_for_kind(&maple_package::PackageKind::ModelPackage);
        assert!(model_types.iter().any(|(name, _)| *name == "model"));
        assert!(model_types.iter().any(|(name, _)| *name == "tokenizer"));
    }

    #[test]
    fn test_add_blob() {
        let tmp = TempDir::new().unwrap();
        let work = tmp.path().join("work");
        let mut builder = PackageBuilder::new(&work).unwrap();

        let data = b"test blob content";
        let desc = builder.add_blob(data, MAPLE_CONFIG_MEDIA_TYPE).unwrap();

        assert_eq!(desc.media_type, MAPLE_CONFIG_MEDIA_TYPE);
        assert_eq!(desc.size, data.len() as u64);
        assert!(desc.digest.starts_with("sha256:"));
    }
}
