//! # Package Signing and Verification
//!
//! Provides Ed25519-based cryptographic signing and verification for MAPLE packages.
//!
//! The signing workflow is:
//! 1. Compute a content-addressed digest of the package (via [`maple_package_format::LayerDigest`]).
//! 2. Sign the digest with an Ed25519 signing key using [`sign_package`].
//! 3. Distribute the resulting [`PackageSignature`] alongside the package.
//! 4. Consumers verify with [`verify_signature`] (self-contained) or
//!    [`verify_against_trust_anchor`] (pinned to a known public key).

use base64::Engine;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use maple_package_format::LayerDigest;
use serde::{Deserialize, Serialize};

/// A detached Ed25519 signature over a package digest.
///
/// Contains all the information needed to verify the signature independently:
/// the digest that was signed, the algorithm used, the base64-encoded signature
/// bytes, and the signer's public key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSignature {
    /// The content-addressed digest that was signed (algorithm + hex).
    pub package_digest: LayerDigest,

    /// Signature algorithm identifier (always "ed25519" for now).
    pub algorithm: String,

    /// Base64-encoded Ed25519 signature bytes (64 bytes raw -> base64).
    pub signature: String,

    /// Base64-encoded Ed25519 public key of the signer (32 bytes raw -> base64).
    pub public_key: String,

    /// Optional human-readable signer identity (e.g., email, key fingerprint).
    pub signer: Option<String>,

    /// Timestamp when the signature was created.
    pub signed_at: DateTime<Utc>,

    /// Optional free-form note attached to the signature (e.g., "release build").
    pub note: Option<String>,
}

/// Errors that can occur during signing or verification.
#[derive(Debug, thiserror::Error)]
pub enum SignError {
    /// The Ed25519 signature did not verify against the provided data and key.
    #[error("Signature verification failed")]
    VerificationFailed,

    /// The base64-encoded signature could not be decoded.
    #[error("Invalid signature encoding: {0}")]
    InvalidSignatureEncoding(String),

    /// The base64-encoded public key could not be decoded.
    #[error("Invalid public key encoding: {0}")]
    InvalidPublicKeyEncoding(String),

    /// The decoded public key bytes are not a valid Ed25519 public key.
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    /// JSON serialization/deserialization failed.
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Sign a package digest with an Ed25519 signing key.
///
/// Produces a [`PackageSignature`] that bundles the digest, the signature,
/// and the signer's public key so verifiers can check it independently.
///
/// # Arguments
/// * `digest` - Content-addressed digest of the package to sign.
/// * `signing_key` - Ed25519 private key used to produce the signature.
/// * `signer_id` - Optional human-readable identifier for the signer.
/// * `note` - Optional free-form annotation for this signature.
///
/// # Example
/// ```
/// use ed25519_dalek::SigningKey;
/// use maple_package_format::LayerDigest;
/// use maple_package_trust::sign::sign_package;
///
/// let signing_key = SigningKey::from_bytes(&[1u8; 32]);
/// let digest = LayerDigest::blake3_from_bytes(b"package content");
/// let sig = sign_package(&digest, &signing_key, None, None).unwrap();
/// assert_eq!(sig.algorithm, "ed25519");
/// ```
pub fn sign_package(
    digest: &LayerDigest,
    signing_key: &SigningKey,
    signer_id: Option<String>,
    note: Option<String>,
) -> Result<PackageSignature, SignError> {
    let engine = base64::engine::general_purpose::STANDARD;

    // Sign the canonical digest string (e.g., "blake3:<hex>")
    let digest_string = digest.to_oci_digest();
    let signature: Signature = signing_key.sign(digest_string.as_bytes());

    let verifying_key = signing_key.verifying_key();

    Ok(PackageSignature {
        package_digest: digest.clone(),
        algorithm: "ed25519".to_string(),
        signature: engine.encode(signature.to_bytes()),
        public_key: engine.encode(verifying_key.to_bytes()),
        signer: signer_id,
        signed_at: Utc::now(),
        note,
    })
}

/// Verify a package signature using the public key embedded in the signature.
///
/// This performs a self-contained verification: it decodes the public key and
/// signature from the [`PackageSignature`], reconstructs the signed message
/// from the digest, and checks the Ed25519 signature.
///
/// Returns `Ok(true)` if the signature is valid, `Ok(false)` is not used
/// (verification failure returns [`SignError::VerificationFailed`]).
///
/// # Arguments
/// * `sig` - The package signature to verify.
///
/// # Errors
/// Returns [`SignError`] if the signature or key encoding is invalid, or if
/// the cryptographic verification fails.
pub fn verify_signature(sig: &PackageSignature) -> Result<bool, SignError> {
    let engine = base64::engine::general_purpose::STANDARD;

    // Decode the public key
    let pk_bytes = engine
        .decode(&sig.public_key)
        .map_err(|e| SignError::InvalidPublicKeyEncoding(e.to_string()))?;
    let pk_array: [u8; 32] = pk_bytes
        .try_into()
        .map_err(|_| SignError::InvalidPublicKey("expected 32 bytes".to_string()))?;
    let verifying_key = VerifyingKey::from_bytes(&pk_array)
        .map_err(|e| SignError::InvalidPublicKey(e.to_string()))?;

    // Decode the signature
    let sig_bytes = engine
        .decode(&sig.signature)
        .map_err(|e| SignError::InvalidSignatureEncoding(e.to_string()))?;
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| SignError::InvalidSignatureEncoding("expected 64 bytes".to_string()))?;
    let signature = Signature::from_bytes(&sig_array);

    // Verify against the canonical digest string
    let digest_string = sig.package_digest.to_oci_digest();
    verifying_key
        .verify(digest_string.as_bytes(), &signature)
        .map_err(|_| SignError::VerificationFailed)?;

    Ok(true)
}

/// Verify a package signature against a specific trusted public key.
///
/// Unlike [`verify_signature`], this function does NOT trust the public key
/// embedded in the signature. Instead, it verifies that the signature was
/// produced by the holder of the specified `trusted_key`.
///
/// This is the recommended verification path for production use: maintain a
/// set of trusted public keys (trust anchors) and verify that packages are
/// signed by one of them.
///
/// # Arguments
/// * `sig` - The package signature to verify.
/// * `trusted_key` - The Ed25519 public key to verify against.
///
/// # Errors
/// Returns [`SignError::VerificationFailed`] if the signature was not
/// produced by the holder of `trusted_key`.
pub fn verify_against_trust_anchor(
    sig: &PackageSignature,
    trusted_key: &VerifyingKey,
) -> Result<bool, SignError> {
    let engine = base64::engine::general_purpose::STANDARD;

    // Decode the signature
    let sig_bytes = engine
        .decode(&sig.signature)
        .map_err(|e| SignError::InvalidSignatureEncoding(e.to_string()))?;
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| SignError::InvalidSignatureEncoding("expected 64 bytes".to_string()))?;
    let signature = Signature::from_bytes(&sig_array);

    // Verify against the canonical digest string using the TRUSTED key
    let digest_string = sig.package_digest.to_oci_digest();
    trusted_key
        .verify(digest_string.as_bytes(), &signature)
        .map_err(|_| SignError::VerificationFailed)?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;
    use rand::RngCore;

    /// Helper: generate a random Ed25519 signing key.
    fn generate_signing_key() -> SigningKey {
        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);
        SigningKey::from_bytes(&secret)
    }

    #[test]
    fn test_sign_and_verify_roundtrip() {
        let signing_key = generate_signing_key();
        let digest = LayerDigest::blake3_from_bytes(b"test package content");

        let sig = sign_package(&digest, &signing_key, Some("test-signer".into()), None).unwrap();

        assert_eq!(sig.algorithm, "ed25519");
        assert_eq!(sig.package_digest, digest);
        assert_eq!(sig.signer, Some("test-signer".to_string()));

        // Self-contained verification should succeed
        let result = verify_signature(&sig).unwrap();
        assert!(result);
    }

    #[test]
    fn test_tampered_digest_fails_verification() {
        let signing_key = generate_signing_key();
        let digest = LayerDigest::blake3_from_bytes(b"original content");

        let mut sig = sign_package(&digest, &signing_key, None, None).unwrap();

        // Tamper with the digest after signing
        sig.package_digest = LayerDigest::blake3_from_bytes(b"tampered content");

        let result = verify_signature(&sig);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SignError::VerificationFailed));
    }

    #[test]
    fn test_verify_against_correct_trust_anchor() {
        let signing_key = generate_signing_key();
        let trusted_key = signing_key.verifying_key();
        let digest = LayerDigest::blake3_from_bytes(b"trusted package");

        let sig = sign_package(&digest, &signing_key, None, None).unwrap();

        let result = verify_against_trust_anchor(&sig, &trusted_key).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_against_wrong_trust_anchor_fails() {
        let signing_key = generate_signing_key();
        let wrong_key = generate_signing_key().verifying_key();
        let digest = LayerDigest::blake3_from_bytes(b"package data");

        let sig = sign_package(&digest, &signing_key, None, None).unwrap();

        let result = verify_against_trust_anchor(&sig, &wrong_key);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SignError::VerificationFailed));
    }

    #[test]
    fn test_signature_json_roundtrip() {
        let signing_key = generate_signing_key();
        let digest = LayerDigest::blake3_from_bytes(b"serialize me");

        let sig = sign_package(
            &digest,
            &signing_key,
            Some("alice@example.com".into()),
            Some("release candidate".into()),
        )
        .unwrap();

        let json = serde_json::to_string_pretty(&sig).unwrap();
        let deserialized: PackageSignature = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.algorithm, sig.algorithm);
        assert_eq!(deserialized.signature, sig.signature);
        assert_eq!(deserialized.public_key, sig.public_key);
        assert_eq!(deserialized.signer, sig.signer);
        assert_eq!(deserialized.note, sig.note);
        assert_eq!(deserialized.package_digest, sig.package_digest);

        // Deserialized signature should still verify
        let result = verify_signature(&deserialized).unwrap();
        assert!(result);
    }

    #[test]
    fn test_tampered_signature_bytes_fails() {
        let signing_key = generate_signing_key();
        let digest = LayerDigest::blake3_from_bytes(b"original");

        let mut sig = sign_package(&digest, &signing_key, None, None).unwrap();

        // Corrupt the base64 signature — decode, flip a bit, re-encode
        let engine = base64::engine::general_purpose::STANDARD;
        let mut sig_bytes = engine.decode(&sig.signature).unwrap();
        sig_bytes[0] ^= 0xFF;
        sig.signature = engine.encode(&sig_bytes);

        let result = verify_signature(&sig);
        assert!(result.is_err());
    }

    #[test]
    fn test_signature_with_note() {
        let signing_key = generate_signing_key();
        let digest = LayerDigest::blake3_from_bytes(b"noted package");

        let sig = sign_package(
            &digest,
            &signing_key,
            None,
            Some("CI build #12345".into()),
        )
        .unwrap();

        assert_eq!(sig.note, Some("CI build #12345".to_string()));
    }

    #[test]
    fn test_sha256_digest_signing() {
        let signing_key = generate_signing_key();
        let digest = LayerDigest::sha256_from_bytes(b"sha256 content");

        let sig = sign_package(&digest, &signing_key, None, None).unwrap();
        let result = verify_signature(&sig).unwrap();
        assert!(result);
    }
}
