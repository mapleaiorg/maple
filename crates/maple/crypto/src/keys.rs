//! Ed25519 key pairs and genesis key material.

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use zeroize::Zeroize;

use crate::signature::SignerId;
use crate::Signature;

/// Ed25519 key pair for a Worldline.
///
/// The signing key is zeroized on drop for security.
///
/// # Example
/// ```
/// use maple_crypto::KeyPair;
///
/// let kp = KeyPair::generate();
/// let data = b"test message";
/// let sig = kp.sign(data);
/// assert!(kp.verify(data, &sig));
/// ```
pub struct KeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl KeyPair {
    /// Generate a new random key pair.
    pub fn generate() -> Self {
        let mut secret = [0u8; 32];
        use rand::RngCore;
        OsRng.fill_bytes(&mut secret);
        let signing_key = SigningKey::from_bytes(&secret);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Create from an existing signing key.
    pub fn from_signing_key(signing_key: SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Get the verifying (public) key.
    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    /// Sign data, producing a detached signature.
    ///
    /// The signer field is set to ZERO (anonymous).
    /// Use `sign_as` to set a specific identity.
    pub fn sign(&self, data: &[u8]) -> Signature {
        use ed25519_dalek::Signer;
        let sig = self.signing_key.sign(data);
        Signature {
            bytes: sig.to_bytes(),
            signer: SignerId::ZERO,
        }
    }

    /// Sign data with a specific signer identity.
    pub fn sign_as(&self, data: &[u8], signer: SignerId) -> Signature {
        use ed25519_dalek::Signer;
        let sig = self.signing_key.sign(data);
        Signature {
            bytes: sig.to_bytes(),
            signer,
        }
    }

    /// Verify a signature against data.
    pub fn verify(&self, data: &[u8], signature: &Signature) -> bool {
        use ed25519_dalek::Verifier;
        let sig = ed25519_dalek::Signature::from_bytes(&signature.bytes);
        self.verifying_key.verify(data, &sig).is_ok()
    }

    /// Get the verifying key bytes (32 bytes).
    pub fn verifying_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }
}

impl Clone for KeyPair {
    fn clone(&self) -> Self {
        Self {
            signing_key: self.signing_key.clone(),
            verifying_key: self.verifying_key,
        }
    }
}

impl std::fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyPair")
            .field(
                "verifying_key",
                &self
                    .verifying_key
                    .as_bytes()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
            )
            .field("signing_key", &"[REDACTED]")
            .finish()
    }
}

impl Drop for KeyPair {
    fn drop(&mut self) {
        let mut key_bytes = self.signing_key.to_bytes();
        key_bytes.zeroize();
    }
}

/// Genesis key material for creating a new Worldline identity.
///
/// The identity hash is derived deterministically from the key material
/// and creation nonce via BLAKE3.
///
/// # Example
/// ```
/// use maple_crypto::GenesisKeyMaterial;
///
/// let genesis = GenesisKeyMaterial::generate();
/// let id_hash = genesis.derive_identity_hash();
///
/// // Deterministic: same genesis material always produces same hash
/// let id_hash2 = genesis.derive_identity_hash();
/// assert_eq!(id_hash, id_hash2);
/// ```
pub struct GenesisKeyMaterial {
    /// The key pair for this worldline.
    pub key_pair: KeyPair,
    /// Random nonce used during creation.
    pub creation_nonce: [u8; 32],
}

impl GenesisKeyMaterial {
    /// Generate fresh genesis key material.
    pub fn generate() -> Self {
        let mut nonce = [0u8; 32];
        use rand::RngCore;
        OsRng.fill_bytes(&mut nonce);

        Self {
            key_pair: KeyPair::generate(),
            creation_nonce: nonce,
        }
    }

    /// Derive the identity hash from this genesis material.
    ///
    /// `identity_hash = BLAKE3(verifying_key_bytes || creation_nonce)`
    ///
    /// This 32-byte hash can be used to construct a WorldlineId or
    /// any other domain-specific identifier.
    pub fn derive_identity_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.key_pair.verifying_key().as_bytes());
        hasher.update(&self.creation_nonce);
        *hasher.finalize().as_bytes()
    }

    /// Get a SignerId from this genesis material.
    pub fn signer_id(&self) -> SignerId {
        SignerId::new(self.derive_identity_hash())
    }
}

impl Clone for GenesisKeyMaterial {
    fn clone(&self) -> Self {
        Self {
            key_pair: self.key_pair.clone(),
            creation_nonce: self.creation_nonce,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_verify_roundtrip() {
        let kp = KeyPair::generate();
        let data = b"hello MAPLE world";
        let sig = kp.sign(data);
        assert!(kp.verify(data, &sig));
    }

    #[test]
    fn test_sign_verify_wrong_data() {
        let kp = KeyPair::generate();
        let sig = kp.sign(b"correct data");
        assert!(!kp.verify(b"wrong data", &sig));
    }

    #[test]
    fn test_sign_verify_wrong_key() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let data = b"test data";
        let sig = kp1.sign(data);
        assert!(!kp2.verify(data, &sig));
    }

    #[test]
    fn test_genesis_deterministic_id() {
        let genesis = GenesisKeyMaterial::generate();
        let id1 = genesis.derive_identity_hash();
        let id2 = genesis.derive_identity_hash();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_different_genesis_different_id() {
        let g1 = GenesisKeyMaterial::generate();
        let g2 = GenesisKeyMaterial::generate();
        assert_ne!(g1.derive_identity_hash(), g2.derive_identity_hash());
    }

    #[test]
    fn test_sign_as_signer() {
        let genesis = GenesisKeyMaterial::generate();
        let signer = genesis.signer_id();
        let data = b"message from worldline";
        let sig = genesis.key_pair.sign_as(data, signer);
        assert_eq!(sig.signer, signer);
        assert!(genesis.key_pair.verify(data, &sig));
    }
}
