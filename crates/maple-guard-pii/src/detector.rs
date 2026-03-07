//! Core PII/secrets detection engine.
//!
//! Provides regex-based detection of sensitive data types including PII,
//! PHI, PCI data, and secrets (API keys, tokens, private keys, etc.).

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Sensitive data type taxonomy
// ---------------------------------------------------------------------------

/// Types of sensitive data that can be detected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitiveDataType {
    // PII
    SocialSecurityNumber,
    Email,
    PhoneNumber,
    CreditCardNumber,
    BankAccountNumber,
    RoutingNumber,
    PassportNumber,
    DriversLicense,
    DateOfBirth,
    FullName,
    Address,
    IpAddress,
    // PHI
    MedicalRecordNumber,
    HealthInsuranceId,
    Diagnosis,
    // Secrets
    ApiKey,
    AwsAccessKey,
    AwsSecretKey,
    GithubToken,
    JwtToken,
    PrivateKey,
    PasswordHash,
    ConnectionString,
    // Custom
    Custom { name: String },
}

impl SensitiveDataType {
    /// A short human-readable label used in redaction placeholders.
    pub fn label(&self) -> &str {
        match self {
            Self::SocialSecurityNumber => "SSN",
            Self::Email => "EMAIL",
            Self::PhoneNumber => "PHONE",
            Self::CreditCardNumber => "CARD",
            Self::BankAccountNumber => "BANK_ACCT",
            Self::RoutingNumber => "ROUTING",
            Self::PassportNumber => "PASSPORT",
            Self::DriversLicense => "DL",
            Self::DateOfBirth => "DOB",
            Self::FullName => "NAME",
            Self::Address => "ADDRESS",
            Self::IpAddress => "IP",
            Self::MedicalRecordNumber => "MRN",
            Self::HealthInsuranceId => "HEALTH_ID",
            Self::Diagnosis => "DIAGNOSIS",
            Self::ApiKey => "API_KEY",
            Self::AwsAccessKey => "AWS_KEY",
            Self::AwsSecretKey => "AWS_SECRET",
            Self::GithubToken => "GITHUB_TOKEN",
            Self::JwtToken => "JWT",
            Self::PrivateKey => "PRIVATE_KEY",
            Self::PasswordHash => "PASSWORD",
            Self::ConnectionString => "CONN_STRING",
            Self::Custom { name } => name,
        }
    }
}

// ---------------------------------------------------------------------------
// Detection result
// ---------------------------------------------------------------------------

/// A single detected sensitive data occurrence.
#[derive(Debug, Clone)]
pub struct Detection {
    /// The kind of sensitive data found.
    pub data_type: SensitiveDataType,
    /// Byte-offset start of the match in the source text.
    pub start: usize,
    /// Byte-offset end (exclusive) of the match in the source text.
    pub end: usize,
    /// Confidence score in `[0.0, 1.0]`.
    pub confidence: f64,
    /// The literal matched text.
    pub matched_text: String,
}

// ---------------------------------------------------------------------------
// Redaction configuration & result
// ---------------------------------------------------------------------------

/// How redacted text should be replaced.
#[derive(Debug, Clone)]
pub enum RedactionStrategy {
    /// Replace with `[SSN]`, `[EMAIL]`, etc.
    Mask,
    /// Replace with `[SSN:a1b2c3d4]` using a BLAKE3 hash prefix.
    Hash,
    /// Remove the matched text entirely.
    Remove,
    /// Replace with a caller-supplied placeholder string.
    Placeholder(String),
}

/// Configuration that controls redaction behaviour.
#[derive(Debug, Clone)]
pub struct RedactionConfig {
    /// The replacement strategy to apply.
    pub strategy: RedactionStrategy,
    /// Only redact detections with confidence >= this threshold.
    pub min_confidence: f64,
    /// If `Some`, only redact these types; if `None`, redact all.
    pub types_to_redact: Option<Vec<SensitiveDataType>>,
}

/// The output of a redaction pass.
#[derive(Debug, Clone)]
pub struct RedactionResult {
    /// The text with sensitive data replaced.
    pub redacted_text: String,
    /// All detections found (sorted by descending start offset).
    pub detections: Vec<Detection>,
    /// How many replacements were actually made.
    pub redaction_count: usize,
}

// ---------------------------------------------------------------------------
// Detector
// ---------------------------------------------------------------------------

/// The PII / secrets detector.
///
/// Holds compiled regex patterns for built-in data types and supports
/// adding custom patterns at runtime.
pub struct SensitiveDataDetector {
    patterns: Vec<(SensitiveDataType, regex::Regex)>,
    custom_patterns: Vec<(String, regex::Regex)>,
}

impl Default for SensitiveDataDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SensitiveDataDetector {
    /// Create a new detector with all built-in patterns.
    pub fn new() -> Self {
        let mut patterns: Vec<(SensitiveDataType, regex::Regex)> = Vec::new();

        // SSN: XXX-XX-XXXX
        patterns.push((
            SensitiveDataType::SocialSecurityNumber,
            regex::Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
        ));

        // Credit Card: 13-19 digits with optional separators
        patterns.push((
            SensitiveDataType::CreditCardNumber,
            regex::Regex::new(r"\b(?:\d[ -]*?){13,19}\b").unwrap(),
        ));

        // Email
        patterns.push((
            SensitiveDataType::Email,
            regex::Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap(),
        ));

        // Phone (US format)
        patterns.push((
            SensitiveDataType::PhoneNumber,
            regex::Regex::new(r"\b(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b")
                .unwrap(),
        ));

        // AWS Access Key
        patterns.push((
            SensitiveDataType::AwsAccessKey,
            regex::Regex::new(r"\b(?:AKIA|ABIA|ACCA|ASIA)[0-9A-Z]{16}\b").unwrap(),
        ));

        // AWS Secret Key (40-char base64-ish)
        patterns.push((
            SensitiveDataType::AwsSecretKey,
            regex::Regex::new(r"\b[0-9a-zA-Z/+]{40}\b").unwrap(),
        ));

        // GitHub Token
        patterns.push((
            SensitiveDataType::GithubToken,
            regex::Regex::new(r"\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,}\b").unwrap(),
        ));

        // JWT Token
        patterns.push((
            SensitiveDataType::JwtToken,
            regex::Regex::new(r"\beyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b")
                .unwrap(),
        ));

        // Private Key header
        patterns.push((
            SensitiveDataType::PrivateKey,
            regex::Regex::new(r"-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap(),
        ));

        // Connection String
        patterns.push((
            SensitiveDataType::ConnectionString,
            regex::Regex::new(r"(?i)(?:postgres|mysql|mongodb|redis)://[^\s]+").unwrap(),
        ));

        // IP Address (v4)
        patterns.push((
            SensitiveDataType::IpAddress,
            regex::Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap(),
        ));

        // Generic API Key (long alphanumeric with common prefix)
        patterns.push((
            SensitiveDataType::ApiKey,
            regex::Regex::new(r"\b(?:sk|pk|api|key|token|secret)[-_][A-Za-z0-9]{20,}\b").unwrap(),
        ));

        Self {
            patterns,
            custom_patterns: Vec::new(),
        }
    }

    /// Register a custom detection pattern at runtime.
    pub fn add_custom_pattern(
        &mut self,
        name: &str,
        pattern: &str,
    ) -> Result<(), regex::Error> {
        let re = regex::Regex::new(pattern)?;
        self.custom_patterns.push((name.to_string(), re));
        Ok(())
    }

    /// Scan `text` and return all detected sensitive data occurrences.
    pub fn detect(&self, text: &str) -> Vec<Detection> {
        let mut detections = Vec::new();

        for (data_type, re) in &self.patterns {
            for mat in re.find_iter(text) {
                let confidence = match data_type {
                    SensitiveDataType::CreditCardNumber => {
                        let digits: String =
                            mat.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
                        if luhn_check(&digits) {
                            0.95
                        } else {
                            0.3
                        }
                    }
                    SensitiveDataType::IpAddress => {
                        let valid = mat
                            .as_str()
                            .split('.')
                            .all(|o| o.parse::<u16>().map(|n| n <= 255).unwrap_or(false));
                        if valid {
                            0.7
                        } else {
                            0.1
                        }
                    }
                    _ => 0.9,
                };

                if confidence >= 0.5 {
                    detections.push(Detection {
                        data_type: data_type.clone(),
                        start: mat.start(),
                        end: mat.end(),
                        confidence,
                        matched_text: mat.as_str().to_string(),
                    });
                }
            }
        }

        // Custom patterns
        for (name, re) in &self.custom_patterns {
            for mat in re.find_iter(text) {
                detections.push(Detection {
                    data_type: SensitiveDataType::Custom { name: name.clone() },
                    start: mat.start(),
                    end: mat.end(),
                    confidence: 0.85,
                    matched_text: mat.as_str().to_string(),
                });
            }
        }

        detections
    }

    /// Detect and redact sensitive data from `text` according to `config`.
    pub fn redact(&self, text: &str, config: &RedactionConfig) -> RedactionResult {
        let detections = self.detect(text);
        if detections.is_empty() {
            return RedactionResult {
                redacted_text: text.to_string(),
                detections: vec![],
                redaction_count: 0,
            };
        }

        let mut redacted = text.to_string();
        // Sort by descending start so replacements don't shift earlier indices.
        let mut sorted = detections;
        sorted.sort_by(|a, b| b.start.cmp(&a.start));

        let mut count = 0;
        for detection in &sorted {
            if detection.confidence < config.min_confidence {
                continue;
            }

            // If a type filter is specified, skip types not in the list.
            if let Some(ref allowed) = config.types_to_redact {
                if !allowed.contains(&detection.data_type) {
                    continue;
                }
            }

            let replacement = match config.strategy {
                RedactionStrategy::Mask => {
                    format!("[{}]", detection.data_type.label())
                }
                RedactionStrategy::Hash => {
                    let hash = blake3::hash(detection.matched_text.as_bytes());
                    format!("[{}:{}]", detection.data_type.label(), &hash.to_hex()[..8])
                }
                RedactionStrategy::Remove => String::new(),
                RedactionStrategy::Placeholder(ref ph) => ph.clone(),
            };
            redacted.replace_range(detection.start..detection.end, &replacement);
            count += 1;
        }

        RedactionResult {
            redacted_text: redacted,
            detections: sorted,
            redaction_count: count,
        }
    }
}

// ---------------------------------------------------------------------------
// Luhn algorithm
// ---------------------------------------------------------------------------

/// Luhn algorithm for credit card number validation.
fn luhn_check(digits: &str) -> bool {
    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let sum: u32 = digits
        .chars()
        .rev()
        .enumerate()
        .map(|(i, c)| {
            let mut d = c.to_digit(10).unwrap_or(0);
            if i % 2 == 1 {
                d *= 2;
                if d > 9 {
                    d -= 9;
                }
            }
            d
        })
        .sum();
    sum % 10 == 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn detector() -> SensitiveDataDetector {
        SensitiveDataDetector::new()
    }

    // 1. Detect SSN
    #[test]
    fn detect_ssn() {
        let d = detector();
        let hits = d.detect("My SSN is 123-45-6789 thanks");
        assert!(
            hits.iter()
                .any(|h| h.data_type == SensitiveDataType::SocialSecurityNumber
                    && h.matched_text == "123-45-6789"),
            "expected SSN detection, got: {hits:?}"
        );
    }

    // 2a. Detect credit card with valid Luhn
    #[test]
    fn detect_credit_card_valid_luhn() {
        let d = detector();
        let hits = d.detect("Card: 4111111111111111 end");
        let card_hits: Vec<_> = hits
            .iter()
            .filter(|h| h.data_type == SensitiveDataType::CreditCardNumber)
            .collect();
        assert!(!card_hits.is_empty(), "expected card detection");
        assert!(
            card_hits[0].confidence > 0.9,
            "expected high confidence for valid Luhn, got {}",
            card_hits[0].confidence
        );
    }

    // 2b. Credit card with invalid Luhn gets low confidence (filtered out)
    #[test]
    fn detect_credit_card_invalid_luhn() {
        let d = detector();
        let hits = d.detect("Card: 1234567890123456 end");
        // Invalid Luhn -> confidence 0.3 -> filtered at threshold 0.5
        let card_hits: Vec<_> = hits
            .iter()
            .filter(|h| h.data_type == SensitiveDataType::CreditCardNumber)
            .collect();
        assert!(
            card_hits.is_empty(),
            "expected no high-confidence card detection for invalid Luhn"
        );
    }

    // 3. Detect email
    #[test]
    fn detect_email() {
        let d = detector();
        let hits = d.detect("Send to user@example.com please");
        assert!(
            hits.iter()
                .any(|h| h.data_type == SensitiveDataType::Email
                    && h.matched_text == "user@example.com"),
            "expected email detection"
        );
    }

    // 4. Detect AWS Access Key
    #[test]
    fn detect_aws_access_key() {
        let d = detector();
        let hits = d.detect("key=AKIAIOSFODNN7EXAMPLE end");
        assert!(
            hits.iter()
                .any(|h| h.data_type == SensitiveDataType::AwsAccessKey
                    && h.matched_text == "AKIAIOSFODNN7EXAMPLE"),
            "expected AWS access key detection, got: {hits:?}"
        );
    }

    // 5. Detect GitHub token
    #[test]
    fn detect_github_token() {
        let d = detector();
        let token = "ghp_ABCdef123456789012345678901234567890";
        let text = format!("token={token} end");
        let hits = d.detect(&text);
        assert!(
            hits.iter()
                .any(|h| h.data_type == SensitiveDataType::GithubToken),
            "expected GitHub token detection, got: {hits:?}"
        );
    }

    // 6. Detect private key header
    #[test]
    fn detect_private_key_header() {
        let d = detector();
        let hits = d.detect("-----BEGIN RSA PRIVATE KEY-----\nMIIE...");
        assert!(
            hits.iter()
                .any(|h| h.data_type == SensitiveDataType::PrivateKey),
            "expected private key detection"
        );
    }

    // 7. Redact with Mask strategy
    #[test]
    fn redact_mask_strategy() {
        let d = detector();
        let config = RedactionConfig {
            strategy: RedactionStrategy::Mask,
            min_confidence: 0.5,
            types_to_redact: None,
        };
        let result = d.redact("SSN: 123-45-6789", &config);
        assert!(
            result.redacted_text.contains("[SSN]"),
            "expected [SSN] in redacted text, got: {}",
            result.redacted_text
        );
        assert!(!result.redacted_text.contains("123-45-6789"));
        assert!(result.redaction_count >= 1);
    }

    // 8. Redact with Hash strategy
    #[test]
    fn redact_hash_strategy() {
        let d = detector();
        let config = RedactionConfig {
            strategy: RedactionStrategy::Hash,
            min_confidence: 0.5,
            types_to_redact: None,
        };
        let result = d.redact("SSN: 123-45-6789", &config);
        assert!(
            result.redacted_text.contains("[SSN:"),
            "expected [SSN:...] in redacted text, got: {}",
            result.redacted_text
        );
        assert!(!result.redacted_text.contains("123-45-6789"));
    }

    // 9. Multiple detections in single text
    #[test]
    fn detect_multiple_in_single_text() {
        let d = detector();
        let text = "SSN: 123-45-6789, email: admin@corp.io, IP: 10.0.0.1";
        let hits = d.detect(text);
        let types: Vec<_> = hits.iter().map(|h| &h.data_type).collect();
        assert!(
            types.contains(&&SensitiveDataType::SocialSecurityNumber),
            "missing SSN"
        );
        assert!(types.contains(&&SensitiveDataType::Email), "missing email");
        assert!(
            types.contains(&&SensitiveDataType::IpAddress),
            "missing IP"
        );
    }

    // 10. No false positives on innocuous text
    #[test]
    fn no_false_positives_on_normal_text() {
        let d = detector();
        let hits = d.detect("The quick brown fox jumps over the lazy dog.");
        assert!(hits.is_empty(), "expected zero detections, got: {hits:?}");
    }

    // 11. Custom pattern detection
    #[test]
    fn custom_pattern_detection() {
        let mut d = detector();
        d.add_custom_pattern("INTERNAL_ID", r"\bINT-\d{6}\b")
            .unwrap();
        let hits = d.detect("Ref: INT-004521 done");
        assert!(
            hits.iter().any(|h| matches!(
                &h.data_type,
                SensitiveDataType::Custom { name } if name == "INTERNAL_ID"
            )),
            "expected custom pattern detection"
        );
    }

    // 12. Min-confidence filtering excludes low-confidence matches
    #[test]
    fn min_confidence_filtering() {
        let d = detector();
        // IP "999.999.999.999" has invalid octets -> confidence 0.1 -> filtered
        let hits = d.detect("addr 999.999.999.999 end");
        let ip_hits: Vec<_> = hits
            .iter()
            .filter(|h| h.data_type == SensitiveDataType::IpAddress)
            .collect();
        assert!(
            ip_hits.is_empty(),
            "invalid IP should be filtered out by confidence threshold"
        );
    }

    // 13. Detect phone number
    #[test]
    fn detect_phone_number() {
        let d = detector();
        let hits = d.detect("Call me at (555) 123-4567 please");
        assert!(
            hits.iter()
                .any(|h| h.data_type == SensitiveDataType::PhoneNumber),
            "expected phone number detection, got: {hits:?}"
        );
    }

    // 14. Detect connection string
    #[test]
    fn detect_connection_string() {
        let d = detector();
        let hits = d.detect("db=postgres://user:pass@host:5432/mydb end");
        assert!(
            hits.iter()
                .any(|h| h.data_type == SensitiveDataType::ConnectionString),
            "expected connection string detection, got: {hits:?}"
        );
    }

    // 15. Detect generic API key
    #[test]
    fn detect_generic_api_key() {
        let d = detector();
        let hits = d.detect("header: sk-proj1234567890abcdefghij end");
        assert!(
            hits.iter()
                .any(|h| h.data_type == SensitiveDataType::ApiKey),
            "expected API key detection, got: {hits:?}"
        );
    }

    // 16. Redact with Remove strategy
    #[test]
    fn redact_remove_strategy() {
        let d = detector();
        let config = RedactionConfig {
            strategy: RedactionStrategy::Remove,
            min_confidence: 0.5,
            types_to_redact: None,
        };
        let result = d.redact("SSN: 123-45-6789 end", &config);
        assert!(
            !result.redacted_text.contains("123-45-6789"),
            "SSN should be removed"
        );
        assert!(
            result.redacted_text.contains("SSN:"),
            "non-sensitive prefix should remain"
        );
    }

    // 17. Redact with Placeholder strategy
    #[test]
    fn redact_placeholder_strategy() {
        let d = detector();
        let config = RedactionConfig {
            strategy: RedactionStrategy::Placeholder("***REDACTED***".to_string()),
            min_confidence: 0.5,
            types_to_redact: None,
        };
        let result = d.redact("SSN: 123-45-6789 end", &config);
        assert!(
            result.redacted_text.contains("***REDACTED***"),
            "expected placeholder in output, got: {}",
            result.redacted_text
        );
    }

    // 18. types_to_redact filter
    #[test]
    fn types_to_redact_filter() {
        let d = detector();
        let config = RedactionConfig {
            strategy: RedactionStrategy::Mask,
            min_confidence: 0.5,
            types_to_redact: Some(vec![SensitiveDataType::Email]),
        };
        let result = d.redact(
            "SSN: 123-45-6789, email: admin@corp.io",
            &config,
        );
        // SSN should NOT be redacted because only Email is in the filter list.
        assert!(
            result.redacted_text.contains("123-45-6789"),
            "SSN should remain when only Email is in types_to_redact"
        );
        assert!(
            result.redacted_text.contains("[EMAIL]"),
            "Email should be redacted"
        );
    }

    // 19. Detect JWT token
    #[test]
    fn detect_jwt_token() {
        let d = detector();
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abc123def456";
        let text = format!("auth: {jwt} done");
        let hits = d.detect(&text);
        assert!(
            hits.iter()
                .any(|h| h.data_type == SensitiveDataType::JwtToken),
            "expected JWT detection, got: {hits:?}"
        );
    }
}
