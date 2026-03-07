//! # Build Attestation
//!
//! Combines build provenance, SBOM, evaluation results, and a cryptographic
//! signature into a single verifiable attestation document.
//!
//! A [`BuildAttestation`] ties together:
//! - **Subject**: The specific package artifact being attested.
//! - **Provenance**: Build context (source commit, builder, timestamp).
//! - **SBOM**: Complete dependency graph and AI metadata.
//! - **Eval results**: Evaluation suite outcomes.
//! - **Signature**: Ed25519 signature over the attestation payload.
//!
//! The attestation can be verified end-to-end: deserialize from JSON, check
//! the signature against a trusted public key, and inspect the provenance and
//! SBOM for compliance.

use base64::Engine;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use maple_package::BuildProvenance;
use serde::{Deserialize, Serialize};

use crate::sbom::MapleSbom;
use crate::sign::SignError;

/// A complete build attestation document.
///
/// This is the top-level artifact produced by the MAPLE trust pipeline.
/// It is designed to be serialized to JSON and distributed alongside the
/// package, or stored in a transparency log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildAttestation {
    /// Attestation format version.
    pub version: String,

    /// The subject artifact being attested.
    pub subject: AttestationSubject,

    /// Build provenance (source, builder, timestamp).
    pub provenance: BuildProvenance,

    /// Complete SBOM for the package.
    pub sbom: MapleSbom,

    /// Evaluation results (pass/fail for each suite).
    pub eval_results: Vec<EvalResult>,

    /// Ed25519 signature over the attestation payload (base64-encoded).
    ///
    /// The payload is the canonical JSON serialization of the attestation
    /// with the `signature` field set to an empty string.
    pub signature: String,
}

/// The subject of a build attestation — identifies the specific artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationSubject {
    /// Fully qualified package name.
    pub name: String,

    /// Semantic version.
    pub version: String,

    /// Content-addressed digest of the package artifact.
    pub digest: String,
}

/// The result of running an evaluation suite against a package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    /// Reference to the evaluation suite (OCI reference or name).
    pub suite_reference: String,

    /// Whether the evaluation passed.
    pub passed: bool,

    /// Numerical score (0.0 - 1.0), if applicable.
    pub score: Option<f64>,

    /// Threshold that was required for passing.
    pub threshold: Option<f64>,

    /// Timestamp when the evaluation was run.
    pub evaluated_at: DateTime<Utc>,

    /// Human-readable summary or error message.
    pub summary: Option<String>,
}

/// Create a signed build attestation.
///
/// The attestation is constructed from the provided components, then signed:
/// 1. A preliminary attestation is built with an empty signature field.
/// 2. The preliminary attestation is serialized to canonical JSON.
/// 3. The JSON bytes are signed with the Ed25519 signing key.
/// 4. The base64-encoded signature is set on the final attestation.
///
/// # Arguments
/// * `subject` - The artifact being attested.
/// * `provenance` - Build provenance data.
/// * `sbom` - The package SBOM.
/// * `eval_results` - Evaluation outcomes.
/// * `signing_key` - Ed25519 private key for signing the attestation.
///
/// # Errors
/// Returns [`SignError::Serialization`] if JSON serialization fails.
pub fn create_attestation(
    subject: AttestationSubject,
    provenance: BuildProvenance,
    sbom: MapleSbom,
    eval_results: Vec<EvalResult>,
    signing_key: &SigningKey,
) -> Result<BuildAttestation, SignError> {
    let engine = base64::engine::general_purpose::STANDARD;

    // Build the attestation with an empty signature for signing
    let mut attestation = BuildAttestation {
        version: "maple-attest/v1".to_string(),
        subject,
        provenance,
        sbom,
        eval_results,
        signature: String::new(),
    };

    // Serialize to canonical JSON for signing
    let payload =
        serde_json::to_vec(&attestation).map_err(|e| SignError::Serialization(e.to_string()))?;

    // Sign the payload
    let signature: Signature = signing_key.sign(&payload);
    attestation.signature = engine.encode(signature.to_bytes());

    Ok(attestation)
}

/// Verify a build attestation's signature against a trusted public key.
///
/// Reconstructs the signing payload by setting the signature field to empty,
/// serializing to JSON, and verifying the Ed25519 signature.
///
/// # Arguments
/// * `attestation` - The attestation to verify.
/// * `trusted_key` - The Ed25519 public key to verify against.
///
/// # Returns
/// `Ok(true)` if the signature is valid.
///
/// # Errors
/// Returns [`SignError`] if the signature is invalid or cannot be decoded.
pub fn verify_attestation(
    attestation: &BuildAttestation,
    trusted_key: &VerifyingKey,
) -> Result<bool, SignError> {
    let engine = base64::engine::general_purpose::STANDARD;

    // Reconstruct the payload that was signed (with empty signature)
    let mut attestation_for_verify = attestation.clone();
    attestation_for_verify.signature = String::new();
    let payload = serde_json::to_vec(&attestation_for_verify)
        .map_err(|e| SignError::Serialization(e.to_string()))?;

    // Decode the signature
    let sig_bytes = engine
        .decode(&attestation.signature)
        .map_err(|e| SignError::InvalidSignatureEncoding(e.to_string()))?;
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| SignError::InvalidSignatureEncoding("expected 64 bytes".to_string()))?;
    let signature = Signature::from_bytes(&sig_array);

    // Verify
    trusted_key
        .verify(&payload, &signature)
        .map_err(|_| SignError::VerificationFailed)?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sbom::{AiSbomMetadata, MapleSbom, SbomPackageInfo};
    use maple_package::{BuildProvenance, PackageKind};
    use rand::rngs::OsRng;
    use rand::RngCore;

    /// Helper: generate a random Ed25519 signing key.
    fn generate_signing_key() -> SigningKey {
        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);
        SigningKey::from_bytes(&secret)
    }

    /// Build a minimal test subject.
    fn test_subject() -> AttestationSubject {
        AttestationSubject {
            name: "testorg/agents/test-agent".to_string(),
            version: "1.0.0".to_string(),
            digest: "blake3:aabbccdd11223344".to_string(),
        }
    }

    /// Build a minimal test provenance.
    fn test_provenance() -> BuildProvenance {
        BuildProvenance {
            digest: "blake3:aabbccdd11223344".to_string(),
            built_at: Utc::now(),
            builder: Some("maple-build/0.1.0".to_string()),
            source: Some(maple_package::SourceReference {
                repository: "https://github.com/testorg/test-agent".to_string(),
                commit: "abc123def456".to_string(),
                branch: Some("main".to_string()),
                dirty: false,
            }),
            resolved_deps: vec![],
            worldline_event: None,
        }
    }

    /// Build a minimal test SBOM.
    fn test_sbom() -> MapleSbom {
        MapleSbom {
            sbom_version: "maple-sbom/v1".to_string(),
            created_at: Utc::now(),
            package: SbomPackageInfo {
                name: "testorg/agents/test-agent".to_string(),
                version: "1.0.0".to_string(),
                kind: PackageKind::AgentPackage,
                description: Some("Test agent".to_string()),
                license: Some("MIT".to_string()),
            },
            components: vec![],
            relationships: vec![],
            ai_metadata: AiSbomMetadata {
                uses_models: true,
                model_references: vec!["openai:gpt-4o".to_string()],
                eval_suites: vec![],
                data_classification: None,
                jurisdictions: vec![],
            },
        }
    }

    /// Build test eval results.
    fn test_eval_results() -> Vec<EvalResult> {
        vec![
            EvalResult {
                suite_reference: "testorg/eval/baseline".to_string(),
                passed: true,
                score: Some(0.97),
                threshold: Some(0.95),
                evaluated_at: Utc::now(),
                summary: Some("All test cases passed".to_string()),
            },
            EvalResult {
                suite_reference: "testorg/eval/safety".to_string(),
                passed: true,
                score: Some(1.0),
                threshold: Some(1.0),
                evaluated_at: Utc::now(),
                summary: None,
            },
        ]
    }

    #[test]
    fn test_create_attestation() {
        let signing_key = generate_signing_key();

        let attestation = create_attestation(
            test_subject(),
            test_provenance(),
            test_sbom(),
            test_eval_results(),
            &signing_key,
        )
        .unwrap();

        assert_eq!(attestation.version, "maple-attest/v1");
        assert_eq!(attestation.subject.name, "testorg/agents/test-agent");
        assert!(!attestation.signature.is_empty());
        assert_eq!(attestation.eval_results.len(), 2);
    }

    #[test]
    fn test_verify_attestation_with_correct_key() {
        let signing_key = generate_signing_key();
        let verifying_key = signing_key.verifying_key();

        let attestation = create_attestation(
            test_subject(),
            test_provenance(),
            test_sbom(),
            test_eval_results(),
            &signing_key,
        )
        .unwrap();

        let result = verify_attestation(&attestation, &verifying_key).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_attestation_with_wrong_key_fails() {
        let signing_key = generate_signing_key();
        let wrong_key = generate_signing_key().verifying_key();

        let attestation = create_attestation(
            test_subject(),
            test_provenance(),
            test_sbom(),
            test_eval_results(),
            &signing_key,
        )
        .unwrap();

        let result = verify_attestation(&attestation, &wrong_key);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SignError::VerificationFailed));
    }

    #[test]
    fn test_attestation_json_roundtrip() {
        let signing_key = generate_signing_key();
        let verifying_key = signing_key.verifying_key();

        let attestation = create_attestation(
            test_subject(),
            test_provenance(),
            test_sbom(),
            test_eval_results(),
            &signing_key,
        )
        .unwrap();

        // Serialize to JSON and back
        let json = serde_json::to_string_pretty(&attestation).unwrap();
        let deserialized: BuildAttestation = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, attestation.version);
        assert_eq!(deserialized.subject.name, attestation.subject.name);
        assert_eq!(deserialized.subject.digest, attestation.subject.digest);
        assert_eq!(deserialized.signature, attestation.signature);

        // The deserialized attestation should still verify
        let result = verify_attestation(&deserialized, &verifying_key).unwrap();
        assert!(result);
    }

    #[test]
    fn test_tampered_attestation_fails() {
        let signing_key = generate_signing_key();
        let verifying_key = signing_key.verifying_key();

        let mut attestation = create_attestation(
            test_subject(),
            test_provenance(),
            test_sbom(),
            test_eval_results(),
            &signing_key,
        )
        .unwrap();

        // Tamper with the subject after signing
        attestation.subject.version = "99.0.0".to_string();

        let result = verify_attestation(&attestation, &verifying_key);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SignError::VerificationFailed));
    }

    #[test]
    fn test_attestation_with_empty_eval_results() {
        let signing_key = generate_signing_key();
        let verifying_key = signing_key.verifying_key();

        let attestation = create_attestation(
            test_subject(),
            test_provenance(),
            test_sbom(),
            vec![],
            &signing_key,
        )
        .unwrap();

        assert!(attestation.eval_results.is_empty());
        let result = verify_attestation(&attestation, &verifying_key).unwrap();
        assert!(result);
    }

    #[test]
    fn test_attestation_preserves_provenance() {
        let signing_key = generate_signing_key();

        let attestation = create_attestation(
            test_subject(),
            test_provenance(),
            test_sbom(),
            test_eval_results(),
            &signing_key,
        )
        .unwrap();

        assert_eq!(
            attestation.provenance.builder,
            Some("maple-build/0.1.0".to_string())
        );
        assert!(attestation.provenance.source.is_some());
        let source = attestation.provenance.source.as_ref().unwrap();
        assert_eq!(source.commit, "abc123def456");
        assert_eq!(source.branch, Some("main".to_string()));
        assert!(!source.dirty);
    }
}
