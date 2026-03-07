//! Catalog types for MAPLE artifact registry entries.
//!
//! Defines the core data structures used by the catalog layer: entries, search
//! results with relevance scoring, and filter criteria for discovery queries.

use maple_package::{PackageKind, PackageMetadata};
use semver::Version;
use serde::{Deserialize, Serialize};

/// A catalog entry for a package in the registry.
///
/// Represents the metadata that the catalog stores for each published package,
/// including version history, trust indicators, and download counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    /// Fully qualified package name (e.g. "mapleai/agents/customer-support").
    pub name: String,
    /// What kind of artifact this entry represents.
    pub kind: PackageKind,
    /// Human-readable description of the package.
    pub description: Option<String>,
    /// The most recent published version.
    pub latest_version: Version,
    /// All versions available in the registry.
    pub available_versions: Vec<Version>,
    /// Package metadata (authors, keywords, labels, etc.).
    pub metadata: PackageMetadata,
    /// Timestamp when the package was first published.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp of the most recent publish.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Total download count.
    pub downloads: u64,
    /// Whether this package has a valid cryptographic signature.
    pub signed: bool,
    /// Whether this package has a build attestation (SLSA provenance).
    pub attested: bool,
}

/// A search result with relevance scoring and match highlights.
///
/// Returned by catalog search operations to indicate how well a catalog entry
/// matched the query, and which fields contributed to the match.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The matching catalog entry.
    pub entry: CatalogEntry,
    /// Relevance score (higher is better, range 0.0 ..= 1.0).
    pub score: f64,
    /// Human-readable descriptions of which fields matched and why.
    pub match_highlights: Vec<String>,
}

/// Filter criteria for catalog search.
///
/// All fields are optional; when set, they act as AND constraints.
/// An empty filter returns all entries.
#[derive(Debug, Clone, Default)]
pub struct SearchFilter {
    /// Free-text query string for fuzzy matching against name, description,
    /// and keywords.
    pub query: Option<String>,
    /// Restrict results to a specific package kind.
    pub kind: Option<PackageKind>,
    /// Restrict results to packages published by a specific organization.
    pub org: Option<String>,
    /// All listed keywords must appear in the entry's keyword set.
    pub keywords: Vec<String>,
    /// When true, only return packages with a valid signature.
    pub signed_only: bool,
    /// When true, only return packages with a build attestation.
    pub attested_only: bool,
    /// Only return packages whose latest version is >= this value.
    pub min_version: Option<Version>,
    /// Maximum number of results to return.
    pub limit: Option<usize>,
    /// Number of results to skip (for pagination).
    pub offset: Option<usize>,
}

/// Detailed version information for a single published version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// The semantic version.
    pub version: Version,
    /// Content digest of this version's artifact.
    pub digest: String,
    /// When this version was published.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Whether this version is signed.
    pub signed: bool,
}
