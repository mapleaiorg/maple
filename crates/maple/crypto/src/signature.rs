//! Detached Ed25519 signatures.

use serde::{Deserialize, Serialize};

/// A 32-byte identity used to track who produced a signature.
/// This is intentionally a simple byte array — conversion to/from
/// domain-specific ID types (WorldlineId, etc.) is done by consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignerId(pub [u8; 32]);

impl SignerId {
    /// The zero signer (sentinel for unsigned/anonymous).
    pub const ZERO: Self = Self([0u8; 32]);

    /// Create from raw bytes.
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Access the raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Encode as hex string.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl Serialize for SignerId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for SignerId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if s.len() != 64 {
            return Err(serde::de::Error::custom("expected 64 hex chars for SignerId"));
        }
        let mut bytes = [0u8; 32];
        for i in 0..32 {
            bytes[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(serde::de::Error::custom)?;
        }
        Ok(Self(bytes))
    }
}

impl std::fmt::Display for SignerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.to_hex()[..16])
    }
}

/// Detached Ed25519 signature with signer identity.
///
/// # Example
/// ```
/// use maple_crypto::{KeyPair, Signature};
///
/// let kp = KeyPair::generate();
/// let sig = kp.sign(b"test data");
/// assert!(kp.verify(b"test data", &sig));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// The 64-byte Ed25519 signature.
    pub bytes: [u8; 64],
    /// The identity that produced this signature.
    pub signer: SignerId,
}

impl Serialize for Signature {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let hex: String = self.bytes.iter().map(|b| format!("{b:02x}")).collect();
        let helper = SignatureSerHelper {
            bytes: hex,
            signer: self.signer,
        };
        helper.serialize(serializer)
    }
}

#[derive(Serialize)]
struct SignatureSerHelper {
    bytes: String,
    signer: SignerId,
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Helper {
            bytes: String,
            signer: SignerId,
        }

        let helper = Helper::deserialize(deserializer)?;
        if helper.bytes.len() != 128 {
            return Err(serde::de::Error::custom(
                "expected 128 hex chars for signature",
            ));
        }
        let mut arr = [0u8; 64];
        for i in 0..64 {
            arr[i] = u8::from_str_radix(&helper.bytes[i * 2..i * 2 + 2], 16)
                .map_err(serde::de::Error::custom)?;
        }
        Ok(Signature {
            bytes: arr,
            signer: helper.signer,
        })
    }
}
