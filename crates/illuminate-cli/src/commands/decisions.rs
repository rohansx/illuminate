use std::path::PathBuf;

use crate::display;
use serde::Serialize;

use super::open_graph;

pub fn list(
    _after: Option<String>,
    _source: Option<String>,
    limit: usize,
) -> illuminate::Result<()> {
    let graph = open_graph()?;
    let episodes = graph.list_episodes(limit, 0)?;

    if episodes.is_empty() {
        println!("No episodes found. Log some decisions first:");
        println!("  illuminate log \"Chose Postgres for billing service\"");
        return Ok(());
    }

    println!("{:<12} {:<12} CONTENT", "ID", "SOURCE");
    println!("{}", "-".repeat(70));

    for episode in &episodes {
        display::print_episode_row(episode);
    }

    println!("\n{} episodes total", episodes.len());

    Ok(())
}

pub fn show(id: String) -> illuminate::Result<()> {
    let graph = open_graph()?;

    let episode = graph.get_episode(&id)?;
    let Some(episode) = episode else {
        return Err(illuminate::IlluminateError::NotFound(format!(
            "episode '{id}'"
        )));
    };

    display::print_episode(&episode, None);

    Ok(())
}

/// Cap on the preview length when rendering decisions for human output.
/// Each entry is a single line (`<id> [<source>] <recorded_at>: <preview>`),
/// so a fixed cap keeps long episode contents from line-wrapping.
const PREVIEW_CHARS: usize = 100;

/// Hard cap on results — matches the MCP `decisions_for` tool so the CLI
/// view doesn't surprise users with a different result set.
const QUERY_LIMIT: usize = 20;

/// Run `illuminate decisions for <PATH>`.
///
/// Thin pass-through over `Graph::search` — the same wiring the MCP
/// `illuminate_decisions_for` tool uses (see
/// `crates/illuminate-mcp/src/tools.rs`). Wrapping the path in FTS5 phrase
/// quotes is required: bare `src/payments` triggers the FTS5 parser on the
/// `/` and raises a syntax error.
pub fn for_path(path: PathBuf, json: bool) -> illuminate::Result<()> {
    let graph = open_graph()?;
    let path_str = path.to_string_lossy().to_string();
    let query = fts5_phrase_query(&path_str);
    let results = graph.search(&query, QUERY_LIMIT)?;

    if json {
        let payload: Vec<DecisionRow> = results
            .iter()
            .map(|(ep, score)| DecisionRow {
                id: ep.id.clone(),
                source: ep.source.clone(),
                recorded_at: ep.recorded_at.to_rfc3339(),
                content: ep.content.clone(),
                score: *score,
            })
            .collect();
        let envelope = JsonOutput {
            path: &path_str,
            decisions: &payload,
        };
        let s = serde_json::to_string_pretty(&envelope)
            .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
        println!("{s}");
        return Ok(());
    }

    if results.is_empty() {
        println!("no decisions referencing {path_str}");
        return Ok(());
    }

    println!("{} decision(s) referencing {path_str}:", results.len());
    for (episode, _score) in &results {
        let id_short = &episode.id[..8.min(episode.id.len())];
        let source = episode.source.as_deref().unwrap_or("?");
        let recorded = episode.recorded_at.format("%Y-%m-%d %H:%M");
        let preview = preview(&episode.content, PREVIEW_CHARS);
        println!("  {id_short} [{source}] {recorded}: {preview}");
    }

    Ok(())
}

/// Wrap a free-text query as an FTS5 phrase so punctuation (path separators,
/// dashes, dots) is treated as literal content rather than parsed as FTS5
/// syntax. Identical to the helper in `illuminate-mcp::tools` — kept private
/// here to avoid pulling that crate into a transitive public surface.
fn fts5_phrase_query(input: &str) -> String {
    let escaped = input.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

fn preview(s: &str, max: usize) -> String {
    let one_line = s.replace('\n', " ");
    if one_line.chars().count() <= max {
        one_line
    } else {
        let mut out: String = one_line.chars().take(max).collect();
        out.push('…');
        out
    }
}

#[derive(Serialize)]
struct DecisionRow {
    id: String,
    source: Option<String>,
    recorded_at: String,
    content: String,
    score: f64,
}

#[derive(Serialize)]
struct JsonOutput<'a> {
    path: &'a str,
    decisions: &'a [DecisionRow],
}

#[cfg(test)]
mod tests {
    use super::{fts5_phrase_query, preview};

    #[test]
    fn fts5_phrase_query_quotes_paths() {
        assert_eq!(fts5_phrase_query("src/payments"), "\"src/payments\"");
    }

    #[test]
    fn fts5_phrase_query_escapes_internal_quotes() {
        assert_eq!(fts5_phrase_query("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn preview_truncates_long_strings_with_ellipsis() {
        let long = "x".repeat(200);
        let p = preview(&long, 100);
        assert_eq!(p.chars().count(), 101); // 100 chars + ellipsis
        assert!(p.ends_with('…'));
    }

    #[test]
    fn preview_collapses_newlines() {
        assert_eq!(preview("a\nb\nc", 100), "a b c");
    }
}
