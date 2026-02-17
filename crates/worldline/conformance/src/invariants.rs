//! Constitutional invariant definitions and verification functions.
//!
//! Each invariant is a function that returns Ok(()) if satisfied,
//! or Err with a description if violated.

use std::fmt;

/// Result of checking a single invariant.
#[derive(Clone, Debug)]
pub struct InvariantResult {
    /// Invariant identifier (e.g., "I.1", "I.S-1")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Whether the invariant holds
    pub passed: bool,
    /// Description of what was checked
    pub description: String,
    /// Details (error message if failed)
    pub details: Option<String>,
}

impl InvariantResult {
    pub fn pass(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            passed: true,
            description: description.into(),
            details: None,
        }
    }

    pub fn fail(id: &str, name: &str, description: &str, details: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            passed: false,
            description: description.into(),
            details: Some(details.into()),
        }
    }
}

impl fmt::Display for InvariantResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.passed { "PASS" } else { "FAIL" };
        write!(
            f,
            "[{}] {} — {}: {}",
            status, self.id, self.name, self.description
        )?;
        if let Some(ref details) = self.details {
            write!(f, " ({})", details)?;
        }
        Ok(())
    }
}

/// Constitutional invariant IDs covered by this suite.
pub const ALL_INVARIANT_IDS: &[&str] = &[
    "I.1",
    "I.2",
    "I.3",
    "I.4",
    "I.5",
    "I.6",
    "I.7",
    "I.8",
    "I.9",
    "I.MRP-1",
    "I.CG-1",
    "I.AAS-3",
    "I.PVP-1",
    "I.GCP-2",
    "I.PROF-1",
    "I.S-1",
    "I.S-2",
    "I.S-3",
    "I.S-4",
    "I.S-5",
    "I.S-BOUND",
    "I.WLP-1",
    "I.EF-1",
    "I.ME-FIN-1",
    "I.CEP-FIN-1",
    "I.PROF-2",
    "I.PROF-3",
];
