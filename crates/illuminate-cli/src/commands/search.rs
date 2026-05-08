//! `illuminate search` — top-level search over the graph.
//!
//! Documented at `docs/CLI.md` § "Search commands". Combines SQLite FTS5 with
//! embedding similarity via [`illuminate::Graph::search_fused`] when an embed
//! engine is available, falling back to FTS5-only via
//! [`illuminate::Graph::search`] when not.
//!
//! Result filtering by `--type` is applied post-search by inspecting each
//! episode's `source` field — see [`type_matches`] for the matching rules.

use std::env;
use std::path::PathBuf;

use illuminate::Episode;
use serde::Serialize;

/// Run `illuminate search`. See module docs for behavior.
pub fn run(
    query: String,
    limit: usize,
    type_filter: Option<String>,
    format: String,
) -> illuminate::Result<()> {
    let db_path = find_db()?;
    let graph = illuminate::Graph::open(&db_path)?;

    // FTS5 parses bare punctuation (dashes, dots, slashes) as syntax. Wrap
    // the user's query as a phrase so it matches as literal text. The same
    // helper backs `illuminate decisions for` — see that module for context.
    let fts_query = fts5_phrase_query(&query);

    let episodes = if let Some(engine) = super::audit::try_load_embed_pub() {
        match engine.embed(&query) {
            Ok(embedding) => {
                let fused = graph.search_fused(&fts_query, &embedding, limit * 4)?;
                fused.into_iter().map(|f| f.episode).collect::<Vec<_>>()
            }
            Err(_) => fts_fallback(&graph, &fts_query, limit * 4)?,
        }
    } else {
        fts_fallback(&graph, &fts_query, limit * 4)?
    };

    let filtered: Vec<Episode> = episodes
        .into_iter()
        .filter(|ep| type_matches(ep, type_filter.as_deref()))
        .take(limit)
        .collect();

    match format.as_str() {
        "json" => print_json(&filtered)?,
        _ => print_text(&filtered),
    }

    Ok(())
}

fn fts_fallback(
    graph: &illuminate::Graph,
    query: &str,
    limit: usize,
) -> illuminate::Result<Vec<Episode>> {
    let results = graph.search(query, limit)?;
    Ok(results.into_iter().map(|(ep, _)| ep).collect())
}

/// Wrap a free-text query as an FTS5 phrase so punctuation (path separators,
/// dashes, dots) is treated as literal content rather than parsed as FTS5
/// syntax. Mirrors `decisions::fts5_phrase_query`.
fn fts5_phrase_query(input: &str) -> String {
    let escaped = input.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

fn type_matches(episode: &Episode, filter: Option<&str>) -> bool {
    let Some(kind) = filter else {
        return true;
    };
    let source = episode.source.as_deref().unwrap_or("").to_lowercase();
    match kind {
        "entity" => true,
        "decision" => {
            source.contains("decision") || source.contains("wiki:dec/") || source.contains("dec-")
        }
        "pattern" => {
            source.contains("pattern") || source.contains("wiki:pat/") || source.contains("pat-")
        }
        "failure" => {
            source.contains("failure") || source.contains("fail-") || source.contains("reflexion")
        }
        // Defensive default — clap's value_parser already guards this.
        _ => true,
    }
}

fn print_text(episodes: &[Episode]) {
    if episodes.is_empty() {
        println!("(no matches — 0 results)");
        return;
    }
    println!("== search ({} results) ==", episodes.len());
    for ep in episodes {
        let snippet: String = ep
            .content
            .chars()
            .take(160)
            .collect::<String>()
            .replace('\n', " ");
        println!(
            "  [{}] {} — {}",
            ep.id,
            ep.source.as_deref().unwrap_or("?"),
            snippet
        );
    }
}

fn print_json(episodes: &[Episode]) -> illuminate::Result<()> {
    #[derive(Serialize)]
    struct Row<'a> {
        id: &'a str,
        source: Option<&'a str>,
        content: &'a str,
    }
    let rows: Vec<Row<'_>> = episodes
        .iter()
        .map(|ep| Row {
            id: &ep.id,
            source: ep.source.as_deref(),
            content: &ep.content,
        })
        .collect();
    let json = serde_json::to_string_pretty(&rows)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
    println!("{json}");
    Ok(())
}

/// Find the nearest `.illuminate/graph.db`, walking upward from cwd. If none
/// exists, fall back to `<repo-root>/.illuminate/graph.db` so a fresh
/// `illuminate.toml` repo with no graph still produces an empty-but-valid DB
/// rather than a confusing "no .illuminate/" error for `search`.
fn find_db() -> illuminate::Result<PathBuf> {
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        let candidate = d.join(".illuminate").join("graph.db");
        if candidate.exists() {
            return Ok(candidate);
        }
        let toml = d.join(".illuminate").join("illuminate.toml");
        if toml.is_file() {
            // No graph.db yet — create an empty one so search is callable.
            let path = d.join(".illuminate").join("graph.db");
            illuminate::Graph::open_or_create(&path)?;
            return Ok(path);
        }
        cur = d.parent();
    }
    Err(illuminate::IlluminateError::NotFound(
        "no .illuminate/ found in current or parent directories. Run `illuminate init` first."
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::type_matches;
    use chrono::Utc;
    use illuminate::Episode;

    fn ep(source: &str) -> Episode {
        Episode {
            id: "ep1".into(),
            content: "x".into(),
            source: Some(source.to_string()),
            recorded_at: Utc::now(),
            metadata: None,
        }
    }

    #[test]
    fn type_filter_none_matches_all() {
        assert!(type_matches(&ep("anything"), None));
    }

    #[test]
    fn type_filter_decision_matches_dash_prefix() {
        assert!(type_matches(&ep("wiki:dec-foo"), Some("decision")));
        assert!(type_matches(&ep("decisions:adr-1"), Some("decision")));
    }

    #[test]
    fn type_filter_failure_matches_reflexion() {
        assert!(type_matches(&ep("reflexion:agent-x"), Some("failure")));
        assert!(type_matches(&ep("failure:fail-cache"), Some("failure")));
    }

    #[test]
    fn type_filter_pattern_excludes_decision() {
        assert!(!type_matches(&ep("wiki:dec-foo"), Some("pattern")));
        assert!(type_matches(&ep("wiki:pat/foo"), Some("pattern")));
    }
}
