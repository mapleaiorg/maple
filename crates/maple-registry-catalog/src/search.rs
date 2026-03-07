//! Fuzzy search engine for catalog entries.
//!
//! Uses `fuzzy-matcher`'s Skim V2 algorithm for high-quality fuzzy matching,
//! then combines the fuzzy score with field-specific boosts to produce a
//! unified relevance score.

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::catalog::{CatalogEntry, SearchFilter, SearchResult};

/// The catalog search engine.
///
/// Holds a pre-built fuzzy matcher and provides methods for scoring and
/// filtering catalog entries against a `SearchFilter`.
pub struct SearchEngine {
    matcher: SkimMatcherV2,
}

impl SearchEngine {
    /// Create a new search engine.
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Score and filter a single entry against the filter. Returns `None` when
    /// the entry does not satisfy the filter's hard constraints.
    pub fn score_entry(
        &self,
        entry: &CatalogEntry,
        filter: &SearchFilter,
    ) -> Option<SearchResult> {
        // --- Hard filters (all must pass) ---

        // Kind filter
        if let Some(ref kind) = filter.kind {
            if &entry.kind != kind {
                return None;
            }
        }

        // Org filter: the entry's name must start with "org/"
        if let Some(ref org) = filter.org {
            if !entry.name.starts_with(&format!("{}/", org)) {
                return None;
            }
        }

        // Keyword filter: every requested keyword must be present.
        if !filter.keywords.is_empty() {
            let entry_keywords: Vec<String> = entry
                .metadata
                .keywords
                .iter()
                .map(|k| k.to_lowercase())
                .collect();
            for kw in &filter.keywords {
                if !entry_keywords.contains(&kw.to_lowercase()) {
                    return None;
                }
            }
        }

        // Signed-only filter
        if filter.signed_only && !entry.signed {
            return None;
        }

        // Attested-only filter
        if filter.attested_only && !entry.attested {
            return None;
        }

        // Minimum version filter
        if let Some(ref min_ver) = filter.min_version {
            if &entry.latest_version < min_ver {
                return None;
            }
        }

        // --- Fuzzy scoring (soft match) ---
        let (score, highlights) = if let Some(ref query) = filter.query {
            self.fuzzy_score(entry, query)
        } else {
            // No query => everything matches with a neutral score.
            (1.0, Vec::new())
        };

        // If a query was provided but nothing matched at all, exclude.
        if filter.query.is_some() && score <= 0.0 {
            return None;
        }

        Some(SearchResult {
            entry: entry.clone(),
            score,
            match_highlights: highlights,
        })
    }

    /// Compute a fuzzy relevance score for an entry against a query string.
    ///
    /// Searches across the package name, description, and keyword fields.
    /// Returns a normalized score in [0.0, 1.0] and a list of highlight
    /// descriptions.
    fn fuzzy_score(&self, entry: &CatalogEntry, query: &str) -> (f64, Vec<String>) {
        let mut best_score: i64 = 0;
        let mut highlights = Vec::new();

        // Match against name (highest weight: 3x)
        if let Some(raw) = self.matcher.fuzzy_match(&entry.name, query) {
            let weighted = raw * 3;
            if weighted > best_score {
                best_score = weighted;
            }
            highlights.push(format!("name matched: \"{}\"", entry.name));
        }

        // Match against description (medium weight: 2x)
        if let Some(ref desc) = entry.description {
            if let Some(raw) = self.matcher.fuzzy_match(desc, query) {
                let weighted = raw * 2;
                if weighted > best_score {
                    best_score = weighted;
                }
                highlights.push(format!("description matched: \"{}\"", desc));
            }
        }

        // Match against each keyword (weight: 1x)
        for kw in &entry.metadata.keywords {
            if let Some(raw) = self.matcher.fuzzy_match(kw, query) {
                if raw > best_score {
                    best_score = raw;
                }
                highlights.push(format!("keyword matched: \"{}\"", kw));
            }
        }

        // Match against kind display name (weight: 1x)
        let kind_str = entry.kind.to_string();
        if let Some(raw) = self.matcher.fuzzy_match(&kind_str, query) {
            if raw > best_score {
                best_score = raw;
            }
            highlights.push(format!("kind matched: \"{}\"", kind_str));
        }

        // Normalize to [0.0, 1.0] using a sigmoid-like mapping.
        // SkimV2 scores can vary widely; we clamp to a reasonable range.
        let normalized = if best_score <= 0 {
            0.0
        } else {
            let f = best_score as f64;
            // Map [0, 300+] -> [0, 1.0] using tanh
            (f / 150.0).tanh()
        };

        (normalized, highlights)
    }

    /// Search the given entries with the provided filter.
    ///
    /// Returns results sorted by descending relevance score, with pagination
    /// (offset + limit) applied.
    pub fn search<'a>(
        &self,
        entries: impl Iterator<Item = &'a CatalogEntry>,
        filter: &SearchFilter,
    ) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = entries
            .filter_map(|e| self.score_entry(e, filter))
            .collect();

        // Sort by score descending, then by name ascending for stability.
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.entry.name.cmp(&b.entry.name))
        });

        // Apply offset/limit pagination.
        let offset = filter.offset.unwrap_or(0);
        let limit = filter.limit.unwrap_or(usize::MAX);

        results.into_iter().skip(offset).take(limit).collect()
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}
