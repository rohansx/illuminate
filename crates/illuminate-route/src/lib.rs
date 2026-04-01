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
pub fn route(
    graph: &Graph,
    embed: Option<&EmbedEngine>,
    subject: &str,
    limit: usize,
) -> illuminate::Result<ReadingPlan> {
    let mut decisions = Vec::new();

    // Fused search if embeddings available, otherwise FTS5 only
    if let Some(embed_engine) = embed
        && let Ok(query_embedding) = embed_engine.embed(subject) {
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

    // Fallback to FTS5-only if no fused results
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
                && let Some(files) = meta.get("files_changed").and_then(|v| v.as_array()) {
                    for file in files {
                        if let Some(path) = file.as_str()
                            && !code_files.iter().any(|f: &FileEntry| f.path == path) {
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
