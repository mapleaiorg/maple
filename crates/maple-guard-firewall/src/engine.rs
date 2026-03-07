//! Glob matching engine for capability patterns.
//!
//! Supports `*` (match any segment characters) and `**` (match across `.` separators).

/// Match a glob pattern against a value using `.` as the segment separator.
///
/// - `*` matches any characters within a single segment (no `.` crossing).
/// - `**` matches any characters across segment boundaries (including `.`).
/// - All other characters are matched literally.
///
/// # Examples
/// ```
/// use maple_guard_firewall::engine::glob_match;
/// assert!(glob_match("zendesk.*", "zendesk.ticket"));
/// assert!(!glob_match("zendesk.*", "zendesk.ticket.read"));
/// assert!(glob_match("zendesk.**", "zendesk.ticket.read"));
/// assert!(glob_match("*", "anything"));
/// assert!(glob_match("**", "a.b.c"));
/// ```
pub fn glob_match(pattern: &str, value: &str) -> bool {
    glob_match_inner(pattern.as_bytes(), value.as_bytes())
}

fn glob_match_inner(pattern: &[u8], value: &[u8]) -> bool {
    let mut pi = 0;
    let mut vi = 0;

    // Track backtrack points for `**`
    let mut star_pi: Option<usize> = None;
    let mut star_vi: Option<usize> = None;

    // Track backtrack points for single `*`
    let mut single_star_pi: Option<usize> = None;
    let mut single_star_vi: Option<usize> = None;

    while vi < value.len() || pi < pattern.len() {
        if pi < pattern.len() {
            // Check for `**`
            if pi + 1 < pattern.len() && pattern[pi] == b'*' && pattern[pi + 1] == b'*' {
                // `**` matches everything including `.`
                star_pi = Some(pi);
                star_vi = Some(vi);
                pi += 2;
                // Skip trailing `.` after `**` if present
                if pi < pattern.len() && pattern[pi] == b'.' {
                    pi += 1;
                }
                // Reset single-star tracking when we find a double-star
                single_star_pi = None;
                single_star_vi = None;
                continue;
            }

            // Check for single `*`
            if pattern[pi] == b'*' {
                single_star_pi = Some(pi);
                single_star_vi = Some(vi);
                pi += 1;
                continue;
            }

            // Literal match
            if vi < value.len() && pattern[pi] == value[vi] {
                pi += 1;
                vi += 1;
                continue;
            }
        }

        // Try backtracking on single `*` (cannot cross `.`)
        if let (Some(sp), Some(sv)) = (single_star_pi, single_star_vi) {
            if sv < value.len() && value[sv] != b'.' {
                let new_sv = sv + 1;
                single_star_vi = Some(new_sv);
                pi = sp + 1;
                vi = new_sv;
                continue;
            }
            // Single star exhausted or hit a `.`; clear it and try double-star
            single_star_pi = None;
            single_star_vi = None;
        }

        // Try backtracking on `**` (can cross anything)
        if let (Some(sp), Some(sv)) = (star_pi, star_vi) {
            let new_sv = sv + 1;
            if new_sv <= value.len() {
                star_vi = Some(new_sv);
                pi = sp + 2;
                // Skip trailing `.` after `**`
                if pi < pattern.len() && pattern[pi] == b'.' {
                    pi += 1;
                }
                vi = new_sv;
                continue;
            }
        }

        return false;
    }

    true
}

/// Path-aware glob matching (delegates to [`glob_match`]).
pub fn glob_match_path(pattern: &str, path: &str) -> bool {
    glob_match(pattern, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(glob_match("hello", "hello"));
        assert!(!glob_match("hello", "world"));
    }

    #[test]
    fn test_single_star_within_segment() {
        assert!(glob_match("zendesk.*", "zendesk.ticket"));
        assert!(glob_match("zendesk.*", "zendesk.read"));
        // Single star should NOT cross `.`
        assert!(!glob_match("zendesk.*", "zendesk.ticket.read"));
    }

    #[test]
    fn test_double_star_across_segments() {
        assert!(glob_match("zendesk.**", "zendesk.ticket.read"));
        assert!(glob_match("zendesk.**", "zendesk.anything"));
        assert!(glob_match("**", "a.b.c.d"));
    }

    #[test]
    fn test_wildcard_all() {
        assert!(glob_match("*", "hello"));
        assert!(!glob_match("*", "hello.world"));
    }

    #[test]
    fn test_mixed_patterns() {
        assert!(glob_match("banking.*", "banking.payment"));
        assert!(glob_match("banking.**", "banking.payment.prepare"));
        assert!(!glob_match("banking.*", "banking.payment.prepare"));
    }

    #[test]
    fn test_resource_paths() {
        // Resource path matching uses `.` as segment separator
        assert!(glob_match("tickets.**", "tickets.team-a.123"));
        assert!(glob_match("tickets.*", "tickets.team-a"));
        assert!(!glob_match("tickets.*", "tickets.team-a.123"));
    }

    #[test]
    fn test_empty() {
        assert!(glob_match("", ""));
        assert!(!glob_match("", "a"));
        assert!(!glob_match("a", ""));
    }
}
