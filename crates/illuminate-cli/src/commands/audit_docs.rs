//! `illuminate audit-docs <FILE>` — agent-aware doc review (knowledge-layer
//! v3.3).
//!
//! Scans a markdown doc's prose paragraphs against the decisions recorded in
//! `.illuminate/graph.db`. For each decision that *carries a rejected concept*
//! (e.g. a `no-redis` decision rejecting Redis), any doc paragraph that
//! AFFIRMATIVELY recommends that concept is flagged as a contradiction.
//!
//! The affirmative-vs-negated classification reuses illuminate-audit's
//! clause-local negation logic ([`illuminate_audit::mentions_as_intent`], the
//! D1 work), so a paragraph saying `do NOT use Redis` is *not* flagged while
//! `use Redis for caching` *is*.
//!
//! Deterministic and network-free: no LLM, no clock-dependent output. Exits 0
//! with a `no decision contradictions found` message when the doc is clean, and
//! exits 1 with a marked `─── illuminate audit-docs ───` report when ≥1
//! contradiction is found (mirrors `doc_decay`'s exit-code contract). `--json`
//! emits `{contradictions:[...],count}`.

use std::env;
use std::path::{Path, PathBuf};

use illuminate::{Episode, Graph};
use illuminate_audit::mentions_as_intent;

use super::open_graph;

/// Upper bound on episodes pulled from the graph. Generous enough to cover a
/// real project's decision record; non-decision episodes are filtered out.
const SCAN_LIMIT: usize = 500;

/// Max characters retained for the paragraph snippet in the report / JSON.
const PARAGRAPH_SNIPPET: usize = 160;

/// A decision that rejects a concept, lifted from the graph.
#[derive(Debug, Clone)]
struct RejectingDecision {
    id: String,
    title: String,
    /// Lowercased concepts this decision rejects (e.g. `"redis"`).
    rejected: Vec<String>,
}

/// A flagged doc paragraph that contradicts a recorded decision.
#[derive(Debug, Clone)]
struct Contradiction {
    file: String,
    /// 1-based line where the contradicting paragraph begins.
    line: usize,
    paragraph: String,
    decision_id: String,
    decision_title: String,
}

/// Run the `audit-docs` subcommand against `file`.
pub fn run(file: PathBuf, json: bool) -> illuminate::Result<()> {
    let text = std::fs::read_to_string(&file).map_err(illuminate::IlluminateError::Io)?;

    let display_path = doc_display_path(&file);

    let graph = open_graph()?;
    let decisions = load_rejecting_decisions(&graph)?;

    let contradictions = scan_doc(&display_path, &text, &decisions);

    if json {
        emit_json(&contradictions);
    } else {
        emit_human(&contradictions, &display_path);
    }

    if contradictions.is_empty() {
        Ok(())
    } else {
        // Nonzero exit signals "contradictions found" to callers / CI; the
        // report itself was already written to stdout (mirrors `doc_decay`).
        std::process::exit(1);
    }
}

/// Render the doc path relative to cwd when possible (so the report reads
/// `design.md:5` rather than an absolute tempdir path), else verbatim.
fn doc_display_path(file: &Path) -> String {
    if let Ok(cwd) = env::current_dir()
        && let Ok(rel) = file.strip_prefix(&cwd)
    {
        return rel.to_string_lossy().to_string();
    }
    file.to_string_lossy().to_string()
}

/// Lift every decision that rejects at least one concept from the graph.
fn load_rejecting_decisions(graph: &Graph) -> illuminate::Result<Vec<RejectingDecision>> {
    let episodes = graph.list_episodes(SCAN_LIMIT, 0)?;
    let mut out: Vec<RejectingDecision> = Vec::new();
    for ep in &episodes {
        if !is_decision(ep) {
            continue;
        }
        let rejected = rejected_concepts(&ep.content);
        if rejected.is_empty() {
            continue;
        }
        out.push(RejectingDecision {
            id: ep.id.clone(),
            title: derive_title(ep),
            rejected,
        });
    }
    // Deterministic order independent of insertion order.
    out.sort_by(|a, b| a.title.cmp(&b.title).then(a.id.cmp(&b.id)));
    Ok(out)
}

/// Whether an episode is a decision (source prefix or `[dec-...]` tag).
fn is_decision(ep: &Episode) -> bool {
    if let Some(src) = ep.source.as_deref()
        && (src.starts_with("wiki:decisions") || src.starts_with("wiki:dec"))
    {
        return true;
    }
    let trimmed = ep.content.trim_start();
    if let Some(rest) = trimmed.strip_prefix('[')
        && let Some(end) = rest.find(']')
    {
        return rest[..end].starts_with("dec-");
    }
    false
}

/// Extract the lowercased concepts a decision rejects.
///
/// Candidates are the proper-noun / technology-like tokens in the decision
/// content. A candidate is *rejected* when the decision does NOT express an
/// intent to use it — i.e. [`mentions_as_intent`] returns `false` for the
/// decision's own prose (the decision says "do not use Redis", so Redis is the
/// rejected concept). This is the exact same clause-local negation logic the
/// doc paragraphs are then checked against, so the two sides agree.
fn rejected_concepts(content: &str) -> Vec<String> {
    // Strip the leading `[dec-...]` id tag and any YAML front matter so the
    // analysis runs over the decision's prose only — the id slug (e.g.
    // `dec-no-redis`) embeds the concept name without a clause-local negator,
    // which would otherwise read as an affirmative use of the concept.
    let body = strip_tag_and_front_matter(content);
    let content_lower = body.to_lowercase();
    let mut seen: Vec<String> = Vec::new();
    for cand in candidate_concepts(body) {
        let cand_lower = cand.to_lowercase();
        // Must actually appear in the content, and the decision must NOT intend
        // to USE it (a negated/rejected mention) for it to be a rejected concept.
        if !content_lower.contains(&cand_lower) {
            continue;
        }
        if mentions_as_intent(&content_lower, &cand_lower) {
            continue;
        }
        if !seen.contains(&cand_lower) {
            seen.push(cand_lower);
        }
    }
    seen.sort();
    seen
}

/// Return decision content with a leading `[dec-...]` id tag and/or a leading
/// `---\n...\n---` YAML front-matter block removed, leaving prose only.
fn strip_tag_and_front_matter(content: &str) -> &str {
    let mut s = content.trim_start();
    // Drop a leading `[...]` id tag on the first line.
    if let Some(rest) = s.strip_prefix('[')
        && let Some(end) = rest.find(']')
    {
        s = rest[end + 1..].trim_start();
    }
    // Drop a leading YAML front-matter block.
    if let Some(rest) = s.strip_prefix("---\n")
        && let Some(end) = rest.find("\n---")
    {
        s = rest[end + 4..].trim_start();
    }
    s
}

/// Candidate concept tokens from a decision's content: capitalized words that
/// look like proper nouns / technology names (e.g. `Redis`, `Kafka`,
/// `MongoDB`). Lowercase prose words, the `[dec-...]` tag, and markdown
/// punctuation never produce candidates.
fn candidate_concepts(content: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in content.split(|c: char| !(c.is_alphanumeric() || c == '-')) {
        let tok = raw.trim_matches('-');
        if !looks_like_concept(tok) {
            continue;
        }
        if !out.iter().any(|e| e == tok) {
            out.push(tok.to_string());
        }
    }
    out
}

/// A concept token is an alphanumeric word ≥3 chars whose FIRST char is an
/// uppercase letter (proper noun / product name). This keeps `Redis`, `Kafka`,
/// `MongoDB` while rejecting `the`, `use`, `cache`, sentence-initial common
/// words are still admitted but harmlessly filtered later by the
/// negation/use-intent check unless they sit in a rejection clause.
fn looks_like_concept(tok: &str) -> bool {
    if tok.chars().count() < 3 {
        return false;
    }
    let mut chars = tok.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_uppercase() {
        return false;
    }
    tok.chars().all(|c| c.is_alphanumeric())
}

/// Scan a doc's prose paragraphs for affirmative recommendations of any
/// rejected concept. Returns one [`Contradiction`] per (paragraph, decision)
/// pair, deterministically ordered by line then decision id.
fn scan_doc(file: &str, text: &str, decisions: &[RejectingDecision]) -> Vec<Contradiction> {
    let mut out: Vec<Contradiction> = Vec::new();
    if decisions.is_empty() {
        return out;
    }
    for para in split_paragraphs(text) {
        if is_code_or_heading(&para.text) {
            continue;
        }
        let para_lower = para.text.to_lowercase();
        for dec in decisions {
            for concept in &dec.rejected {
                if !para_lower.contains(concept.as_str()) {
                    continue;
                }
                if mentions_as_intent(&para_lower, concept) {
                    out.push(Contradiction {
                        file: file.to_string(),
                        line: para.line,
                        paragraph: snippet(&para.text),
                        decision_id: dec.id.clone(),
                        decision_title: dec.title.clone(),
                    });
                    // One flag per (paragraph, decision) — don't double-report
                    // the same decision for multiple rejected concepts.
                    break;
                }
            }
        }
    }
    out.sort_by(|a, b| a.line.cmp(&b.line).then(a.decision_id.cmp(&b.decision_id)));
    out
}

/// A prose paragraph plus the 1-based line where it begins.
struct Paragraph {
    line: usize,
    text: String,
}

/// Split markdown into blank-line-separated paragraphs, tracking each one's
/// starting line (1-based).
fn split_paragraphs(text: &str) -> Vec<Paragraph> {
    let mut out: Vec<Paragraph> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut start_line = 0usize;
    for (idx, line) in text.lines().enumerate() {
        let line_no = idx + 1;
        if line.trim().is_empty() {
            if !current.is_empty() {
                out.push(Paragraph {
                    line: start_line,
                    text: current.join("\n"),
                });
                current.clear();
            }
        } else {
            if current.is_empty() {
                start_line = line_no;
            }
            current.push(line.to_string());
        }
    }
    if !current.is_empty() {
        out.push(Paragraph {
            line: start_line,
            text: current.join("\n"),
        });
    }
    out
}

/// Whether a paragraph is a markdown heading or a fenced code block (which
/// aren't prose claims and shouldn't be audited for recommendations).
fn is_code_or_heading(para: &str) -> bool {
    let trimmed = para.trim_start();
    trimmed.starts_with('#') || trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

fn snippet(para: &str) -> String {
    let one_line: String = para
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if one_line.chars().count() <= PARAGRAPH_SNIPPET {
        return one_line;
    }
    let truncated: String = one_line.chars().take(PARAGRAPH_SNIPPET).collect();
    format!("{truncated}…")
}

/// Derive a human title for a decision episode. Prefers `metadata.title`, then
/// the first non-empty content line with any leading `[id]` tag stripped.
fn derive_title(ep: &Episode) -> String {
    if let Some(meta) = &ep.metadata
        && let Some(t) = meta.get("title").and_then(|v| v.as_str())
        && !t.is_empty()
    {
        return t.to_string();
    }
    for line in ep.content.lines() {
        let l = line.trim();
        if l.is_empty() || l == "---" {
            continue;
        }
        let cleaned = if let Some(rest) = l.strip_prefix('[')
            && let Some(end) = rest.find(']')
        {
            rest[end + 1..].trim().to_string()
        } else {
            l.to_string()
        };
        if !cleaned.is_empty() {
            return cleaned;
        }
    }
    "(untitled decision)".to_string()
}

fn emit_human(contradictions: &[Contradiction], file: &str) {
    if contradictions.is_empty() {
        println!("no decision contradictions found (scanned {file}).");
        return;
    }
    println!("─── illuminate audit-docs ───");
    println!(
        "  {} doc paragraph(s) contradict a recorded decision:",
        contradictions.len()
    );
    for c in contradictions {
        println!(
            "  CONTRADICTS {}:{} → decision \"{}\"",
            c.file, c.line, c.decision_title
        );
        println!("    {}", c.paragraph);
    }
}

fn emit_json(contradictions: &[Contradiction]) {
    let arr: Vec<serde_json::Value> = contradictions
        .iter()
        .map(|c| {
            serde_json::json!({
                "file": c.file,
                "line": c.line,
                "paragraph": c.paragraph,
                "decision_id": c.decision_id,
                "decision_title": c.decision_title,
            })
        })
        .collect();
    let payload = serde_json::json!({
        "contradictions": arr,
        "count": contradictions.len(),
    });
    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;
    use illuminate::Episode;

    fn ep(content: &str, source: Option<&str>) -> Episode {
        let mut b = Episode::builder(content);
        if let Some(s) = source {
            b = b.source(s);
        }
        b.build()
    }

    #[test]
    fn split_paragraphs_tracks_line_numbers() {
        let text = "# Title\n\nfirst para line.\n\nsecond para\nspans two lines.\n";
        let paras = split_paragraphs(text);
        assert_eq!(paras.len(), 3);
        assert_eq!(paras[0].line, 1);
        assert_eq!(paras[0].text, "# Title");
        assert_eq!(paras[1].line, 3);
        assert_eq!(paras[2].line, 5);
        assert_eq!(paras[2].text, "second para\nspans two lines.");
    }

    #[test]
    fn split_paragraphs_handles_no_trailing_newline() {
        let paras = split_paragraphs("only one para");
        assert_eq!(paras.len(), 1);
        assert_eq!(paras[0].line, 1);
    }

    #[test]
    fn rejected_concepts_lifts_negated_concept() {
        let rejected =
            rejected_concepts("[dec-no-redis] No Redis sidecar\n\nWe do not use Redis here.");
        assert!(
            rejected.contains(&"redis".to_string()),
            "Redis must be a rejected concept: {rejected:?}"
        );
    }

    #[test]
    fn rejected_concepts_ignores_affirmatively_used_concept() {
        // A decision that AFFIRMS Postgres does not reject it.
        let rejected =
            rejected_concepts("[dec-use-pg] Use Postgres\n\nWe use Postgres for storage.");
        assert!(
            !rejected.contains(&"postgres".to_string()),
            "an affirmed concept must NOT be rejected: {rejected:?}"
        );
    }

    #[test]
    fn affirmative_paragraph_is_flagged_negated_is_not() {
        let decisions = vec![RejectingDecision {
            id: "dec-no-redis".into(),
            title: "No Redis sidecar".into(),
            rejected: vec!["redis".into()],
        }];

        // Affirmative recommendation → flagged.
        let flagged = scan_doc("d.md", "We use Redis for the cache.", &decisions);
        assert_eq!(flagged.len(), 1, "affirmative use must be flagged");
        assert_eq!(flagged[0].decision_title, "No Redis sidecar");
        assert_eq!(flagged[0].line, 1);

        // Negated mention → not flagged.
        let clean = scan_doc("d.md", "We do not use Redis.", &decisions);
        assert!(clean.is_empty(), "negated mention must not be flagged");
    }

    #[test]
    fn empty_decisions_graph_is_always_clean() {
        let flagged = scan_doc("d.md", "We use Redis everywhere.", &[]);
        assert!(
            flagged.is_empty(),
            "no decisions ⇒ no contradictions regardless of doc content"
        );
    }

    #[test]
    fn is_decision_recognizes_source_and_tag() {
        assert!(is_decision(&ep("body", Some("wiki:dec/no-redis"))));
        assert!(is_decision(&ep("[dec-no-redis] No Redis", None)));
        assert!(!is_decision(&ep("[pat-lru] LRU", Some("wiki:pat/lru"))));
        assert!(!is_decision(&ep("just prose", None)));
    }

    #[test]
    fn headings_and_code_are_skipped() {
        let decisions = vec![RejectingDecision {
            id: "dec-no-redis".into(),
            title: "No Redis sidecar".into(),
            rejected: vec!["redis".into()],
        }];
        // A heading that names the concept is not a prose recommendation.
        let heading = scan_doc("d.md", "# Use Redis", &decisions);
        assert!(heading.is_empty(), "headings must not be flagged");
        let code = scan_doc("d.md", "```\nuse Redis\n```", &decisions);
        assert!(code.is_empty(), "fenced code must not be flagged");
    }

    #[test]
    fn derive_title_strips_tag() {
        assert_eq!(
            derive_title(&ep("[dec-no-redis] No Redis sidecar\n\nbody", None)),
            "No Redis sidecar"
        );
    }
}
