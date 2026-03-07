//! OCI Distribution Spec client for MAPLE package registries.
//!
//! Implements the core registry operations: push/pull blobs, push/pull manifests,
//! tag listing, and high-level package push/pull.

use std::collections::HashMap;

use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_LENGTH, CONTENT_TYPE, LOCATION};
use reqwest::StatusCode;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use crate::auth::{apply_auth, CredentialStore, RegistryAuth};
use maple_package_format::layout::OciManifest;

/// Default registry when none is specified in a reference.
pub const DEFAULT_REGISTRY: &str = "registry.maple.ai";

/// Errors that can occur during registry operations.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("authentication failed: {0}")]
    Auth(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("upload initiation failed: status {status}, body: {body}")]
    UploadInitFailed { status: u16, body: String },

    #[error("no upload location returned by registry")]
    NoUploadLocation,

    #[error("upload failed: status {status}, body: {body}")]
    UploadFailed { status: u16, body: String },

    #[error("blob not found: {digest}")]
    BlobNotFound { digest: String },

    #[error("manifest not found: {reference}")]
    ManifestNotFound { reference: String },

    #[error("manifest push failed: status {status}, body: {body}")]
    ManifestPushFailed { status: u16, body: String },

    #[error("list tags failed: status {status}, body: {body}")]
    ListTagsFailed { status: u16, body: String },

    #[error("invalid reference: {reason}")]
    InvalidReference { reason: String },
}

/// A parsed OCI reference.
///
/// Handles the following forms:
/// - `name:tag` (e.g. `mapleai/agents/support:1.0.0`)
/// - `registry.com/name:tag` (e.g. `registry.maple.ai/mapleai/agents/support:1.0.0`)
/// - `name@sha256:...` (digest reference)
/// - `oci://registry.com/name:tag` (explicit OCI scheme)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciReference {
    /// Registry hostname (defaults to `DEFAULT_REGISTRY`).
    pub registry: String,
    /// Repository path (e.g. `mapleai/agents/support`).
    pub repository: String,
    /// Optional tag (e.g. `1.0.0`, `latest`).
    pub tag: Option<String>,
    /// Optional digest (e.g. `sha256:abcdef...`).
    pub digest: Option<String>,
}

impl OciReference {
    /// Parse a string reference into its components.
    ///
    /// Supported formats:
    /// - `org/category/name:tag`
    /// - `registry.example.com/org/category/name:tag`
    /// - `org/name@sha256:abcdef1234...`
    /// - `oci://registry.example.com/org/name:tag`
    pub fn parse(reference: &str) -> Result<Self, RegistryError> {
        if reference.is_empty() {
            return Err(RegistryError::InvalidReference {
                reason: "empty reference".to_string(),
            });
        }

        // Strip oci:// prefix if present
        let s = reference
            .strip_prefix("oci://")
            .unwrap_or(reference);

        // Split off digest (@sha256:...) or tag (:version)
        let (name_part, tag, digest) = if let Some(at_pos) = s.rfind('@') {
            let digest_str = s[at_pos + 1..].to_string();
            let name_part = &s[..at_pos];
            (name_part, None, Some(digest_str))
        } else if let Some(colon_pos) = s.rfind(':') {
            let after_colon = &s[colon_pos + 1..];
            // Check if this is a port number (contains a slash after)
            if after_colon.contains('/') {
                // Port in registry hostname, no tag
                (s, None, None)
            } else {
                let tag_str = after_colon.to_string();
                let name_part = &s[..colon_pos];
                (name_part, Some(tag_str), None)
            }
        } else {
            (s, None, None)
        };

        // Determine if the first segment is a registry hostname
        let segments: Vec<&str> = name_part.split('/').collect();
        if segments.is_empty() || segments.iter().any(|s| s.is_empty()) {
            return Err(RegistryError::InvalidReference {
                reason: format!("invalid path segments in '{}'", name_part),
            });
        }

        let (registry, repository) = if segments.len() >= 2
            && (segments[0].contains('.') || segments[0].contains(':'))
        {
            // First segment looks like a hostname
            (
                segments[0].to_string(),
                segments[1..].join("/"),
            )
        } else {
            // No registry in reference, use default
            (DEFAULT_REGISTRY.to_string(), name_part.to_string())
        };

        if repository.is_empty() {
            return Err(RegistryError::InvalidReference {
                reason: "empty repository name".to_string(),
            });
        }

        Ok(Self {
            registry,
            repository,
            tag,
            digest,
        })
    }

    /// Returns the reference string suitable for display.
    pub fn to_string_ref(&self) -> String {
        let mut s = format!("{}/{}", self.registry, self.repository);
        if let Some(ref tag) = self.tag {
            s.push(':');
            s.push_str(tag);
        }
        if let Some(ref digest) = self.digest {
            s.push('@');
            s.push_str(digest);
        }
        s
    }
}

impl std::fmt::Display for OciReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_ref())
    }
}

/// Compute SHA256 hex digest of a byte slice.
pub fn sha2_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Format a byte count into a human-readable string.
pub fn humanize_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;
    const TIB: u64 = 1024 * GIB;

    if bytes >= TIB {
        format!("{:.2} TiB", bytes as f64 / TIB as f64)
    } else if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.2} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// OCI Distribution Spec registry client.
///
/// Provides push/pull operations for blobs, manifests, and complete MAPLE packages
/// against OCI-compliant registries.
pub struct OciRegistryClient {
    http: reqwest::Client,
    credentials: CredentialStore,
}

impl OciRegistryClient {
    /// Create a new client with default credential loading.
    pub fn new() -> Result<Self, RegistryError> {
        let credentials = CredentialStore::load().map_err(|e| {
            warn!("Failed to load credentials, continuing with anonymous: {}", e);
            e
        }).unwrap_or_else(|_| {
            // Create a minimal credential store on failure
            CredentialStore::load().unwrap_or_else(|_| {
                // This should not happen in practice; load() only errors on
                // parse failures, and we recover with defaults.
                panic!("credential store initialization failed twice");
            })
        });

        let http = reqwest::Client::builder()
            .user_agent("maple-registry-client/0.1")
            .build()?;

        Ok(Self { http, credentials })
    }

    /// Create a client with an explicit credential store (useful for testing).
    pub fn with_credentials(credentials: CredentialStore) -> Result<Self, RegistryError> {
        let http = reqwest::Client::builder()
            .user_agent("maple-registry-client/0.1")
            .build()?;

        Ok(Self { http, credentials })
    }

    /// Build the base URL for a registry.
    fn base_url(&self, registry: &str) -> String {
        if registry.starts_with("http://") || registry.starts_with("https://") {
            format!("{}/v2", registry)
        } else {
            format!("https://{}/v2", registry)
        }
    }

    /// Build authenticated request headers for a registry.
    fn auth_headers(&self, registry: &str) -> HeaderMap {
        let auth = self.credentials.get_auth(registry);
        let mut headers = HeaderMap::new();
        apply_auth(&mut headers, &auth);
        headers
    }

    /// Check if a blob exists in the registry (HEAD request).
    pub async fn blob_exists(
        &self,
        registry: &str,
        repository: &str,
        digest: &str,
    ) -> Result<bool, RegistryError> {
        let url = format!(
            "{}/{}/blobs/{}",
            self.base_url(registry),
            repository,
            digest
        );
        debug!(url = %url, "Checking blob existence");

        let resp = self
            .http
            .head(&url)
            .headers(self.auth_headers(registry))
            .send()
            .await?;

        match resp.status() {
            StatusCode::OK => Ok(true),
            StatusCode::NOT_FOUND => Ok(false),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(RegistryError::Auth(format!(
                    "authentication failed for {}",
                    registry
                )))
            }
            status => {
                warn!(status = %status, "Unexpected status checking blob");
                Ok(false)
            }
        }
    }

    /// Push a blob to the registry using the monolithic upload protocol.
    pub async fn push_blob(
        &self,
        registry: &str,
        repository: &str,
        digest: &str,
        data: Bytes,
    ) -> Result<(), RegistryError> {
        let size = data.len();
        info!(
            digest = %digest,
            size = %humanize_bytes(size as u64),
            "Pushing blob"
        );

        // Step 1: Initiate upload (POST)
        let init_url = format!(
            "{}/{}/blobs/uploads/",
            self.base_url(registry),
            repository
        );

        let resp = self
            .http
            .post(&init_url)
            .headers(self.auth_headers(registry))
            .send()
            .await?;

        let status = resp.status();
        if status != StatusCode::ACCEPTED && status != StatusCode::OK {
            let body = resp.text().await.unwrap_or_default();
            return Err(RegistryError::UploadInitFailed {
                status: status.as_u16(),
                body,
            });
        }

        // Step 2: Get the upload location
        let location = resp
            .headers()
            .get(LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(String::from)
            .ok_or(RegistryError::NoUploadLocation)?;

        // Step 3: Complete the upload (PUT with digest query param)
        let upload_url = if location.contains('?') {
            format!("{}&digest={}", location, digest)
        } else {
            format!("{}?digest={}", location, digest)
        };

        let mut headers = self.auth_headers(registry);
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
        headers.insert(CONTENT_LENGTH, HeaderValue::from(size));

        let resp = self
            .http
            .put(&upload_url)
            .headers(headers)
            .body(data)
            .send()
            .await?;

        let status = resp.status();
        if status != StatusCode::CREATED && status != StatusCode::OK {
            let body = resp.text().await.unwrap_or_default();
            return Err(RegistryError::UploadFailed {
                status: status.as_u16(),
                body,
            });
        }

        debug!(digest = %digest, "Blob pushed successfully");
        Ok(())
    }

    /// Pull a blob from the registry.
    pub async fn pull_blob(
        &self,
        registry: &str,
        repository: &str,
        digest: &str,
    ) -> Result<Bytes, RegistryError> {
        let url = format!(
            "{}/{}/blobs/{}",
            self.base_url(registry),
            repository,
            digest
        );
        debug!(url = %url, "Pulling blob");

        let resp = self
            .http
            .get(&url)
            .headers(self.auth_headers(registry))
            .send()
            .await?;

        match resp.status() {
            StatusCode::OK => {
                let data = resp.bytes().await?;
                info!(
                    digest = %digest,
                    size = %humanize_bytes(data.len() as u64),
                    "Blob pulled"
                );
                Ok(data)
            }
            StatusCode::NOT_FOUND => Err(RegistryError::BlobNotFound {
                digest: digest.to_string(),
            }),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(RegistryError::Auth(
                format!("authentication failed for {}", registry),
            )),
            status => {
                let body = resp.text().await.unwrap_or_default();
                Err(RegistryError::BlobNotFound {
                    digest: format!("{} (status: {}, body: {})", digest, status, body),
                })
            }
        }
    }

    /// Push an OCI manifest to the registry.
    pub async fn push_manifest(
        &self,
        registry: &str,
        repository: &str,
        reference: &str,
        manifest: &OciManifest,
    ) -> Result<String, RegistryError> {
        let url = format!(
            "{}/{}/manifests/{}",
            self.base_url(registry),
            repository,
            reference
        );

        let manifest_json = serde_json::to_vec(manifest)?;
        let digest = format!("sha256:{}", sha2_hex(&manifest_json));

        info!(
            reference = %reference,
            digest = %digest,
            "Pushing manifest"
        );

        let mut headers = self.auth_headers(registry);
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/vnd.oci.image.manifest.v1+json"),
        );

        let resp = self
            .http
            .put(&url)
            .headers(headers)
            .body(manifest_json)
            .send()
            .await?;

        let status = resp.status();
        if status != StatusCode::CREATED && status != StatusCode::OK {
            let body = resp.text().await.unwrap_or_default();
            return Err(RegistryError::ManifestPushFailed {
                status: status.as_u16(),
                body,
            });
        }

        debug!(digest = %digest, "Manifest pushed successfully");
        Ok(digest)
    }

    /// Pull an OCI manifest from the registry.
    pub async fn pull_manifest(
        &self,
        registry: &str,
        repository: &str,
        reference: &str,
    ) -> Result<OciManifest, RegistryError> {
        let url = format!(
            "{}/{}/manifests/{}",
            self.base_url(registry),
            repository,
            reference
        );
        debug!(url = %url, "Pulling manifest");

        let mut headers = self.auth_headers(registry);
        headers.insert(
            reqwest::header::ACCEPT,
            HeaderValue::from_static("application/vnd.oci.image.manifest.v1+json"),
        );

        let resp = self
            .http
            .get(&url)
            .headers(headers)
            .send()
            .await?;

        match resp.status() {
            StatusCode::OK => {
                let manifest: OciManifest = resp.json().await?;
                info!(
                    reference = %reference,
                    layers = manifest.layers.len(),
                    "Manifest pulled"
                );
                Ok(manifest)
            }
            StatusCode::NOT_FOUND => Err(RegistryError::ManifestNotFound {
                reference: reference.to_string(),
            }),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(RegistryError::Auth(
                format!("authentication failed for {}", registry),
            )),
            status => {
                let body = resp.text().await.unwrap_or_default();
                Err(RegistryError::ManifestNotFound {
                    reference: format!("{} (status: {}, body: {})", reference, status, body),
                })
            }
        }
    }

    /// List tags for a repository.
    pub async fn list_tags(
        &self,
        registry: &str,
        repository: &str,
    ) -> Result<Vec<String>, RegistryError> {
        let url = format!(
            "{}/{}/tags/list",
            self.base_url(registry),
            repository
        );
        debug!(url = %url, "Listing tags");

        let resp = self
            .http
            .get(&url)
            .headers(self.auth_headers(registry))
            .send()
            .await?;

        match resp.status() {
            StatusCode::OK => {
                let body: serde_json::Value = resp.json().await?;
                let tags: Vec<String> = body
                    .get("tags")
                    .and_then(|t| t.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                info!(
                    repository = %repository,
                    count = tags.len(),
                    "Tags listed"
                );
                Ok(tags)
            }
            status => {
                let body = resp.text().await.unwrap_or_default();
                Err(RegistryError::ListTagsFailed {
                    status: status.as_u16(),
                    body,
                })
            }
        }
    }

    /// Push a complete MAPLE package to a registry.
    ///
    /// This performs the full push sequence:
    /// 1. Push config blob
    /// 2. Push each layer blob (with dedup via blob_exists)
    /// 3. Push the OCI manifest
    pub async fn push_package(
        &self,
        reference: &OciReference,
        manifest: &OciManifest,
        blobs: &HashMap<String, Bytes>,
    ) -> Result<String, RegistryError> {
        info!(
            reference = %reference,
            layers = manifest.layers.len(),
            "Pushing package"
        );

        let registry = &reference.registry;
        let repository = &reference.repository;

        // Push config blob
        let config_digest = &manifest.config.digest;
        if let Some(config_data) = blobs.get(config_digest) {
            if !self.blob_exists(registry, repository, config_digest).await? {
                self.push_blob(registry, repository, config_digest, config_data.clone())
                    .await?;
            } else {
                debug!(digest = %config_digest, "Config blob already exists, skipping");
            }
        }

        // Push layer blobs
        for layer in &manifest.layers {
            let layer_digest = &layer.digest;
            if let Some(layer_data) = blobs.get(layer_digest) {
                if !self.blob_exists(registry, repository, layer_digest).await? {
                    self.push_blob(registry, repository, layer_digest, layer_data.clone())
                        .await?;
                } else {
                    debug!(digest = %layer_digest, "Layer blob already exists, skipping");
                }
            } else {
                warn!(digest = %layer_digest, "Layer blob not provided in blobs map");
            }
        }

        // Push manifest
        let tag = reference
            .tag
            .as_deref()
            .or(reference.digest.as_deref())
            .unwrap_or("latest");

        let manifest_digest = self
            .push_manifest(registry, repository, tag, manifest)
            .await?;

        info!(
            reference = %reference,
            digest = %manifest_digest,
            "Package pushed successfully"
        );

        Ok(manifest_digest)
    }

    /// Pull a complete MAPLE package from a registry.
    ///
    /// Returns the OCI manifest and a map of digest -> blob data.
    pub async fn pull_package(
        &self,
        reference: &OciReference,
    ) -> Result<(OciManifest, HashMap<String, Bytes>), RegistryError> {
        info!(reference = %reference, "Pulling package");

        let registry = &reference.registry;
        let repository = &reference.repository;
        let tag = reference
            .tag
            .as_deref()
            .or(reference.digest.as_deref())
            .unwrap_or("latest");

        // Pull manifest
        let manifest = self.pull_manifest(registry, repository, tag).await?;

        let mut blobs = HashMap::new();

        // Pull config blob
        let config_data = self
            .pull_blob(registry, repository, &manifest.config.digest)
            .await?;
        blobs.insert(manifest.config.digest.clone(), config_data);

        // Pull all layer blobs
        for layer in &manifest.layers {
            let layer_data = self
                .pull_blob(registry, repository, &layer.digest)
                .await?;
            blobs.insert(layer.digest.clone(), layer_data);
        }

        info!(
            reference = %reference,
            layers = manifest.layers.len(),
            total_blobs = blobs.len(),
            "Package pulled successfully"
        );

        Ok((manifest, blobs))
    }

    /// Login to a registry (verify credentials and optionally store them).
    pub async fn login(
        &mut self,
        registry: &str,
        auth: RegistryAuth,
    ) -> Result<(), RegistryError> {
        info!(registry = %registry, "Logging in");

        // Test the credentials with a /v2/ ping
        let url = format!("{}/", self.base_url(registry));
        let mut headers = HeaderMap::new();
        apply_auth(&mut headers, &auth);

        let resp = self.http.get(&url).headers(headers).send().await?;

        match resp.status() {
            StatusCode::OK | StatusCode::UNAUTHORIZED => {
                // 401 with WWW-Authenticate is expected for token-based auth
                // Store the credentials for future use
                self.credentials.store(registry, auth);
                info!(registry = %registry, "Login successful");
                Ok(())
            }
            status => Err(RegistryError::Auth(format!(
                "login failed with status {} for registry {}",
                status, registry
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oci_reference_parse_full_form() {
        let r = OciReference::parse("registry.maple.ai/mapleai/agents/support:1.0.0").unwrap();
        assert_eq!(r.registry, "registry.maple.ai");
        assert_eq!(r.repository, "mapleai/agents/support");
        assert_eq!(r.tag.as_deref(), Some("1.0.0"));
        assert!(r.digest.is_none());
    }

    #[test]
    fn test_oci_reference_parse_short_form() {
        let r = OciReference::parse("mapleai/agents/support:1.0.0").unwrap();
        assert_eq!(r.registry, DEFAULT_REGISTRY);
        assert_eq!(r.repository, "mapleai/agents/support");
        assert_eq!(r.tag.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn test_oci_reference_parse_digest() {
        let r = OciReference::parse(
            "mapleai/agents/support@sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        )
        .unwrap();
        assert_eq!(r.registry, DEFAULT_REGISTRY);
        assert_eq!(r.repository, "mapleai/agents/support");
        assert!(r.tag.is_none());
        assert_eq!(
            r.digest.as_deref(),
            Some("sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
        );
    }

    #[test]
    fn test_oci_reference_parse_no_tag() {
        let r = OciReference::parse("mapleai/agents/support").unwrap();
        assert_eq!(r.registry, DEFAULT_REGISTRY);
        assert_eq!(r.repository, "mapleai/agents/support");
        assert!(r.tag.is_none());
        assert!(r.digest.is_none());
    }

    #[test]
    fn test_oci_reference_parse_with_oci_prefix() {
        let r =
            OciReference::parse("oci://registry.example.com/org/agents/myagent:2.0.0").unwrap();
        assert_eq!(r.registry, "registry.example.com");
        assert_eq!(r.repository, "org/agents/myagent");
        assert_eq!(r.tag.as_deref(), Some("2.0.0"));
    }

    #[test]
    fn test_oci_reference_parse_default_registry() {
        let r = OciReference::parse("org/name:latest").unwrap();
        assert_eq!(r.registry, DEFAULT_REGISTRY);
        assert_eq!(r.repository, "org/name");
        assert_eq!(r.tag.as_deref(), Some("latest"));
    }

    #[test]
    fn test_oci_reference_parse_empty() {
        let result = OciReference::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_oci_reference_display() {
        let r = OciReference {
            registry: "registry.maple.ai".to_string(),
            repository: "mapleai/agents/support".to_string(),
            tag: Some("1.0.0".to_string()),
            digest: None,
        };
        assert_eq!(
            r.to_string(),
            "registry.maple.ai/mapleai/agents/support:1.0.0"
        );
    }

    #[test]
    fn test_oci_reference_display_with_digest() {
        let r = OciReference {
            registry: "registry.maple.ai".to_string(),
            repository: "mapleai/agents/support".to_string(),
            tag: None,
            digest: Some("sha256:abc123".to_string()),
        };
        assert_eq!(
            r.to_string(),
            "registry.maple.ai/mapleai/agents/support@sha256:abc123"
        );
    }

    #[test]
    fn test_sha2_hex() {
        let digest = sha2_hex(b"hello world");
        // Known SHA256 of "hello world"
        assert_eq!(
            digest,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_sha2_hex_empty() {
        let digest = sha2_hex(b"");
        // Known SHA256 of empty string
        assert_eq!(
            digest,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_humanize_bytes_b() {
        assert_eq!(humanize_bytes(0), "0 B");
        assert_eq!(humanize_bytes(512), "512 B");
        assert_eq!(humanize_bytes(1023), "1023 B");
    }

    #[test]
    fn test_humanize_bytes_kib() {
        assert_eq!(humanize_bytes(1024), "1.00 KiB");
        assert_eq!(humanize_bytes(1536), "1.50 KiB");
        assert_eq!(humanize_bytes(10240), "10.00 KiB");
    }

    #[test]
    fn test_humanize_bytes_mib() {
        assert_eq!(humanize_bytes(1048576), "1.00 MiB");
        assert_eq!(humanize_bytes(5 * 1024 * 1024), "5.00 MiB");
    }

    #[test]
    fn test_humanize_bytes_gib() {
        assert_eq!(humanize_bytes(1073741824), "1.00 GiB");
        assert_eq!(humanize_bytes(2 * 1024 * 1024 * 1024), "2.00 GiB");
    }

    #[test]
    fn test_humanize_bytes_tib() {
        assert_eq!(humanize_bytes(1099511627776), "1.00 TiB");
    }
}
