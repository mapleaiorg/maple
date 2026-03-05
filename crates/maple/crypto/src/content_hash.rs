//! Content-addressed hashing using BLAKE3.

use serde::{Deserialize, Serialize};

use crate::CryptoError;

/// BLAKE3-256 content hash for integrity verification and content addressing.
///
/// # Example
/// ```
/// use maple_crypto::ContentHash;
///
/// let h1 = ContentHash::of(b"hello world");
/// let h2 = ContentHash::of(b"hello world");
/// assert_eq!(h1, h2, "Same input must produce same hash");
///
/// let h3 = ContentHash::of(b"different");
/// assert_ne!(h1, h3);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    /// Hash raw bytes.
    pub fn of(data: &[u8]) -> Self {
        Self(*blake3::hash(data).as_bytes())
    }

    /// Hash a serializable value by computing BLAKE3 over its canonical JSON representation.
    ///
    /// # Example
    /// ```
    /// use maple_crypto::ContentHash;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Foo { x: u32 }
    ///
    /// let h = ContentHash::of_canonical(&Foo { x: 42 }).unwrap();
    /// let h2 = ContentHash::of_canonical(&Foo { x: 42 }).unwrap();
    /// assert_eq!(h, h2);
    /// ```
    pub fn of_canonical<T: serde::Serialize>(value: &T) -> Result<Self, CryptoError> {
        let bytes =
            serde_json::to_vec(value).map_err(|e| CryptoError::Serialization(e.to_string()))?;
        Ok(Self::of(&bytes))
    }

    /// Access raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Encode as hex string.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The zero hash (sentinel).
    pub const ZERO: Self = Self([0u8; 32]);
}

impl std::fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ContentHash({})", &self.to_hex()[..16])
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.to_hex()[..16])
    }
}

impl Serialize for ContentHash {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_hex())
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            if s.len() != 64 {
                return Err(serde::de::Error::custom("expected 64 hex chars"));
            }
            let mut bytes = [0u8; 32];
            for i in 0..32 {
                bytes[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                    .map_err(serde::de::Error::custom)?;
            }
            Ok(Self(bytes))
        } else {
            let bytes = <Vec<u8>>::deserialize(deserializer)?;
            if bytes.len() != 32 {
                return Err(serde::de::Error::custom(format!(
                    "expected 32 bytes, got {}",
                    bytes.len()
                )));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            Ok(Self(arr))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        let a = ContentHash::of(b"deterministic test");
        let b = ContentHash::of(b"deterministic test");
        assert_eq!(a, b);
    }

    #[test]
    fn test_different_inputs() {
        let a = ContentHash::of(b"input a");
        let b = ContentHash::of(b"input b");
        assert_ne!(a, b);
    }

    #[test]
    fn test_canonical_deterministic() {
        #[derive(serde::Serialize)]
        struct TestData {
            x: u32,
            y: String,
        }
        let data = TestData {
            x: 42,
            y: "hello".into(),
        };
        let h1 = ContentHash::of_canonical(&data).unwrap();
        let h2 = ContentHash::of_canonical(&data).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_json_roundtrip() {
        let hash = ContentHash::of(b"json test");
        let json = serde_json::to_string(&hash).unwrap();
        let recovered: ContentHash = serde_json::from_str(&json).unwrap();
        assert_eq!(hash, recovered);
    }
}
