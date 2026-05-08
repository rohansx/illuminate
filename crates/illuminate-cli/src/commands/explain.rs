//! `illuminate explain <PATH>` — surface every decision, pattern, and
//! failure linked to a file, grouped by source heuristic.
//!
//! Mirrors the wiring of the MCP `illuminate_explain` tool
//! (`crates/illuminate-mcp/src/tools.rs`), but adds source-based grouping
//! so the human view distinguishes decisions from patterns from failures.
//!
//! The grouping is heuristic-only — it inspects `episode.source` strings
//! like `wiki:dec/...`, `wiki:pat/...`, `wiki:fail/...`, or content with
//! `reflexion`/`failure`/`pattern` substrings. Anything that doesn't
//! match a known prefix lands in `other` so we never silently drop data.

use std::path::PathBuf;

use serde_json::{Value, json};

use super::open_graph;

/// Length cap for the human-readable content preview line. Long episodes
/// would otherwise wrap and confuse the bullet list under each section.
const PREVIEW_CHARS: usize = 80;

/// Run the `explain` subcommand.
///
/// Reads anchors for `<path>` via `Graph::get_anchors_for_file`, fetches
/// each linked episode, and groups by source. Renders human (default) or
/// JSON output. Always exits 0 — no anchors is a normal result, not an
/// error (the file simply isn't yet linked to any decision/pattern/failure).
pub fn run(path: PathBuf, json_output: bool) -> illuminate::Result<()> {
    let graph = open_graph()?;
    let path_str = path.to_string_lossy().to_string();

    let anchors = graph.get_anchors_for_file(&path_str)?;

    let mut decisions: Vec<Value> = Vec::new();
    let mut patterns: Vec<Value> = Vec::new();
    let mut failures: Vec<Value> = Vec::new();
    let mut other: Vec<Value> = Vec::new();

    for anchor in &anchors {
        let Some(episode) = graph.get_episode(&anchor.episode_id)? else {
            continue;
        };
        let source = episode.source.as_deref().unwrap_or("");
        let entry = json!({
            "id": episode.id,
            "content": episode.content,
            "source": episode.source,
            "recorded_at": episode.recorded_at.to_rfc3339(),
            "anchor": {
                "symbol": anchor.symbol_name,
                "lines": format!(
                    "{}-{}",
                    anchor.line_start.unwrap_or(0),
                    anchor.line_end.unwrap_or(0)
                ),
            }
        });

        match classify(source) {
            Category::Decision => decisions.push(entry),
            Category::Pattern => patterns.push(entry),
            Category::Failure => failures.push(entry),
            Category::Other => other.push(entry),
        }
    }

    if json_output {
        let total = decisions.len() + patterns.len() + failures.len() + other.len();
        let envelope = json!({
            "path": path_str,
            "decisions": decisions,
            "patterns": patterns,
            "failures": failures,
            "other": other,
            "total": total,
        });
        let s = serde_json::to_string_pretty(&envelope)
            .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
        println!("{s}");
    } else {
        print_human(&path_str, &decisions, &patterns, &failures, &other);
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Decision,
    Pattern,
    Failure,
    Other,
}

/// Classify an episode source string into one of the four buckets.
///
/// Order matters: `failure` is checked before `pattern`/`decision` because
/// a free-form reflexion source could in theory contain multiple keywords.
/// Wiki-prefixed sources (`wiki:dec/`, `wiki:pat/`, `wiki:fail/`) are the
/// canonical case and take priority.
fn classify(source: &str) -> Category {
    let s = source.to_lowercase();
    if s.starts_with("wiki:dec/") || s.contains("dec-") || s.contains("decision") {
        return Category::Decision;
    }
    if s.starts_with("wiki:pat/") || s.contains("pat-") || s.contains("pattern") {
        return Category::Pattern;
    }
    if s.starts_with("wiki:fail/")
        || s.contains("fail-")
        || s.contains("failure")
        || s.contains("reflexion")
    {
        return Category::Failure;
    }
    Category::Other
}

fn print_human(
    path: &str,
    decisions: &[Value],
    patterns: &[Value],
    failures: &[Value],
    other: &[Value],
) {
    println!("=== {path} ===\n");
    print_section("Decisions", decisions);
    print_section("Patterns", patterns);
    print_section("Failures", failures);
    print_section("Other", other);

    let total = decisions.len() + patterns.len() + failures.len() + other.len();
    if total == 0 {
        println!(
            "(no anchors found for this path — this file isn't yet linked to any decision/pattern/failure)"
        );
    }
}

fn print_section(label: &str, entries: &[Value]) {
    if entries.is_empty() {
        return;
    }
    println!("{label} ({}):", entries.len());
    for e in entries {
        let id = e["id"].as_str().unwrap_or("?");
        let source = e["source"].as_str().unwrap_or("?");
        let date = e["recorded_at"].as_str().unwrap_or("?");
        let content = e["content"].as_str().unwrap_or("");
        let preview = preview_line(content, PREVIEW_CHARS);
        let symbol = e["anchor"]["symbol"].as_str().unwrap_or("");
        let lines = e["anchor"]["lines"].as_str().unwrap_or("?-?");
        println!("  - [{id}] ({source} @ {date}): {preview}");
        if symbol.is_empty() {
            println!("    └── line {lines}");
        } else {
            println!("    └── line {lines} ({symbol})");
        }
    }
    println!();
}

fn preview_line(s: &str, max: usize) -> String {
    let one_line = s.replace('\n', " ");
    if one_line.chars().count() <= max {
        one_line
    } else {
        let mut out: String = one_line.chars().take(max).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{Category, classify, preview_line};

    #[test]
    fn classify_wiki_decision_prefix() {
        assert_eq!(classify("wiki:dec/no-redis"), Category::Decision);
    }

    #[test]
    fn classify_wiki_pattern_prefix() {
        assert_eq!(classify("wiki:pat/lru-cache"), Category::Pattern);
    }

    #[test]
    fn classify_wiki_failure_prefix() {
        assert_eq!(classify("wiki:fail/cache-stampede"), Category::Failure);
    }

    #[test]
    fn classify_reflexion_lands_in_failures() {
        assert_eq!(classify("reflexion:agent-loop-7"), Category::Failure);
    }

    #[test]
    fn classify_unknown_source_lands_in_other() {
        assert_eq!(classify("git-commit:abc123"), Category::Other);
        assert_eq!(classify(""), Category::Other);
    }

    #[test]
    fn preview_line_truncates_long_strings() {
        let long = "x".repeat(200);
        let p = preview_line(&long, 80);
        assert_eq!(p.chars().count(), 81);
        assert!(p.ends_with('…'));
    }
}
