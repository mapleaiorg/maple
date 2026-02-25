//! Identity and Continuity Types
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdentityRef {
    pub id: String,
    pub continuity_ref: ContinuityRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation: Option<Attestation>,
}

impl IdentityRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            continuity_ref: ContinuityRef::new(),
            attestation: None,
        }
    }
}

impl fmt::Display for IdentityRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Identity({})", &self.id[..8.min(self.id.len())])
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ContinuityRef {
    pub chain: Vec<ContinuityLink>,
    pub current: usize,
}

impl ContinuityRef {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn verify(&self) -> bool {
        true
    }

    pub fn len(&self) -> usize {
        self.chain.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContinuityLink {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(with = "hex_bytes_32")]
    pub state_hash: [u8; 32],
    #[serde(with = "hex_bytes_64")]
    pub signature: [u8; 64],
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Attestation {
    pub attestation_type: AttestationType,
    pub data: Vec<u8>,
    pub verifier: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttestationType {
    Ed25519Signature,
    EcdsaSignature,
    HsmAttestation,
    HumanVerification,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CausalRef {
    pub identity: IdentityRef,
    pub sequence: u64,
}

/// Serde helper for [u8; 32] arrays - serializes as hex string
pub mod hex_bytes_32 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_string: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
        serializer.serialize_str(&hex_string)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes: Vec<u8> = (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<Vec<u8>, _>>()
            .map_err(serde::de::Error::custom)?;

        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("invalid length for [u8; 32]"))
    }
}

/// Serde helper for [u8; 64] arrays - serializes as hex string
pub mod hex_bytes_64 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_string: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
        serializer.serialize_str(&hex_string)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes: Vec<u8> = (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<Vec<u8>, _>>()
            .map_err(serde::de::Error::custom)?;

        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("invalid length for [u8; 64]"))
    }
}
