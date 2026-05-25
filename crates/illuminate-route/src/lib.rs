//! illuminate-route: Subject-to-file routing and reading plan generation.
//!
//! Given a natural language subject, returns a ranked reading plan
//! combining decision history, code locations, and reflexion episodes.

use serde::{Deserialize, Serialize};

use illuminate::Graph;
use illuminate_embed::EmbedEngine;

/// A ranked reading plan for a subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingPlan {
    pub decisions: Vec<DecisionEntry>,
    pub code_files: Vec<FileEntry>,
    pub estimated_tokens: usize,
}

/// A decision relevant to the subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEntry {
    pub id: String,
    pub content: String,
    pub source: Option<String>,
    pub score: f64,
}

/// A code file relevant to the subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub symbols: Vec<String>,
    pub priority: u8,
    pub estimated_tokens: usize,
}

/// Generate a reading plan for a subject using the decision graph.
///
/// Uses RRF (Reciprocal Rank Fusion) across FTS5 and semantic search.
/// The raw `subject` is used for embedding (preserves meaning); FTS5 paths
/// use [`sanitize_for_fts5`] to strip operator characters and stopwords
/// before the MATCH clause.
pub fn route(
    graph: &Graph,
    embed: Option<&EmbedEngine>,
    subject: &str,
    limit: usize,
) -> illuminate::Result<ReadingPlan> {
    let mut decisions = Vec::new();

    // Sanitize for FTS5 — strips `/`, `:`, `*`, stopwords; OR-joins the rest.
    // Empty result means no usable search terms (return empty plan early).
    let fts_query = sanitize_for_fts5(subject);
    if fts_query.is_empty() {
        return Ok(ReadingPlan {
            decisions,
            code_files: Vec::new(),
            estimated_tokens: 0,
        });
    }

    // Fused search if embeddings available, otherwise FTS5 only.
    // The embedding still sees the raw `subject` so meaning is preserved.
    if let Some(embed_engine) = embed
        && let Ok(query_embedding) = embed_engine.embed(subject)
    {
        let results = graph.search_fused(&fts_query, &query_embedding, limit)?;
        for r in results {
            decisions.push(DecisionEntry {
                id: r.episode.id,
                content: r.episode.content,
                source: r.episode.source,
                score: r.score,
            });
        }
    }

    // Fallback to FTS5-only if no fused results
    if decisions.is_empty() {
        let results = graph.search(&fts_query, limit)?;
        for (episode, score) in results {
            decisions.push(DecisionEntry {
                id: episode.id,
                content: episode.content,
                source: episode.source,
                score,
            });
        }
    }

    // Extract file paths from decision metadata
    let mut code_files = Vec::new();
    for decision in &decisions {
        // Check if decision metadata contains files_changed
        if let Ok(Some(ep)) = graph.get_episode(&decision.id)
            && let Some(ref meta) = ep.metadata
            && let Some(files) = meta.get("files_changed").and_then(|v| v.as_array())
        {
            for file in files {
                if let Some(path) = file.as_str()
                    && !code_files.iter().any(|f: &FileEntry| f.path == path)
                {
                    let estimated_tokens = estimate_tokens_for_file(path);
                    code_files.push(FileEntry {
                        path: path.to_string(),
                        symbols: Vec::new(),
                        priority: 2,
                        estimated_tokens,
                    });
                }
            }
        }
    }

    let total_tokens: usize = decisions.len() * 100 // ~100 tokens per decision
        + code_files.iter().map(|f| f.estimated_tokens).sum::<usize>();

    Ok(ReadingPlan {
        decisions,
        code_files,
        estimated_tokens: total_tokens,
    })
}

/// Turn a free-form prompt or subject into a safe FTS5 query.
///
/// FTS5 has reserved characters (`/`, `:`, `*`, `"`, parens, etc.) and AND-joins
/// whitespace tokens by default — both wrong for natural language. We extract
/// alphanumeric tokens ≥ 3 chars, drop common stopwords, lowercase, dedup,
/// and OR them together. Empty result means no usable search terms.
///
/// Originally lived in `illuminate-enrich`; promoted here in v0.19 so audit /
/// search / MCP all benefit from the same fix.
pub fn sanitize_for_fts5(text: &str) -> String {
    const STOPWORDS: &[&str] = &[
        "the", "and", "for", "with", "from", "into", "that", "this", "have", "has", "had", "but",
        "not", "are", "was", "were", "been", "you", "your", "our", "all", "any", "add", "use",
        "new", "out", "via", "per", "let", "set", "get", "can", "may", "now", "yes", "ado", "off",
        "one", "two",
    ];
    let mut seen: std::collections::BTreeSet<String> = Default::default();
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

/// Rough token estimate for a file based on path.
fn estimate_tokens_for_file(path: &str) -> usize {
    // Rough estimate: average code file is ~200 lines, ~4 chars/token
    // This is a placeholder — real implementation would stat the file
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" | "go" | "java" | "c" | "cpp" => 800,
        "ts" | "tsx" | "js" | "jsx" => 600,
        "py" => 500,
        "toml" | "yaml" | "json" => 200,
        _ => 400,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizer_strips_fts5_operator_chars() {
        // The exact characters that broke audit + route in v0.18: `/`, `:`, `<`,
        // `*`, parens, quotes. None should appear in the output.
        let q = sanitize_for_fts5("add Redis caching to src/payments/txn.rs (urgent!)");
        for bad in ['/', ':', '<', '>', '*', '"', '(', ')', '!'] {
            assert!(!q.contains(bad), "found operator char {bad:?} in: {q}");
        }
        assert!(q.contains("redis"));
        assert!(q.contains("caching"));
        assert!(q.contains("payments"));
        assert!(q.contains(" OR "));
    }

    #[test]
    fn sanitizer_drops_stopwords_and_short_tokens() {
        let q = sanitize_for_fts5("add the new for any can may");
        assert_eq!(q, "", "all tokens were stopwords; got: {q}");
    }

    #[test]
    fn sanitizer_lowercases_and_dedups() {
        let q = sanitize_for_fts5("Redis REDIS redis caching CACHING");
        // BTreeSet dedup → deterministic order: redis appears once, caching once.
        let parts: Vec<&str> = q.split(" OR ").collect();
        assert!(parts.contains(&"redis"));
        assert!(parts.contains(&"caching"));
        assert_eq!(parts.iter().filter(|p| **p == "redis").count(), 1);
    }

    #[test]
    fn sanitizer_handles_empty_and_garbage() {
        assert_eq!(sanitize_for_fts5(""), "");
        assert_eq!(sanitize_for_fts5("///:::***"), "");
        assert_eq!(sanitize_for_fts5("a b c d e"), ""); // all < 3 chars
    }
}
