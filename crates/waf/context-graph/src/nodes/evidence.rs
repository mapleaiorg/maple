use crate::types::ContentHash;
use serde::{Deserialize, Serialize};

/// Reference to an evidence bundle stored externally.
/// WLL stores the hash pointer only — full bundle lives in evidence store.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvidenceBundleRef {
    /// Content-addressed hash of the full EvidenceBundle.
    pub bundle_hash: ContentHash,
    /// Number of tests in the bundle.
    pub test_count: usize,
    /// Number of tests that passed.
    pub tests_passed: usize,
    /// Number of invariants checked.
    pub invariants_checked: usize,
    /// Number of invariants that passed.
    pub invariants_passed: usize,
    /// Whether the reproducible build check passed.
    pub repro_build_verified: bool,
    /// Summary string for quick display.
    pub summary: String,
}

impl EvidenceBundleRef {
    pub fn new(bundle_hash: ContentHash) -> Self {
        Self {
            bundle_hash,
            test_count: 0,
            tests_passed: 0,
            invariants_checked: 0,
            invariants_passed: 0,
            repro_build_verified: false,
            summary: String::new(),
        }
    }

    pub fn all_passed(&self) -> bool {
        self.tests_passed == self.test_count
            && self.invariants_passed == self.invariants_checked
            && self.repro_build_verified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_ref_all_passed() {
        let mut r = EvidenceBundleRef::new(ContentHash::hash(b"bundle"));
        r.test_count = 10;
        r.tests_passed = 10;
        r.invariants_checked = 14;
        r.invariants_passed = 14;
        r.repro_build_verified = true;
        assert!(r.all_passed());
    }

    #[test]
    fn evidence_ref_not_all_passed() {
        let mut r = EvidenceBundleRef::new(ContentHash::hash(b"bundle"));
        r.test_count = 10;
        r.tests_passed = 9;
        r.invariants_checked = 14;
        r.invariants_passed = 14;
        r.repro_build_verified = true;
        assert!(!r.all_passed());
    }

    #[test]
    fn evidence_ref_serde() {
        let r = EvidenceBundleRef::new(ContentHash::hash(b"test"));
        let json = serde_json::to_string(&r).unwrap();
        let restored: EvidenceBundleRef = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.bundle_hash, r.bundle_hash);
    }
}
