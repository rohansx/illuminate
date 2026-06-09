//! illuminate-route: Subject-to-file routing and reading plan generation.
//!
//! Given a natural language subject, returns a ranked reading plan
//! combining decision history, code locations, and reflexion episodes.

use serde::{Deserialize, Serialize};

use illuminate::Graph;
use illuminate_embed::EmbedEngine;
use illuminate_index::indexer::CodeIndex;

// FTS5 sanitizer lives in `illuminate-core` (v0.20+). Re-export so existing
// importers of `illuminate_route::sanitize_for_fts5` keep working.
pub use illuminate::sanitize_for_fts5;

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
/// FTS5 sanitization happens inside [`Graph::search`] / [`Graph::search_fused`]
/// (since v0.20) — this function just hands the raw subject through.
///
/// This is the no-index convenience wrapper. It delegates to
/// [`route_with_index`] with no code index, so `FileEntry.symbols` stays empty.
/// Callers that have an [`CodeIndex`] available should call
/// [`route_with_index`] to populate symbols from the code graph.
pub fn route(
    graph: &Graph,
    embed: Option<&EmbedEngine>,
    subject: &str,
    limit: usize,
) -> illuminate::Result<ReadingPlan> {
    route_with_index(graph, embed, None, subject, limit)
}

/// Generate a reading plan, optionally enriching each routed file with the
/// symbols the code graph ([`CodeIndex`]) extracted for that path.
///
/// When `index` is `Some`, every routed [`FileEntry`] has its `symbols` field
/// populated from [`CodeIndex::lookup_file`] (the indexed symbol names for that
/// path). When `index` is `None`, `symbols` stays empty.
///
/// Token estimates are content-aware: see [`estimate_tokens_for_file`].
pub fn route_with_index(
    graph: &Graph,
    embed: Option<&EmbedEngine>,
    index: Option<&CodeIndex>,
    subject: &str,
    limit: usize,
) -> illuminate::Result<ReadingPlan> {
    let mut decisions = Vec::new();

    // Fused search if embeddings available, otherwise FTS5 only.
    // The embedding sees the raw subject (preserves meaning); Graph::search_fused
    // sanitizes internally before the FTS5 path.
    if let Some(embed_engine) = embed
        && let Ok(query_embedding) = embed_engine.embed(subject)
    {
        let results = graph.search_fused(subject, &query_embedding, limit)?;
        for r in results {
            decisions.push(DecisionEntry {
                id: r.episode.id,
                content: r.episode.content,
                source: r.episode.source,
                score: r.score,
            });
        }
    }

    // Fallback to FTS5-only if no fused results.
    if decisions.is_empty() {
        let results = graph.search(subject, limit)?;
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
                    let symbols = symbols_for_path(index, path);
                    code_files.push(FileEntry {
                        path: path.to_string(),
                        symbols,
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

/// Average characters per token for source-code-like content.
///
/// GPT-family BPE tokenizers average roughly 4 characters per token on English
/// prose and source code; we use the same divisor so the estimate is grounded
/// in a documented, real measure rather than a per-extension guess.
const CHARS_PER_TOKEN: usize = 4;

/// Estimate how many tokens reading a file would cost.
///
/// When the file exists and is readable, the estimate is derived from the file's
/// real character count divided by [`CHARS_PER_TOKEN`] — so a large file
/// estimates materially more tokens than a small one, regardless of extension.
/// Empty-but-readable files round up to one token.
///
/// When the file is missing or unreadable (e.g. a path recorded in decision
/// history that no longer exists on disk), we fall back to the per-extension
/// heuristic so the plan still carries a non-zero budget for that file.
fn estimate_tokens_for_file(path: &str) -> usize {
    match std::fs::read_to_string(path) {
        Ok(content) => (content.chars().count() / CHARS_PER_TOKEN).max(1),
        Err(_) => extension_token_heuristic(path),
    }
}

/// Per-extension fallback token estimate, used only when the file content is
/// unavailable. These are coarse averages for a "typical" file of each kind.
fn extension_token_heuristic(path: &str) -> usize {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" | "go" | "java" | "c" | "cpp" => 800,
        "ts" | "tsx" | "js" | "jsx" => 600,
        "py" => 500,
        "toml" | "yaml" | "json" => 200,
        _ => 400,
    }
}

/// Resolve the indexed symbol names for a routed file path from the code graph.
///
/// Returns the names of the symbols [`CodeIndex`] extracted for `path`. When no
/// index is supplied, or the lookup fails, or the path has no indexed symbols,
/// this returns an empty vec — never an error — so routing degrades gracefully.
fn symbols_for_path(index: Option<&CodeIndex>, path: &str) -> Vec<String> {
    let Some(index) = index else {
        return Vec::new();
    };
    match index.lookup_file(path) {
        Ok(symbols) => symbols.into_iter().map(|s| s.name).collect(),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizer_reexport_matches_core() {
        // The symbol is re-exported from `illuminate::sanitize_for_fts5` since
        // v0.20. Verify the re-export still points at the same function so any
        // downstream importer (illuminate-enrich, future crates) keeps working.
        let via_route = sanitize_for_fts5("add Redis caching");
        let via_core = illuminate::sanitize_for_fts5("add Redis caching");
        assert_eq!(via_route, via_core);
    }
}
