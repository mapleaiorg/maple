//! Package store operations — content-addressed blob storage, manifest management,
//! index tracking, garbage collection, and disk usage.

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use maple_package::PackageKind;
use maple_package_format::OciManifest;

use crate::error::StoreError;
use crate::layout::StoreLayout;

/// The on-disk package index: maps qualified package names to their metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageIndex {
    /// Map from qualified package name to its index entry.
    pub entries: HashMap<String, IndexEntry>,
}

/// Metadata for a single package tracked by the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Qualified package name (e.g. `mapleai/agents/support`).
    pub name: String,
    /// Map of tag to the BLAKE3 hex digest of the manifest blob.
    pub tags: HashMap<String, String>,
    /// The kind of package.
    pub kind: PackageKind,
    /// When the package was last pulled from a registry (if ever).
    pub last_pulled: Option<DateTime<Utc>>,
}

/// Result of a garbage-collection run.
#[derive(Debug, Clone, Default)]
pub struct GcResult {
    /// Number of blobs removed.
    pub blobs_removed: u64,
    /// Total bytes freed.
    pub bytes_freed: u64,
}

/// Summary of disk usage for the store.
#[derive(Debug, Clone, Default)]
pub struct DiskUsage {
    /// Total bytes used by blobs.
    pub blobs_bytes: u64,
    /// Number of blobs.
    pub blobs_count: u64,
    /// Total bytes used by manifests.
    pub manifests_bytes: u64,
    /// Number of manifests.
    pub manifests_count: u64,
    /// Combined total bytes.
    pub total_bytes: u64,
}

/// The local package store.
///
/// Provides content-addressed blob storage, manifest management,
/// an index for tracking installed packages, and housekeeping operations.
pub struct PackageStore {
    layout: StoreLayout,
}

impl PackageStore {
    /// Open the store at the default location (`~/.maple/`).
    ///
    /// Creates the directory structure if it does not exist.
    pub async fn open() -> Result<Self, StoreError> {
        let layout = StoreLayout::new();
        Self::init_dirs(&layout).await?;
        Ok(Self { layout })
    }

    /// Open the store at a custom root (primarily for testing).
    ///
    /// Creates the directory structure if it does not exist.
    pub async fn open_at(root: PathBuf) -> Result<Self, StoreError> {
        let layout = StoreLayout::with_root(root);
        Self::init_dirs(&layout).await?;
        Ok(Self { layout })
    }

    /// Ensure required directories exist.
    async fn init_dirs(layout: &StoreLayout) -> Result<(), StoreError> {
        tokio::fs::create_dir_all(layout.packages_dir()).await?;
        tokio::fs::create_dir_all(layout.blobs_dir()).await?;
        tokio::fs::create_dir_all(layout.manifests_dir()).await?;
        tokio::fs::create_dir_all(layout.models_dir()).await?;
        Ok(())
    }

    // ---------------------------------------------------------------
    // Blob operations
    // ---------------------------------------------------------------

    /// Store a blob by its BLAKE3 hex digest.
    ///
    /// The data is written to `<blobs_dir>/<hex>/data`. If a blob with the
    /// same digest already exists the write is skipped (content-addressed
    /// deduplication).
    pub async fn store_blob(&self, digest: &str, data: &[u8]) -> Result<PathBuf, StoreError> {
        let path = self.layout.blob_path(digest);
        if path.exists() {
            debug!(digest, "blob already exists, skipping write");
            return Ok(path);
        }
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, data).await?;
        debug!(digest, bytes = data.len(), "stored blob");
        Ok(path)
    }

    /// Retrieve a blob by its BLAKE3 hex digest.
    ///
    /// Returns `None` if the blob does not exist.
    pub async fn get_blob(&self, digest: &str) -> Result<Option<Vec<u8>>, StoreError> {
        let path = self.layout.blob_path(digest);
        if !path.exists() {
            return Ok(None);
        }
        let data = tokio::fs::read(&path).await?;
        Ok(Some(data))
    }

    /// Check whether a blob with the given digest exists on disk.
    pub fn has_blob(&self, digest: &str) -> bool {
        self.layout.blob_path(digest).exists()
    }

    // ---------------------------------------------------------------
    // Manifest operations
    // ---------------------------------------------------------------

    /// Store a manifest for a given package name and tag.
    ///
    /// The manifest is serialised to JSON and written to
    /// `<manifests_dir>/<name>/<tag>.json`.
    pub async fn store_manifest(
        &self,
        name: &str,
        tag: &str,
        manifest: &OciManifest,
    ) -> Result<PathBuf, StoreError> {
        let path = self.layout.manifest_path(name, tag);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let json = serde_json::to_vec_pretty(manifest)?;
        tokio::fs::write(&path, &json).await?;
        debug!(name, tag, "stored manifest");
        Ok(path)
    }

    /// Retrieve a manifest for a given package name and tag.
    ///
    /// Returns `None` if the manifest does not exist.
    pub async fn get_manifest(
        &self,
        name: &str,
        tag: &str,
    ) -> Result<Option<OciManifest>, StoreError> {
        let path = self.layout.manifest_path(name, tag);
        if !path.exists() {
            return Ok(None);
        }
        let data = tokio::fs::read(&path).await?;
        let manifest: OciManifest = serde_json::from_slice(&data)?;
        Ok(Some(manifest))
    }

    // ---------------------------------------------------------------
    // Index operations
    // ---------------------------------------------------------------

    /// Load the package index from disk.
    ///
    /// If the index file does not exist, an empty index is returned.
    pub async fn load_index(&self) -> Result<PackageIndex, StoreError> {
        let path = self.layout.index_path();
        if !path.exists() {
            return Ok(PackageIndex::default());
        }
        let data = tokio::fs::read(&path).await?;
        let index: PackageIndex = serde_json::from_slice(&data)?;
        Ok(index)
    }

    /// Persist the package index to disk.
    pub async fn save_index(&self, index: &PackageIndex) -> Result<(), StoreError> {
        let path = self.layout.index_path();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let json = serde_json::to_vec_pretty(index)?;
        tokio::fs::write(&path, &json).await?;
        debug!("saved package index ({} entries)", index.entries.len());
        Ok(())
    }

    /// List all packages tracked in the index.
    pub async fn list_packages(&self) -> Result<Vec<IndexEntry>, StoreError> {
        let index = self.load_index().await?;
        Ok(index.entries.into_values().collect())
    }

    /// Remove a specific tag for a package.
    ///
    /// Removes the manifest file, the tag from the index entry, and if the
    /// entry has no remaining tags the entire index entry is removed.
    pub async fn remove(&self, name: &str, tag: &str) -> Result<(), StoreError> {
        // Remove the manifest file
        let manifest_path = self.layout.manifest_path(name, tag);
        if manifest_path.exists() {
            tokio::fs::remove_file(&manifest_path).await?;
            debug!(name, tag, "removed manifest file");
        }

        // Update the index
        let mut index = self.load_index().await?;
        let mut remove_entry = false;
        if let Some(entry) = index.entries.get_mut(name) {
            entry.tags.remove(tag);
            if entry.tags.is_empty() {
                remove_entry = true;
            }
        }
        if remove_entry {
            index.entries.remove(name);
        }
        self.save_index(&index).await?;
        info!(name, tag, "removed package tag");
        Ok(())
    }

    // ---------------------------------------------------------------
    // Housekeeping
    // ---------------------------------------------------------------

    /// Run garbage collection: remove blobs that are not referenced by any
    /// manifest currently in the index.
    ///
    /// This is a simple mark-and-sweep:
    /// 1. Collect all digests referenced by manifests in the index.
    /// 2. Walk the blobs directory and remove any blob not in the set.
    pub async fn gc(&self) -> Result<GcResult, StoreError> {
        let index = self.load_index().await?;

        // Collect referenced digests from all stored manifests.
        let mut referenced: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for entry in index.entries.values() {
            for (_tag, digest) in &entry.tags {
                referenced.insert(digest.clone());
            }
        }

        let mut result = GcResult::default();

        let blobs_dir = self.layout.blobs_dir();
        if !blobs_dir.exists() {
            return Ok(result);
        }

        // Walk blob directories: each is <blobs_dir>/<hex>/data
        let mut dir_entries = tokio::fs::read_dir(&blobs_dir).await?;
        while let Some(entry) = dir_entries.next_entry().await? {
            let hex_name = entry
                .file_name()
                .to_string_lossy()
                .to_string();
            if !referenced.contains(&hex_name) {
                // This blob is unreferenced — remove it.
                let blob_dir = entry.path();
                let data_path = blob_dir.join("data");
                if data_path.exists() {
                    let meta = tokio::fs::metadata(&data_path).await?;
                    result.bytes_freed += meta.len();
                }
                tokio::fs::remove_dir_all(&blob_dir).await?;
                result.blobs_removed += 1;
                debug!(hex = hex_name, "gc: removed unreferenced blob");
            }
        }

        info!(
            removed = result.blobs_removed,
            freed = result.bytes_freed,
            "garbage collection complete"
        );
        Ok(result)
    }

    /// Calculate disk usage for the store.
    pub async fn disk_usage(&self) -> Result<DiskUsage, StoreError> {
        let mut usage = DiskUsage::default();

        // Walk blobs
        let blobs_dir = self.layout.blobs_dir();
        if blobs_dir.exists() {
            for entry in walkdir::WalkDir::new(&blobs_dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    if let Ok(meta) = entry.metadata() {
                        usage.blobs_bytes += meta.len();
                        usage.blobs_count += 1;
                    }
                }
            }
        }

        // Walk manifests
        let manifests_dir = self.layout.manifests_dir();
        if manifests_dir.exists() {
            for entry in walkdir::WalkDir::new(&manifests_dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    if let Ok(meta) = entry.metadata() {
                        usage.manifests_bytes += meta.len();
                        usage.manifests_count += 1;
                    }
                }
            }
        }

        usage.total_bytes = usage.blobs_bytes + usage.manifests_bytes;
        Ok(usage)
    }

    /// Borrow the underlying layout.
    pub fn layout(&self) -> &StoreLayout {
        &self.layout
    }
}
