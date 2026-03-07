//! MAPLE Guard PII — sensitive data detection and redaction.
//!
//! Provides regex-based PII/secrets detection with configurable
//! redaction strategies (mask, hash, remove, tokenize).

pub mod detector;

pub use detector::{
    Detection, RedactionConfig, RedactionResult, RedactionStrategy,
    SensitiveDataDetector, SensitiveDataType,
};
