//! Store directory layout for the local MAPLE package store.
//!
//! Default root is `~/.maple/` with the following structure:
//!
//! ```text
//! ~/.maple/
//! ├── packages/
//! │   ├── index.json
//! │   ├── blobs/blake3/<hex>/data
//! │   └── manifests/<name>/<tag>.json
//! ├── models/<model-name>/<version>/
//! ├── credentials.json
//! └── config.json
//! ```

use std::path::{Path, PathBuf};

/// Describes the on-disk layout of the local MAPLE store.
#[derive(Debug, Clone)]
pub struct StoreLayout {
    root: PathBuf,
}

impl StoreLayout {
    /// Create a new `StoreLayout` rooted at the default location (`~/.maple/`).
    ///
    /// # Panics
    ///
    /// Panics if the home directory cannot be determined.
    pub fn new() -> Self {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .expect("unable to determine home directory");
        Self {
            root: PathBuf::from(home).join(".maple"),
        }
    }

    /// Create a `StoreLayout` rooted at a custom path.
    ///
    /// Useful for testing so that tests never touch `~/.maple`.
    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    /// The store root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `<root>/packages/` — top-level packages directory.
    pub fn packages_dir(&self) -> PathBuf {
        self.root.join("packages")
    }

    /// `<root>/packages/blobs/blake3/` — content-addressed blob storage.
    pub fn blobs_dir(&self) -> PathBuf {
        self.root.join("packages").join("blobs").join("blake3")
    }

    /// `<root>/packages/manifests/` — manifest storage by name/tag.
    pub fn manifests_dir(&self) -> PathBuf {
        self.root.join("packages").join("manifests")
    }

    /// `<root>/models/` — model artifacts directory.
    pub fn models_dir(&self) -> PathBuf {
        self.root.join("models")
    }

    /// `<root>/config.json` — local configuration file.
    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }

    /// `<root>/credentials.json` — credentials file.
    pub fn credentials_path(&self) -> PathBuf {
        self.root.join("credentials.json")
    }

    /// `<root>/packages/index.json` — the package index file.
    pub fn index_path(&self) -> PathBuf {
        self.root.join("packages").join("index.json")
    }

    /// Path for a specific blob by its hex digest.
    ///
    /// Returns `<root>/packages/blobs/blake3/<hex>/data`.
    pub fn blob_path(&self, hex: &str) -> PathBuf {
        self.blobs_dir().join(hex).join("data")
    }

    /// Path for a specific manifest by name and tag/ref.
    ///
    /// Returns `<root>/packages/manifests/<name>/<tag>.json`.
    ///
    /// The `name` is used as-is (callers should sanitise slashes to a flat key
    /// or use a nested scheme). For OCI-style names with `/`, sub-directories
    /// are created automatically when the file is written.
    pub fn manifest_path(&self, name: &str, reference: &str) -> PathBuf {
        self.manifests_dir().join(name).join(format!("{}.json", reference))
    }
}

impl Default for StoreLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_with_root() {
        let root = PathBuf::from("/tmp/test-maple-store");
        let layout = StoreLayout::with_root(root.clone());
        assert_eq!(layout.root(), root);
        assert_eq!(layout.packages_dir(), root.join("packages"));
        assert_eq!(
            layout.blobs_dir(),
            root.join("packages").join("blobs").join("blake3")
        );
        assert_eq!(
            layout.manifests_dir(),
            root.join("packages").join("manifests")
        );
        assert_eq!(layout.models_dir(), root.join("models"));
        assert_eq!(layout.config_path(), root.join("config.json"));
        assert_eq!(layout.credentials_path(), root.join("credentials.json"));
        assert_eq!(
            layout.index_path(),
            root.join("packages").join("index.json")
        );
    }

    #[test]
    fn test_blob_path() {
        let root = PathBuf::from("/tmp/test-maple-store");
        let layout = StoreLayout::with_root(root.clone());
        let hex = "abcdef1234567890";
        assert_eq!(
            layout.blob_path(hex),
            root.join("packages")
                .join("blobs")
                .join("blake3")
                .join(hex)
                .join("data")
        );
    }

    #[test]
    fn test_manifest_path() {
        let root = PathBuf::from("/tmp/test-maple-store");
        let layout = StoreLayout::with_root(root.clone());
        assert_eq!(
            layout.manifest_path("my-package", "1.0.0"),
            root.join("packages")
                .join("manifests")
                .join("my-package")
                .join("1.0.0.json")
        );
    }
}
