//! Query-string utilities for the graph layer.
//!
//! FTS5 has reserved characters (`/`, `:`, `*`, `"`, parens, etc.) and AND-joins
//! whitespace tokens by default — both are wrong when callers pass free-form
//! natural-language queries. [`sanitize_for_fts5`] normalizes the input into
//! a safe OR-query over meaningful tokens.
//!
//! Promoted into `illuminate-core` in v0.20 from `illuminate-route` so that
//! [`crate::Graph::search`], [`crate::Graph::search_entities`], and
//! [`crate::Graph::search_fused`] can all apply it transparently for every
//! caller (audit, route, MCP, dashboard, CLI — none have to remember to
//! sanitize themselves).

use std::collections::BTreeSet;

/// Stopwords filtered out before joining the FTS5 query.
///
/// These are the words that fire 30+ times in any English prompt and add no
/// search signal. Keeping the list short is intentional — aggressive filtering
/// drops too many real matches.
const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "from", "into", "that", "this", "have", "has", "had", "but",
    "not", "are", "was", "were", "been", "you", "your", "our", "all", "any", "add", "use", "new",
    "out", "via", "per", "let", "set", "get", "can", "may", "now", "yes", "ado", "off", "one",
    "two",
];

/// Turn a free-form prompt or subject into a safe FTS5 query.
///
/// - Splits on anything non-alphanumeric (so `/`, `:`, `*`, `"`, parens, etc.
///   become delimiters instead of operators).
/// - Drops tokens shorter than 3 ASCII chars (typically articles/abbreviations
///   that produce noise).
/// - Drops stopwords ([`STOPWORDS`]).
/// - Lowercases everything; deduplicates via [`BTreeSet`] for stable ordering.
/// - Joins the survivors with ` OR ` so episodes containing *any* meaningful
///   word match — closer to what users expect from a natural-language query
///   than the default AND-of-tokens behaviour.
///
/// Returns the empty string if no usable tokens remain. Graph search methods
/// short-circuit to an empty result set when given an empty query so we never
/// pass `""` to a `MATCH` clause (which fails with a syntax error).
pub fn sanitize_for_fts5(text: &str) -> String {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut tokens: Vec<String> = Vec::new();
    for raw in text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
        if raw.len() < 3 {
            continue;
        }
        let lower = raw.to_ascii_lowercase();
        if STOPWORDS.contains(&lower.as_str()) {
            continue;
        }
        if seen.insert(lower.clone()) {
            tokens.push(lower);
        }
    }
    tokens.join(" OR ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_fts5_operator_characters() {
        // These are the exact characters that crashed the audit MCP tool in
        // v0.18 with `fts5: syntax error near "/"`. None should survive.
        let q = sanitize_for_fts5("add Redis caching to src/payments/txn.rs (urgent!)");
        for bad in ['/', ':', '<', '>', '*', '"', '(', ')', '!', '?'] {
            assert!(!q.contains(bad), "operator char {bad:?} in: {q}");
        }
        assert!(q.contains("redis"));
        assert!(q.contains("caching"));
        assert!(q.contains("payments"));
        assert!(q.contains(" OR "));
    }

    #[test]
    fn drops_stopwords_and_short_tokens() {
        assert_eq!(sanitize_for_fts5("add the new for any can may"), "");
        assert_eq!(sanitize_for_fts5("a b c d e"), "");
    }

    #[test]
    fn lowercases_and_dedups() {
        let q = sanitize_for_fts5("Redis REDIS redis caching CACHING");
        let parts: Vec<&str> = q.split(" OR ").collect();
        assert_eq!(parts.iter().filter(|p| **p == "redis").count(), 1);
        assert_eq!(parts.iter().filter(|p| **p == "caching").count(), 1);
    }

    #[test]
    fn handles_empty_and_garbage() {
        assert_eq!(sanitize_for_fts5(""), "");
        assert_eq!(sanitize_for_fts5("///:::***"), "");
    }

    #[test]
    fn preserves_underscores_inside_identifiers() {
        // Snake-case identifiers should survive as single tokens, not split.
        let q = sanitize_for_fts5("refactor process_payment in cache_layer");
        assert!(q.contains("process_payment"), "got: {q}");
        assert!(q.contains("cache_layer"), "got: {q}");
    }
}
