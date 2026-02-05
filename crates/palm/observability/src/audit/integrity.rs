//! Integrity verification for audit chains

use super::entry::AuditEntry;
use crate::error::Result;
use sha2::{Digest, Sha256};

/// Manages the integrity chain for audit entries
#[derive(Debug)]
pub struct IntegrityChain {
    /// Hash of the last entry in the chain
    last_hash: Option<String>,

    /// Total number of entries
    entry_count: u64,
}

impl IntegrityChain {
    /// Create a new integrity chain
    pub fn new() -> Self {
        Self {
            last_hash: None,
            entry_count: 0,
        }
    }

    /// Create from existing state
    pub fn from_state(last_hash: Option<String>, entry_count: u64) -> Self {
        Self {
            last_hash,
            entry_count,
        }
    }

    /// Get the previous hash for the next entry
    pub fn previous_hash(&self) -> Option<String> {
        self.last_hash.clone()
    }

    /// Update the chain with a new entry
    pub fn update(&mut self, entry: &AuditEntry) {
        self.last_hash = Some(entry.entry_hash.clone());
        self.entry_count += 1;
    }

    /// Get the entry count
    pub fn entry_count(&self) -> u64 {
        self.entry_count
    }

    /// Get the current chain head hash
    pub fn head_hash(&self) -> Option<&String> {
        self.last_hash.as_ref()
    }
}

impl Default for IntegrityChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Verifies the integrity of audit entries
pub struct IntegrityVerifier;

impl IntegrityVerifier {
    /// Verify a single entry's hash
    pub fn verify_entry(entry: &AuditEntry) -> Result<bool> {
        let computed_hash = Self::compute_hash(entry);
        Ok(computed_hash == entry.entry_hash)
    }

    /// Verify a chain of entries
    pub fn verify_chain(entries: &[AuditEntry]) -> Result<ChainVerificationResult> {
        if entries.is_empty() {
            return Ok(ChainVerificationResult {
                valid: true,
                total_entries: 0,
                verified_entries: 0,
                first_invalid_index: None,
                error_message: None,
            });
        }

        let mut result = ChainVerificationResult {
            valid: true,
            total_entries: entries.len(),
            verified_entries: 0,
            first_invalid_index: None,
            error_message: None,
        };

        for (i, entry) in entries.iter().enumerate() {
            // Verify entry hash
            if !Self::verify_entry(entry)? {
                result.valid = false;
                result.first_invalid_index = Some(i);
                result.error_message = Some(format!("Entry {} has invalid hash", entry.id));
                return Ok(result);
            }

            // Verify chain linkage (skip first entry)
            if i > 0 {
                let expected_prev = &entries[i - 1].entry_hash;
                if entry.previous_hash.as_ref() != Some(expected_prev) {
                    result.valid = false;
                    result.first_invalid_index = Some(i);
                    result.error_message = Some(format!(
                        "Entry {} has broken chain link (expected prev: {}, got: {:?})",
                        entry.id, expected_prev, entry.previous_hash
                    ));
                    return Ok(result);
                }
            } else if entry.previous_hash.is_some() {
                // First entry should have no previous hash (unless it's a continuation)
                // This is not an error, just noted
            }

            result.verified_entries = i + 1;
        }

        Ok(result)
    }

    /// Compute the expected hash for an entry
    fn compute_hash(entry: &AuditEntry) -> String {
        let hash_input = format!(
            "{}{}{}{}{}{}{}{}",
            entry.id,
            entry.timestamp.to_rfc3339(),
            entry.platform,
            serde_json::to_string(&entry.actor).unwrap_or_default(),
            serde_json::to_string(&entry.action).unwrap_or_default(),
            serde_json::to_string(&entry.resource).unwrap_or_default(),
            serde_json::to_string(&entry.outcome).unwrap_or_default(),
            entry.previous_hash.as_deref().unwrap_or("")
        );

        let mut hasher = Sha256::new();
        hasher.update(hash_input.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Detect tampering by comparing hashes
    pub fn detect_tampering(
        entry: &AuditEntry,
        stored_hash: &str,
    ) -> Result<TamperDetectionResult> {
        let computed = Self::compute_hash(entry);
        let stored_matches = stored_hash == entry.entry_hash;
        let computed_matches = computed == entry.entry_hash;

        let result = if computed_matches && stored_matches {
            TamperDetectionResult::NoTampering
        } else if !computed_matches {
            TamperDetectionResult::EntryTampered {
                computed_hash: computed,
                stored_hash: entry.entry_hash.clone(),
            }
        } else {
            TamperDetectionResult::ExternalHashMismatch {
                entry_hash: entry.entry_hash.clone(),
                external_hash: stored_hash.to_string(),
            }
        };

        Ok(result)
    }
}

/// Result of chain verification
#[derive(Debug, Clone)]
pub struct ChainVerificationResult {
    /// Whether the chain is valid
    pub valid: bool,

    /// Total number of entries checked
    pub total_entries: usize,

    /// Number of entries successfully verified
    pub verified_entries: usize,

    /// Index of first invalid entry (if any)
    pub first_invalid_index: Option<usize>,

    /// Error message (if any)
    pub error_message: Option<String>,
}

/// Result of tamper detection
#[derive(Debug, Clone)]
pub enum TamperDetectionResult {
    /// No tampering detected
    NoTampering,

    /// Entry content was tampered
    EntryTampered {
        computed_hash: String,
        stored_hash: String,
    },

    /// External hash doesn't match entry hash
    ExternalHashMismatch {
        entry_hash: String,
        external_hash: String,
    },
}

impl TamperDetectionResult {
    /// Check if no tampering was detected
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::NoTampering)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::entry::{
        AuditAction, AuditActor, AuditEntry as Entry, AuditOutcome, AuditResource,
    };

    fn create_test_entry(previous_hash: Option<String>) -> AuditEntry {
        Entry::builder()
            .platform("development")
            .actor(AuditActor::system("test"))
            .action(AuditAction::SystemStarted)
            .resource(AuditResource::system("test"))
            .outcome(AuditOutcome::success())
            .build()
            .unwrap()
            .finalize(previous_hash)
    }

    #[test]
    fn test_integrity_chain() {
        let mut chain = IntegrityChain::new();
        assert!(chain.previous_hash().is_none());
        assert_eq!(chain.entry_count(), 0);

        let entry1 = create_test_entry(chain.previous_hash());
        chain.update(&entry1);
        assert_eq!(chain.entry_count(), 1);
        assert_eq!(chain.head_hash(), Some(&entry1.entry_hash));

        let entry2 = create_test_entry(chain.previous_hash());
        chain.update(&entry2);
        assert_eq!(chain.entry_count(), 2);
        assert_eq!(chain.head_hash(), Some(&entry2.entry_hash));
    }

    #[test]
    fn test_verify_entry() {
        let entry = create_test_entry(None);
        assert!(IntegrityVerifier::verify_entry(&entry).unwrap());
    }

    #[test]
    fn test_verify_chain() {
        let mut chain = IntegrityChain::new();

        let entry1 = create_test_entry(chain.previous_hash());
        chain.update(&entry1);

        let entry2 = create_test_entry(chain.previous_hash());
        chain.update(&entry2);

        let entry3 = create_test_entry(chain.previous_hash());
        chain.update(&entry3);

        let entries = vec![entry1, entry2, entry3];
        let result = IntegrityVerifier::verify_chain(&entries).unwrap();

        assert!(result.valid);
        assert_eq!(result.total_entries, 3);
        assert_eq!(result.verified_entries, 3);
    }

    #[test]
    fn test_detect_tampering() {
        let entry = create_test_entry(None);
        let result = IntegrityVerifier::detect_tampering(&entry, &entry.entry_hash).unwrap();
        assert!(result.is_valid());

        // Test with wrong hash
        let result = IntegrityVerifier::detect_tampering(&entry, "wrong_hash").unwrap();
        assert!(!result.is_valid());
    }
}
