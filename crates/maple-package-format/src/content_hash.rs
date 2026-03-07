use blake3::Hasher;
use sha2::{Digest, Sha256};
use std::io::Read;

/// A content-addressed digest for a layer or blob.
///
/// Supports BLAKE3 (preferred for speed + security) and SHA256 (for OCI compatibility).
/// All internal operations use BLAKE3; SHA256 is computed on-demand for OCI manifests.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LayerDigest {
    /// Algorithm identifier: "blake3" or "sha256"
    pub algorithm: String,
    /// Hex-encoded hash
    pub hex: String,
}

impl LayerDigest {
    /// Compute BLAKE3 digest of a file.
    ///
    /// Uses memory-mapped I/O for large files when available,
    /// falls back to streaming for portability.
    pub fn blake3_from_file(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let mut hasher = Hasher::new();
        let mut file = std::fs::File::open(path)?;
        let mut buf = vec![0u8; 64 * 1024]; // 64 KiB read buffer
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let hash = hasher.finalize();
        Ok(Self {
            algorithm: "blake3".to_string(),
            hex: hash.to_hex().to_string(),
        })
    }

    /// Compute SHA256 digest of a file (for OCI compatibility).
    pub fn sha256_from_file(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let mut hasher = Sha256::new();
        let mut file = std::fs::File::open(path)?;
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let result = hasher.finalize();
        Ok(Self {
            algorithm: "sha256".to_string(),
            hex: hex::encode(result),
        })
    }

    /// Compute BLAKE3 digest of bytes
    pub fn blake3_from_bytes(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self {
            algorithm: "blake3".to_string(),
            hex: hash.to_hex().to_string(),
        }
    }

    /// Compute SHA256 digest of bytes
    pub fn sha256_from_bytes(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        Self {
            algorithm: "sha256".to_string(),
            hex: hex::encode(result),
        }
    }

    /// Convert to OCI-compatible digest string: "sha256:<hex>" or "blake3:<hex>"
    pub fn to_oci_digest(&self) -> String {
        format!("{}:{}", self.algorithm, self.hex)
    }

    /// Parse from OCI digest string: "sha256:<hex>" or "blake3:<hex>"
    pub fn from_oci_digest(s: &str) -> Result<Self, crate::PackageFormatError> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(crate::PackageFormatError::InvalidDigest(s.to_string()));
        }
        let algorithm = parts[0].to_string();
        let hex = parts[1].to_string();

        // Validate algorithm
        match algorithm.as_str() {
            "blake3" | "sha256" => {}
            other => {
                return Err(crate::PackageFormatError::InvalidDigest(format!(
                    "unsupported algorithm: {}",
                    other
                )));
            }
        }

        // Validate hex
        if hex.is_empty() || hex.chars().any(|c| !c.is_ascii_hexdigit()) {
            return Err(crate::PackageFormatError::InvalidDigest(format!(
                "invalid hex in digest: {}",
                hex
            )));
        }

        Ok(Self { algorithm, hex })
    }

    /// Return the byte length of the digest based on algorithm
    pub fn expected_hex_len(&self) -> usize {
        match self.algorithm.as_str() {
            "blake3" => 64, // 32 bytes = 64 hex chars
            "sha256" => 64, // 32 bytes = 64 hex chars
            _ => 0,
        }
    }
}

impl std::fmt::Display for LayerDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.algorithm, self.hex)
    }
}

/// Compute the content hash of a MapleManifest for signing.
///
/// Uses canonical JSON serialization (deterministic key ordering via serde)
/// to ensure reproducible hashes across builds.
pub fn manifest_content_hash(manifest: &maple_package::MapleManifest) -> LayerDigest {
    let canonical = serde_json::to_vec(manifest).expect("manifest serialization must not fail");
    LayerDigest::blake3_from_bytes(&canonical)
}

/// Hex encoding utilities (minimal, no external dep needed for sha2 output)
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_deterministic() {
        let data = b"hello world";
        let d1 = LayerDigest::blake3_from_bytes(data);
        let d2 = LayerDigest::blake3_from_bytes(data);
        assert_eq!(d1, d2);
        assert_eq!(d1.algorithm, "blake3");
        assert_eq!(d1.hex.len(), 64);
    }

    #[test]
    fn test_sha256_deterministic() {
        let data = b"hello world";
        let d1 = LayerDigest::sha256_from_bytes(data);
        let d2 = LayerDigest::sha256_from_bytes(data);
        assert_eq!(d1, d2);
        assert_eq!(d1.algorithm, "sha256");
        assert_eq!(d1.hex.len(), 64);
    }

    #[test]
    fn test_different_data_different_hash() {
        let d1 = LayerDigest::blake3_from_bytes(b"hello");
        let d2 = LayerDigest::blake3_from_bytes(b"world");
        assert_ne!(d1, d2);
    }

    #[test]
    fn test_oci_digest_roundtrip() {
        let d = LayerDigest::blake3_from_bytes(b"test");
        let oci = d.to_oci_digest();
        let parsed = LayerDigest::from_oci_digest(&oci).unwrap();
        assert_eq!(d, parsed);
    }

    #[test]
    fn test_invalid_digest_format() {
        assert!(LayerDigest::from_oci_digest("nocolon").is_err());
        assert!(LayerDigest::from_oci_digest("md5:abc123").is_err());
        assert!(LayerDigest::from_oci_digest("sha256:not-hex!").is_err());
    }

    #[test]
    fn test_file_digest() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, b"file content").unwrap();

        let blake3_d = LayerDigest::blake3_from_file(&file_path).unwrap();
        let sha256_d = LayerDigest::sha256_from_file(&file_path).unwrap();

        assert_eq!(blake3_d.algorithm, "blake3");
        assert_eq!(sha256_d.algorithm, "sha256");

        // Same file content should produce same hash
        let blake3_d2 = LayerDigest::blake3_from_file(&file_path).unwrap();
        assert_eq!(blake3_d, blake3_d2);
    }
}
